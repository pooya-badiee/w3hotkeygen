//! Well-known Windows folders: drives, ProgramData, each user's AppData and
//! Documents.

use std::fs;
use std::path::{Path, PathBuf};

use winreg::RegKey;
use winreg::enums::{HKEY_CURRENT_USER, KEY_READ};

/// Drive roots that exist, e.g. `C:\`.
pub fn drives() -> Vec<PathBuf> {
    ('A'..='Z').map(|l| PathBuf::from(format!("{l}:\\"))).filter(|p| p.is_dir()).collect()
}

/// `%PROGRAMDATA%`.
pub fn program_data() -> PathBuf {
    std::env::var_os("PROGRAMDATA").map_or_else(|| r"C:\ProgramData".into(), PathBuf::from)
}

/// Every user's profile folder, the current user first.
fn user_profiles() -> Vec<PathBuf> {
    const NOT_USERS: [&str; 4] = ["public", "default", "default user", "all users"];
    let mut out: Vec<PathBuf> = subdirs(Path::new(r"C:\Users"))
        .into_iter()
        .filter(|u| u.file_name().is_some_and(|n| !NOT_USERS.contains(&n.to_string_lossy().to_lowercase().as_str())))
        .collect();
    if let Some(me) = std::env::var_os("USERNAME") {
        out.sort_by_key(|u| u.file_name() != Some(me.as_os_str()));
    }
    out
}

/// Every user's roaming AppData folder, `%APPDATA%` first.
pub fn roaming_app_data() -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> =
        user_profiles().into_iter().map(|u| u.join(r"AppData\Roaming")).filter(|p| p.is_dir()).collect();
    if let Some(p) = std::env::var_os("APPDATA") {
        out.insert(0, PathBuf::from(p));
    }
    out
}

/// Every user's Documents folder, including OneDrive redirection, the
/// current user's first. Paths may not exist.
pub fn documents() -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = native_documents().into_iter().collect();
    for profile in user_profiles() {
        out.extend(["Documents", r"OneDrive\Documents"].map(|d| profile.join(d)));
    }
    out
}

/// The current user's Documents folder as Explorer sees it (follows OneDrive
/// and other redirections).
fn native_documents() -> Option<PathBuf> {
    let key = RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey_with_flags(r"Software\Microsoft\Windows\CurrentVersion\Explorer\Shell Folders", KEY_READ)
        .ok()?;
    key.get_value::<String, _>("Personal").ok().map(PathBuf::from)
}

fn subdirs(dir: &Path) -> Vec<PathBuf> {
    fs::read_dir(dir)
        .map(|it| it.flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect())
        .unwrap_or_default()
}
