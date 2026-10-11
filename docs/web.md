# Web version

The same egui app, compiled to WASM and served from its own page on the site (`/app`). It's heavier than a plain page (about 5MB of WASM), but it's a dedicated page and feels like downloading an app. Nothing is installed.

Code: [crates/web](../crates/web) (the WASM entry point and browser glue) and [site/src/pages/app/index.astro](../site/src/pages/app/index.astro) (the page). The look and the screen parts the exe also uses are in [crates/ui](../crates/ui).

## How it differs from the exe

| | Exe | Web |
|---|---|---|
| Find the install | Registry, folder scan | The user picks the WC3 folder on the page |
| Read game files | CASC reader on `std::fs` | The same reader, on browser `File` handles |
| Output | Apply replaces `CustomKeyBindings\CustomKeys.txt`, with a backup | **Download** saves `CustomKeys.txt`. The page says where to put it |
| Starting keys | Read from the user's file | Read from the last downloaded file (`localStorage`), else defaults |
| Reset | Restores the backup | Not shown. To undo, the user deletes the file |
| Loading | Theme and data on worker threads | Everything is read up front (`spawn_local`), then shown |
| Window | Borderless window with close and minimize | A fixed 768x490 canvas on the page |

## Flow
1. The page loads the WASM in the background and asks for the folder. It uses `showDirectoryPicker()` on Chrome and Edge, and `<input webkitdirectory>` elsewhere. Picking the folder is HTML, not egui, so the click is a real user gesture, and the app only starts once the game art is available.
2. The page passes only the files the reader opens: the root files (`.build.info`), `Data/config/**` and `Data/data/*`. They're a `Map` of path to `File`. With `showDirectoryPicker` only those folders are walked. It refuses a folder without `.build.info`.
3. [`start`](../crates/web/src/lib.rs) runs the app on the canvas. `FileSource` reads through `File.slice()`, so the multi-GB `data.###` archives are never loaded whole.
4. **Download** generates the file exactly like Apply (checked: byte for byte the same as the exe's output), saves it as a download, and keeps it in `localStorage`. **Change** reloads the page to pick another folder.

- **Where to save:** under the canvas the page names `Documents\Warcraft III\CustomKeyBindings` and mentions `OneDrive\Documents\…`, because the page can't follow OneDrive redirection the way the exe does.
- **No Blizzard assets are shipped.** Art, fonts and data come from the user's folder.

## The CASC reader
[crates/casc](../crates/casc), described in [game-files.md](game-files.md#reader-our-own-in-pure-rust). It replaced CascLib, which is C++ and doesn't build for `wasm32-unknown-unknown`.

**Async reads:** browser reads are async, so the `Source` trait is async. On native, `Storage` runs it with a trivial blocking executor, because file system reads finish on the first poll. On the web, `Casc` is awaited directly.

## Build
`tools/web/build.sh` builds `crates/web` for `wasm32-unknown-unknown` and runs `wasm-bindgen --target web` into `site/public/app/pkg` (gitignored). [pages.yml](../.github/workflows/pages.yml) does the same before building the site. `crates/web` has dependencies only on wasm32, so native workspace builds neither pull in the browser crates nor unify features with them.

**Tested** in headless Chrome (Playwright, `webkitdirectory` path): picking the folder, loading the art and unit data (about 8s in software WebGL), setting a key, and Download.

## Later, if wanted
- Remember the folder on Chrome and Edge (`FileSystemDirectoryHandle` in IndexedDB), so a returning user skips the picker.
- "Open my CustomKeys.txt", to start from a file made elsewhere.
- `wasm-opt` to shrink the WASM.
