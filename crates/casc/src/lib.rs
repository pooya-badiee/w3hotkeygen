//! Reads files out of a Warcraft III (Reforged) CASC storage, in pure Rust so
//! it also runs in the browser. [`Casc`] works over any [`Source`]; on native,
//! [`Storage`] wraps it over the file system with a blocking API.

mod blte;
mod index;
mod storage;
mod tvfs;

use std::fmt;
use std::future::Future;
use std::io;

pub use storage::Casc;

/// The first 9 bytes of an encoded key: all that indexes and TVFS store.
type EKey = [u8; 9];

/// The install folder's files. Paths are relative to the install root and use
/// `/`, e.g. `Data/data/data.003`.
pub trait Source {
    /// `len` bytes from `offset`, or the rest of the file if `len` is `None`.
    fn read(&self, path: &str, offset: u64, len: Option<u64>) -> impl Future<Output = io::Result<Vec<u8>>>;
    /// File names in a folder.
    fn list(&self, dir: &str) -> impl Future<Output = io::Result<Vec<String>>>;
}

#[derive(Debug, Clone)]
pub enum Error {
    Io(String),
    NotFound,
    Format(&'static str),
    Unsupported(&'static str),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io(e) => write!(f, "{e}"),
            Error::NotFound => write!(f, "file not found"),
            Error::Format(what) => write!(f, "{what}"),
            Error::Unsupported(what) => write!(f, "unsupported: {what}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::Io(e.to_string())
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub use native::{FsSource, Storage};

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use std::fs::{self, File};
    use std::future::Future;
    use std::io::{self, Read, Seek, SeekFrom};
    use std::path::{Path, PathBuf};
    use std::pin::pin;
    use std::task::{Context, Poll, Waker};

    use crate::{Casc, Error, Source};

    /// The install folder on disk.
    pub struct FsSource(PathBuf);

    impl Source for FsSource {
        async fn read(&self, path: &str, offset: u64, len: Option<u64>) -> io::Result<Vec<u8>> {
            let mut file = File::open(self.0.join(path))?;
            file.seek(SeekFrom::Start(offset))?;
            let mut out = Vec::new();
            match len {
                Some(len) => {
                    out.resize(len as usize, 0);
                    file.read_exact(&mut out)?;
                }
                None => {
                    file.read_to_end(&mut out)?;
                }
            }
            Ok(out)
        }

        async fn list(&self, dir: &str) -> io::Result<Vec<String>> {
            fs::read_dir(self.0.join(dir))?
                .map(|e| e.map(|e| e.file_name().to_string_lossy().into_owned()))
                .collect()
        }
    }

    /// An open storage on disk: the game's install folder.
    pub struct Storage(Casc<FsSource>);

    impl Storage {
        pub fn open(path: &Path) -> Result<Storage, Error> {
            block_on(Casc::open(FsSource(path.to_path_buf()))).map(Storage)
        }

        /// A whole file, by name, e.g. `war3.w3mod:units\humanunitfunc.txt`.
        pub fn read(&self, name: &str) -> Result<Vec<u8>, Error> {
            block_on(self.0.read(name))
        }

        /// Names of the files under `prefix`; see [`Casc::list`].
        pub fn list(&self, prefix: &str) -> Result<Vec<String>, Error> {
            block_on(self.0.list(prefix))
        }
    }

    /// [`FsSource`] never waits, so its futures finish on the first poll.
    fn block_on<F: Future>(future: F) -> F::Output {
        match pin!(future).poll(&mut Context::from_waker(Waker::noop())) {
            Poll::Ready(out) => out,
            Poll::Pending => unreachable!("file system reads finish on the first poll"),
        }
    }
}
