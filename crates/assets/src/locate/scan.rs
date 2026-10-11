//! Last resort: a bounded breadth-first walk of every drive.

use std::collections::VecDeque;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use super::{Found, Source, detect, folders};

/// How deep below a drive root to look. `D:\Games\Blizzard\Warcraft III` is depth 3.
const MAX_DEPTH: usize = 4;

/// Folders that are huge and never hold a game install.
const SKIP: [&str; 8] =
    ["windows", "$recycle.bin", "system volume information", "appdata", "programdata", "node_modules", ".git", "winsxs"];

/// Walks every drive up to [`MAX_DEPTH`] levels. Stops early when `cancel`
/// is set. `progress` is called with each folder visited.
pub fn deep_scan(cancel: &AtomicBool, mut progress: impl FnMut(&Path)) -> Vec<Found> {
    let mut queue: VecDeque<(PathBuf, usize)> = folders::drives().into_iter().map(|p| (p, 0)).collect();
    let mut found = Vec::new();
    while let Some((dir, depth)) = queue.pop_front() {
        if cancel.load(Ordering::Relaxed) {
            break;
        }
        progress(&dir);
        if let Some(install) = detect(&dir) {
            found.push(Found { install, source: Source::DeepScan });
            continue;
        }
        if depth == MAX_DEPTH {
            continue;
        }
        let Ok(entries) = fs::read_dir(&dir) else { continue };
        for entry in entries.flatten() {
            let is_dir = entry.file_type().is_ok_and(|t| t.is_dir());
            let name = entry.file_name().to_string_lossy().to_lowercase();
            if is_dir && !SKIP.contains(&name.as_str()) {
                queue.push_back((entry.path(), depth + 1));
            }
        }
    }
    found
}
