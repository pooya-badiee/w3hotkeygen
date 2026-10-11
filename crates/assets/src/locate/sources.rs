//! Candidate install paths, in priority order. Nothing here validates; the
//! caller checks each candidate and dedupes.

use std::fs;
use std::path::{Path, PathBuf};

use winreg::RegKey;
use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ};

use super::Source;
use super::extract::windows_paths;
use super::folders;

/// Folder names (relative to a drive root) where WC3 is commonly installed.
const DRIVE_SUBDIRS: [&str; 9] = [
    r"Program Files (x86)\Warcraft III",
    r"Program Files\Warcraft III",
    r"Program Files (x86)\Blizzard\Warcraft III",
    r"Program Files\Blizzard\Warcraft III",
    r"Games\Warcraft III",
    r"Games\Blizzard\Warcraft III",
    r"Blizzard\Warcraft III",
    r"Battle.net\Warcraft III",
    r"Warcraft III",
];

const BLIZZARD_KEYS: [&str; 2] =
    [r"Software\Blizzard Entertainment\Warcraft III", r"Software\WOW6432Node\Blizzard Entertainment\Warcraft III"];
const UNINSTALL_KEYS: [&str; 2] = [
    r"Software\Microsoft\Windows\CurrentVersion\Uninstall",
    r"Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall",
];

pub fn candidates() -> Vec<(PathBuf, Source)> {
    let mut out = Vec::new();
    if let Some(p) = std::env::var_os(super::ENV_OVERRIDE) {
        out.push((PathBuf::from(p), Source::EnvVar));
    }
    registry(&mut out);
    // Battle.net records the actual install.
    battle_net(&mut out);
    // Then guess common folders on every drive.
    for drive in folders::drives() {
        out.extend(DRIVE_SUBDIRS.iter().map(|s| (drive.join(s), Source::DefaultFolder)));
    }
    out
}

fn registry(out: &mut Vec<(PathBuf, Source)>) {
    for hive in [HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE] {
        let hive = RegKey::predef(hive);
        // Classic installers: InstallPath, InstallPathX, Program, ...
        for key in BLIZZARD_KEYS {
            if let Ok(k) = hive.open_subkey_with_flags(key, KEY_READ) {
                push_string_values(&k, out);
            }
        }
        // Battle.net registers each game under Uninstall with DisplayName.
        for key in UNINSTALL_KEYS {
            let Ok(uninstall) = hive.open_subkey_with_flags(key, KEY_READ) else { continue };
            for name in uninstall.enum_keys().flatten() {
                let Ok(app) = uninstall.open_subkey_with_flags(&name, KEY_READ) else { continue };
                let display: String = app.get_value("DisplayName").unwrap_or_default();
                if display.contains("Warcraft III") {
                    push_string_values(&app, out);
                }
            }
        }
    }
}

/// Every path found in any string value (InstallLocation, DisplayIcon, ...).
fn push_string_values(key: &RegKey, out: &mut Vec<(PathBuf, Source)>) {
    for (name, _) in key.enum_values().flatten() {
        let Ok(value) = key.get_value::<String, _>(&name) else { continue };
        out.extend(windows_paths(value.as_bytes()).into_iter().map(|p| (PathBuf::from(p), Source::Registry)));
    }
}

/// Battle.net agent data: `product.db` (protobuf), `aggregate.json` (launcher
/// cache with the WC3 launcher path) and each user's `Battle.net.config`
/// (`DefaultInstallPath`, where new games go).
fn battle_net(out: &mut Vec<(PathBuf, Source)>) {
    let agent = folders::program_data().join(r"Battle.net\Agent");
    for file in [agent.join("product.db"), agent.join("aggregate.json")] {
        push_scraped(&file, out);
    }
    for app_data in folders::roaming_app_data() {
        let Ok(bytes) = fs::read(app_data.join(r"Battle.net\Battle.net.config")) else { continue };
        for p in windows_paths(&bytes).into_iter().map(PathBuf::from) {
            out.push((p.join("Warcraft III"), Source::BattleNet));
            out.push((p, Source::BattleNet));
        }
    }
}

fn push_scraped(file: &Path, out: &mut Vec<(PathBuf, Source)>) {
    let Ok(bytes) = fs::read(file) else { return };
    // Only paths that look like WC3; product.db lists every Blizzard game.
    for p in windows_paths(&bytes).into_iter().filter(|p| p.contains("Warcraft")) {
        out.push((PathBuf::from(p), Source::BattleNet));
    }
}
