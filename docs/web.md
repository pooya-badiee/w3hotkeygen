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
CascLib is C++, so it doesn't build for `wasm32-unknown-unknown`. It gets replaced by a small read-only reader written in pure Rust. The app only needs the calls "open storage" and "read file by name" ([crates/casc/src/lib.rs](../crates/casc/src/lib.rs)).

Warcraft III layout, as CascLib reads it:
1. `.build.info` → build config → encoding key and `vfs-root` key.
2. `Data\data\*.idx` → where each encoded key sits in `data.###`.
3. BLTE blocks: plain and zlib (`miniz_oxide`, pure Rust). Check whether any of the files we read are encrypted.
4. Encoding table: content key → encoded key.
5. TVFS root (`vfs-root`), which resolves names like `war3.w3mod:units\humanunitfunc.txt`.

The reader takes a random-access source (a trait), backed by `std::fs` on native and by `File.slice()` on the web.

**Open question: async reads.** Browser reads are async, and the native reader is sync. The options are:
- an async source trait, with native using a trivial blocking executor, or
- running the loader in a Web Worker and using `FileReaderSync`, so the reader stays sync.

The app already loads game data and the theme off the UI thread and polls for the result, so either option fits that flow.

## Build order
1. **Pure-Rust CASC reader.** Test it byte for byte against CascLib on native, for every file the app reads.
2. **Switch the exe to it**, then drop CascLib, CMake and `vendor/`. This gives one code path, a smaller exe and simpler CI.
3. **`cfg`-split the app.** Install detection (`winreg`), `keyfile.rs` and `config.rs` become native-only. Add a wasm entry point (`eframe::WebRunner`).
4. **Web loader:** folder picker, then the file source, then the existing theme and data loading.
5. **Output:** the download, the optional import of an existing file, and the status line text.
6. **Site:** build with `wasm-bindgen` into `site/public/app/`, add an Astro page at `/app`, and make [pages.yml](../.github/workflows/pages.yml) install Rust, build the wasm, and also trigger on `crates/**`.
