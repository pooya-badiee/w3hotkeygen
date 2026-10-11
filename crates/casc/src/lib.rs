//! Minimal bindings to CascLib (vendor/CascLib, MIT): open a game's CASC
//! storage and read a file by name. Only the calls this app needs are
//! declared, by hand, from `CascLib.h`.

use std::ffi::{CString, c_void};
use std::fmt;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use std::ptr;

mod ffi {
    use std::ffi::c_void;

    pub type Handle = *mut c_void;

    /// `CASC_OPEN_BY_NAME`.
    pub const OPEN_BY_NAME: u32 = 0;

    // `WINAPI` is stdcall on 32-bit Windows; "system" matches it everywhere.
    unsafe extern "system" {
        /// `params` is a wide string: CascLib is built with `CASC_UNICODE`.
        pub fn CascOpenStorage(params: *const u16, locale_mask: u32, storage: *mut Handle) -> bool;
        pub fn CascCloseStorage(storage: Handle) -> bool;
        pub fn CascOpenFile(storage: Handle, name: *const c_void, locale: u32, flags: u32, file: *mut Handle) -> bool;
        pub fn CascGetFileSize64(file: Handle, size: *mut u64) -> bool;
        pub fn CascReadFile(file: Handle, buffer: *mut c_void, to_read: u32, read: *mut u32) -> bool;
        pub fn CascCloseFile(file: Handle) -> bool;
    }

    unsafe extern "C" {
        pub fn GetCascError() -> u32;
    }
}

/// A failed CascLib call, with its error code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    pub call: &'static str,
    pub code: u32,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} failed (error {})", self.call, self.code)
    }
}

impl std::error::Error for Error {}

fn last_error(call: &'static str) -> Error {
    Error { call, code: unsafe { ffi::GetCascError() } }
}

/// An open CASC storage: the game's install folder.
pub struct Storage(ffi::Handle);

impl Storage {
    pub fn open(path: &Path) -> Result<Storage, Error> {
        let wide: Vec<u16> = path.as_os_str().encode_wide().collect();
        if wide.contains(&0) {
            return Err(Error { call: "CascOpenStorage", code: 0 });
        }
        let params: Vec<u16> = wide.into_iter().chain([0]).collect();
        let mut handle = ptr::null_mut();
        if unsafe { ffi::CascOpenStorage(params.as_ptr(), 0, &mut handle) } {
            Ok(Storage(handle))
        } else {
            Err(last_error("CascOpenStorage"))
        }
    }

    /// A whole file, by name, e.g. `war3.w3mod:units\humanunitfunc.txt`.
    pub fn read(&self, name: &str) -> Result<Vec<u8>, Error> {
        let name = CString::new(name).map_err(|_| Error { call: "CascOpenFile", code: 0 })?;
        let mut file = ptr::null_mut();
        if !unsafe { ffi::CascOpenFile(self.0, name.as_ptr().cast(), 0, ffi::OPEN_BY_NAME, &mut file) } {
            return Err(last_error("CascOpenFile"));
        }
        let file = File(file);
        let mut size = 0u64;
        if !unsafe { ffi::CascGetFileSize64(file.0, &mut size) } {
            return Err(last_error("CascGetFileSize64"));
        }
        let mut out = vec![0u8; size as usize];
        let mut done = 0;
        while done < out.len() {
            let want = (out.len() - done).min(u32::MAX as usize) as u32;
            let mut got = 0u32;
            let buf: *mut c_void = out[done..].as_mut_ptr().cast();
            if !unsafe { ffi::CascReadFile(file.0, buf, want, &mut got) } {
                return Err(last_error("CascReadFile"));
            }
            if got == 0 {
                break;
            }
            done += got as usize;
        }
        out.truncate(done);
        Ok(out)
    }
}

impl Drop for Storage {
    fn drop(&mut self) {
        unsafe { ffi::CascCloseStorage(self.0) };
    }
}

/// Closes an open file on every return path.
struct File(ffi::Handle);

impl Drop for File {
    fn drop(&mut self) {
        unsafe { ffi::CascCloseFile(self.0) };
    }
}
