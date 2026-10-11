//! Finding and validating a Warcraft III installation.
//!
//! [`find_all`] checks every cheap source we know of (env override, registry,
//! Battle.net databases, default folders). [`deep_scan`] is a slower,
//! bounded walk of the drives for when that fails.

mod extract;
mod folders;
mod scan;
mod sources;
mod user_files;

use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

pub use scan::deep_scan;
pub use user_files::custom_keys_path;

/// Env var that overrides detection (checked first).
pub const ENV_OVERRIDE: &str = "W3HK_WC3_DIR";

/// A Reforged / 1.30+ install: CASC storage with `.build.info` + `Data/`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Install {
    pub root: PathBuf,
    /// Game version, when it can be read cheaply (from `.build.info`).
    pub version: Option<String>,
}

/// Where a detected install came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    EnvVar,
    Registry,
    BattleNet,
    DefaultFolder,
    DeepScan,
}

impl fmt::Display for Source {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Source::EnvVar => "W3HK_WC3_DIR",
            Source::Registry => "Windows registry",
            Source::BattleNet => "Battle.net",
            Source::DefaultFolder => "default folder",
            Source::DeepScan => "drive scan",
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Found {
    pub install: Install,
    pub source: Source,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LocateError {
    Missing(PathBuf),
    NotWarcraft(PathBuf),
}

impl fmt::Display for LocateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LocateError::Missing(p) => write!(f, "Folder does not exist: {}", p.display()),
            LocateError::NotWarcraft(p) => {
                write!(f, "No Warcraft III 1.30+ install found in {} (expected .build.info + Data)", p.display())
            }
        }
    }
}

impl std::error::Error for LocateError {}

/// Checks `path` for a WC3 install. Accepts a file inside the install (e.g. the
/// launcher exe) or a subfolder (e.g. `_retail_\x86_64`), walking up at most
/// two levels.
pub fn validate(path: &Path) -> Result<Install, LocateError> {
    let dir = if path.is_file() { path.parent().unwrap_or(path) } else { path };
    if !dir.is_dir() {
        return Err(LocateError::Missing(path.to_path_buf()));
    }
    dir.ancestors()
        .take(3)
        .find_map(detect)
        .ok_or_else(|| LocateError::NotWarcraft(path.to_path_buf()))
}

/// Checks a single folder without walking up.
fn detect(dir: &Path) -> Option<Install> {
    let build_info = dir.join(".build.info");
    if !(build_info.is_file() && dir.join("Data").is_dir()) {
        return None;
    }
    let text = fs::read_to_string(&build_info).ok()?;
    // Other CASC games share this layout; WC3's product code is "w3".
    if !is_w3_build_info(&text) {
        return None;
    }
    let version = build_info_field(&text, "Version").map(str::to_string);
    Some(Install { root: dir.to_path_buf(), version })
}

/// `.build.info` is a `|`-separated table: a header of `Name!TYPE:size`
/// columns followed by one row per branch.
fn build_info_field<'a>(text: &'a str, name: &str) -> Option<&'a str> {
    let mut lines = text.lines().filter(|l| !l.trim().is_empty());
    let col = lines.next()?.split('|').position(|h| h.split('!').next() == Some(name))?;
    let value = lines.next()?.split('|').nth(col)?.trim();
    (!value.is_empty()).then_some(value)
}

/// Old `.build.info` files have no Product column; accept those.
fn is_w3_build_info(text: &str) -> bool {
    build_info_field(text, "Product").is_none_or(|p| p.eq_ignore_ascii_case("w3"))
}

/// Retries with up to two trailing characters dropped, since paths scraped from
/// binary files can carry a stray byte at the end.
fn validate_scraped(path: &Path) -> Option<Install> {
    let s = path.to_string_lossy();
    let mut end = s.len();
    for _ in 0..3 {
        if let Ok(install) = validate(Path::new(&s[..end])) {
            return Some(install);
        }
        end = s[..end].char_indices().last()?.0;
    }
    None
}

/// Every distinct install found through the cheap sources, best first.
pub fn find_all() -> Vec<Found> {
    let mut found: Vec<Found> = Vec::new();
    let mut seen: Vec<PathBuf> = Vec::new();
    for (path, source) in sources::candidates() {
        let Some(install) = validate_scraped(&path) else { continue };
        let key = fs::canonicalize(&install.root).unwrap_or_else(|_| install.root.clone());
        if !seen.contains(&key) {
            seen.push(key);
            found.push(Found { install, source });
        }
    }
    found
}

/// The best install from the cheap sources.
pub fn find() -> Option<Found> {
    find_all().into_iter().next()
}

#[cfg(test)]
mod tests {
    use super::*;

    const BUILD_INFO: &str = "Branch!STRING:0|Active!DEC:1|Build Key!HEX:16|Version!STRING:0|Product!STRING:0\n\
                              us|1|3a9d8f26806936764d2d9ad526a65e04|3.0.0.24268|w3\n";

    pub(super) fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("w3hk-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn fake_casc(root: &Path) {
        fs::create_dir_all(root.join("Data")).unwrap();
        fs::write(root.join(".build.info"), BUILD_INFO).unwrap();
    }

    #[test]
    fn reads_version_from_build_info() {
        assert_eq!(build_info_field(BUILD_INFO, "Version"), Some("3.0.0.24268"));
        assert_eq!(build_info_field("Branch!STRING:0\nus\n", "Version"), None);
    }

    #[test]
    fn rejects_other_casc_games() {
        let root = temp_dir("overwatch");
        fs::create_dir_all(root.join("Data")).unwrap();
        fs::write(root.join(".build.info"), "Branch!STRING:0|Product!STRING:0\nus|pro\n").unwrap();
        assert!(validate(&root).is_err());
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn validates_from_subfolder_and_file() {
        let root = temp_dir("casc");
        fake_casc(&root);
        fs::create_dir_all(root.join(r"_retail_\x86_64")).unwrap();
        fs::write(root.join("Warcraft III Launcher.exe"), b"").unwrap();

        let install = validate(&root.join(r"_retail_\x86_64")).unwrap();
        assert_eq!(install.root, root);
        assert_eq!(install.version.as_deref(), Some("3.0.0.24268"));
        assert_eq!(validate(&root.join("Warcraft III Launcher.exe")).unwrap().root, root);
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn rejects_unrelated_and_missing_folders() {
        let root = temp_dir("empty");
        assert_eq!(validate(&root), Err(LocateError::NotWarcraft(root.clone())));
        fs::remove_dir_all(&root).unwrap();
        assert_eq!(validate(&root), Err(LocateError::Missing(root.clone())));
    }

    #[test]
    fn scraped_path_with_trailing_junk() {
        let root = temp_dir("junk");
        fake_casc(&root);
        let junk = PathBuf::from(format!("{}R", root.display()));
        assert_eq!(validate_scraped(&junk).unwrap().root, root);
        fs::remove_dir_all(&root).unwrap();
    }
}
