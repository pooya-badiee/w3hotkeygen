# Install detection

Code: [crates/assets/src/locate/](../crates/assets/src/locate/). CLI check: `cargo run -p w3hk-assets --example find [-- --deep]`.

Windows only, since that's the only build.

## Validation (`validate`)
A folder is accepted as an install if it has `.build.info` + `Data/` (CASC, 1.30+ / Reforged), with `Product` = `w3`. This rejects other Blizzard games that use the same layout. Classic (MPQ, ≤1.29) installs aren't recognized.

Lenient inputs are also accepted:
- A file inside the install, e.g. the launcher exe.
- A subfolder such as `_retail_\x86_64`. Validation walks up at most 2 levels.

Paths scraped from binary files may get up to 2 trailing junk characters trimmed before validating.

The version is read from the `Version` column of `.build.info`.

## Sources (`find_all`), in priority order
1. **`W3HK_WC3_DIR` env var.**
2. **Windows registry:**
   - `Blizzard Entertainment\Warcraft III` keys (HKCU/HKLM, including WOW6432Node). Every string value is scanned for paths.
   - Every `Uninstall` entry whose `DisplayName` contains "Warcraft III".
3. **Battle.net:**
   - `%PROGRAMDATA%\Battle.net\Agent\product.db`: protobuf with length bytes in front of each path. Paths are scraped and only ones containing "Warcraft" are kept.
   - `...\Agent\aggregate.json`: holds the `icon_path` of the WC3 launcher.
   - Each user's `AppData\Roaming\Battle.net\Battle.net.config`: `DefaultInstallPath`. Both the folder itself and `<folder>\Warcraft III` are tried.
4. **Default folders:** 9 common subfolders checked on every drive.

Deep scan (`deep_scan`, manual "Scan drives" button):
- Walks every drive breadth-first, up to 4 levels deep.
- Skips system folders such as `Windows`, `AppData` and `node_modules`.
- Can be cancelled.

## User files (`custom_keys_path`)
Code: [user_files.rs](../crates/assets/src/locate/user_files.rs). The game reads `Documents\Warcraft III\CustomKeyBindings\CustomKeys.txt`. Candidates, in order:
- The current user's Documents from the `Shell Folders\Personal` registry value, so OneDrive redirection is followed. Verified: on a OneDrive machine, `%USERPROFILE%\Documents\Warcraft III` doesn't exist.
- Each user's `Documents` and `OneDrive\Documents` under `C:\Users`, the current user first.

Picking: an existing `CustomKeys.txt` wins, then a candidate whose `Warcraft III` folder exists (the game has run), then the first candidate.

## Verified
- Registry detection and the OneDrive Documents path (Reforged 3.0.0).

## Unverified
- It's unknown whether Reforged writes the classic `InstallPath` registry value.
