//! Reads unit and ability data out of the game files. Which files, and the
//! parsing, are in `w3hk_core`; this only reads them.

use w3hk_core::data::GameData;
use w3hk_core::load::LoadError;

use crate::GameFiles;

/// Units, abilities and their default positions; see [`w3hk_core::load::load`].
pub fn load(files: &GameFiles, locale: &str) -> Result<GameData, LoadError> {
    w3hk_core::load::load(|path| files.read(path).map(|b| String::from_utf8_lossy(&b).into_owned()), locale)
}
