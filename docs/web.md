# Web version

The same egui app, compiled to WASM and served from its own page on the site (`/app`). It's heavier than a plain page, but it's a dedicated page and feels like downloading an app. Nothing is installed.

## How it differs from the exe

| | Exe | Web |
|---|---|---|
| Find the install | Registry, folder scan | The user picks the WC3 folder |
| Read game files | Pure-Rust CASC reader on `std::fs` | Same reader, on browser `File` handles |
| Output | Replaces `CustomKeyBindings\CustomKeys.txt`, with a backup | **Downloads `CustomKeys.txt`**. The status line says where to put it |
| Starting keys | Read from the user's file | Defaults, or an optional "Open my CustomKeys.txt" |
| Reset | Restores the backup | Not shown. To undo, the user deletes the file |
| Settings | `config.ini` | eframe storage (`localStorage`) |

- **Picking the folder:** `showDirectoryPicker()` on Chromium. Elsewhere, `<input webkitdirectory>`. Both give lazy `File` handles, so the multi-GB `data.###` archives are read with `slice()` and never loaded whole. Chromium can keep the folder handle in IndexedDB, so a returning user doesn't have to pick it again.
- **Where to save:** the instructions name `Documents\Warcraft III\CustomKeyBindings\` and mention `OneDrive\Documents\...`, because the page can't follow OneDrive redirection the way the exe does.
- **No Blizzard assets are shipped.** Art, fonts and data still come from the user's folder.

## The CASC reader
Done: [crates/casc](../crates/casc), described in [game-files.md](game-files.md#reader-our-own-in-pure-rust). It replaced CascLib, which is C++ and doesn't build for `wasm32-unknown-unknown`.

**Async reads:** browser reads are async, so the `Source` trait is async. On native, `Storage` runs it with a trivial blocking executor, because file system reads finish on the first poll. On the web, `Casc` is awaited directly.

## Build order
1. **Done: pure-Rust CASC reader.** Checked byte for byte against CascLib on native, for every file the app reads.
2. **Done: switched the exe to it** and dropped CascLib, CMake and `vendor/`.
3. **`cfg`-split the app.** Install detection (`winreg`), `keyfile.rs` and `config.rs` become native-only. Add a wasm entry point (`eframe::WebRunner`).
4. **Web loader:** folder picker, then the file source, then the existing theme and data loading.
5. **Output:** the download, the optional import of an existing file, and the status line text.
6. **Site:** build with `wasm-bindgen` into `site/public/app/`, add an Astro page at `/app`, and make [pages.yml](../.github/workflows/pages.yml) install Rust, build the wasm, and also trigger on `crates/**`.
