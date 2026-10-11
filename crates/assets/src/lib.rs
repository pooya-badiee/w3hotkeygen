//! Access to the user's local Warcraft III installation.
//!
//! We never ship Blizzard assets; everything is read from the user's own install.

pub mod game_data;
pub mod game_files;
pub mod locate;

pub use w3hk_core::load::LoadError;
pub use game_files::{GameFiles, OpenError};
pub use locate::{Found, Install, LocateError, Source, custom_keys_path, deep_scan, find, find_all, validate};
