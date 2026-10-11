//! Where the game reads per-user files such as `CustomKeys.txt`.

use std::path::{Path, PathBuf};

use super::folders;

/// Where the game reads `CustomKeys.txt`: an existing file if there is one,
/// else a location whose `Warcraft III` folder exists (the game has run), else
/// the most likely location.
pub fn custom_keys_path() -> Option<PathBuf> {
    let candidates: Vec<PathBuf> = folders::documents()
        .into_iter()
        .map(|d| d.join(r"Warcraft III\CustomKeyBindings\CustomKeys.txt"))
        .collect();
    pick(&candidates)
}

fn pick(candidates: &[PathBuf]) -> Option<PathBuf> {
    let game_ran = |f: &&PathBuf| f.parent().and_then(Path::parent).is_some_and(Path::is_dir);
    candidates
        .iter()
        .find(|f| f.is_file())
        .or_else(|| candidates.iter().find(game_ran))
        .or(candidates.first())
        .cloned()
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::super::tests::temp_dir;
    use super::*;

    #[test]
    fn prefers_existing_file_then_a_folder_the_game_made() {
        let root = temp_dir("keys-path");
        let file = |user: &str| root.join(user).join(r"Warcraft III\CustomKeyBindings\CustomKeys.txt");
        let candidates = [file("a"), file("b"), file("c")];
        assert_eq!(pick(&candidates), Some(file("a")));
        fs::create_dir_all(root.join(r"c\Warcraft III")).unwrap();
        assert_eq!(pick(&candidates), Some(file("c")));
        fs::create_dir_all(file("b").parent().unwrap()).unwrap();
        fs::write(file("b"), b"").unwrap();
        assert_eq!(pick(&candidates), Some(file("b")));
        fs::remove_dir_all(&root).unwrap();
    }
}
