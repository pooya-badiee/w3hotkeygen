//! Reading files out of the user's game archives.

use std::fmt;

use crate::Install;

/// Open game storage. Paths use CASC syntax, e.g.
/// `war3.w3mod:webui\baseframes\test_texture-wood.png`.
pub struct GameFiles {
    storage: w3hk_casc::Storage,
}

#[derive(Debug)]
pub struct OpenError(w3hk_casc::Error);

impl fmt::Display for OpenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Couldn't open game files: {}", self.0)
    }
}

impl std::error::Error for OpenError {}

impl GameFiles {
    pub fn open(install: &Install) -> Result<Self, OpenError> {
        let storage = w3hk_casc::Storage::open(&install.root).map_err(OpenError)?;
        Ok(GameFiles { storage })
    }

    /// Whole file contents, or `None` if it doesn't exist.
    pub fn read(&self, path: &str) -> Option<Vec<u8>> {
        self.storage.read(path).ok()
    }
}
