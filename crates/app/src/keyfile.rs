//! Reading and writing the user's `CustomKeys.txt`, with a backup so Reset can
//! put back what they had before the first Apply.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use w3hk_core::Keys;

/// Next to the keys file. Written once, on the first Apply, and removed by
/// Reset. Empty means there was no keys file before.
fn backup_path(file: &Path) -> PathBuf {
    file.with_extension("txt.w3hk-backup")
}

pub fn has_backup(file: &Path) -> bool {
    backup_path(file).is_file()
}

/// The keys in the file now (defaults if it doesn't exist).
pub fn load(file: &Path) -> Keys {
    Keys::read(&fs::read_to_string(file).unwrap_or_default())
}

/// Replaces the file with `text`, backing up the original first.
pub fn apply(file: &Path, text: &str) -> io::Result<()> {
    let existing = match fs::read_to_string(file) {
        Ok(text) => text,
        Err(e) if e.kind() == io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(e),
    };
    if let Some(dir) = file.parent() {
        fs::create_dir_all(dir)?;
    }
    let backup = backup_path(file);
    if !backup.exists() {
        fs::write(&backup, &existing)?;
    }
    fs::write(file, text)
}

/// Puts back the file from before the first Apply.
pub fn reset(file: &Path) -> io::Result<()> {
    let backup = backup_path(file);
    let original = fs::read(&backup)?;
    if original.is_empty() {
        match fs::remove_file(file) {
            Err(e) if e.kind() != io::ErrorKind::NotFound => return Err(e),
            _ => {}
        }
    } else {
        fs::write(file, original)?;
    }
    fs::remove_file(backup)
}

#[cfg(test)]
mod tests {
    use super::*;
    use w3hk_core::{Key, Slot};

    fn temp_file(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("w3hk-keyfile-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir.join("CustomKeyBindings/CustomKeys.txt")
    }

    #[test]
    fn apply_then_reset_restores_original() {
        let file = temp_file("restore");
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(&file, "[a002]\nHotkey=W\n").unwrap();

        apply(&file, "[CmdAttack]\nHotkey=K\n").unwrap();
        apply(&file, "[CmdAttack]\nHotkey=L\n").unwrap(); // second apply keeps the first backup
        assert_eq!(load(&file).get(Slot::Attack), Key::letter('L'));
        assert!(has_backup(&file));

        reset(&file).unwrap();
        assert_eq!(fs::read_to_string(&file).unwrap(), "[a002]\nHotkey=W\n");
        assert!(!has_backup(&file));
        fs::remove_dir_all(file.parent().unwrap().parent().unwrap()).unwrap();
    }

    #[test]
    fn reset_removes_a_file_we_created() {
        let file = temp_file("created");
        apply(&file, "[CmdAttack]\nHotkey=A\n").unwrap();
        assert!(file.is_file());
        reset(&file).unwrap();
        assert!(!file.exists());
        fs::remove_dir_all(file.parent().unwrap().parent().unwrap()).unwrap();
    }
}
