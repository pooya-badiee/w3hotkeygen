# Web version

The same egui app, compiled to WASM. It's the site's whole front page; the Windows app is a link under it. It needs no install: the game's art, fonts and unit data ship with the page. Downloads are about 5MB of WASM plus a 2.4MB data pack.

Code: [crates/web](../crates/web) (the WASM entry point and browser glue) and [site/src/pages/index.astro](../site/src/pages/index.astro) (the page). The look and the screen parts the exe also uses are in [crates/ui](../crates/ui).

## How it differs from the exe

| | Exe | Web |
|---|---|---|
| Game files | Read from the user's install | Bundled: `site/public/game.pack` |
| Unit data | Always the installed patch | The patch the pack was extracted from (shown under the buttons) |
| Output | Apply replaces `CustomKeyBindings\CustomKeys.txt`, with a backup | **Download** saves `CustomKeys.txt`. The status line says where to put it |
| Starting keys | Read from the user's file | Read from the last downloaded file (`localStorage`), else defaults |
| Reset | Restores the backup | Not shown. To undo, the user deletes the file |
| Window | Borderless window with close and minimize | A fixed 768x490 canvas on the page; a narrower desktop window scrolls. Touch-only devices (`hover: none` and `pointer: coarse`) get a "made for a computer with a keyboard" note instead, and nothing is downloaded |

## The game pack
Shipping Blizzard's files is the owner's decision (2026-10-11), and applies only to the web version. The pack holds what the app reads, and nothing else: the theme art and fonts (`theme::paths()`), the `enus` unit data (`load::paths("enus")`), and the game version.

- **Format:** [`w3hk_core::pack`](../crates/core/src/pack.rs) (a flat list of named files), zlib-compressed, because GitHub Pages doesn't compress unknown file types (8.9MB raw).
- **Extract it** from the local install (3.0.0.24268 as of now):
  ```sh
  cargo run -p w3hk-ui --example pack -- site/public/game.pack
  ```
- **After a game patch,** re-extract and commit the pack, so the web version's unit data matches the game. Until then, its file is generated from the older data.

## Flow
1. The page fetches the WASM and `game.pack` at the same time, then calls [`start`](../crates/web/src/lib.rs) with the pack's bytes.
2. The app unpacks it, decodes the art, parses the unit data, and shows the screen. That takes about half a second locally.
3. **Download** generates the file exactly like Apply (checked: byte for byte the same as the exe's output), saves it as a download, and keeps it in `localStorage`.

- **The page around it is minimal:** a plain dark background, and two small lines under the window: the in-game step (Preset Keybindings = Custom), and the Windows app (noting it's unsigned, so Windows may warn) and source links. After Download, the status line names `Documents\Warcraft III\CustomKeyBindings`.

## Build
`tools/web/build.sh` builds `crates/web` for `wasm32-unknown-unknown` and runs `wasm-bindgen --target web` into `site/public/pkg` (gitignored). [pages.yml](../.github/workflows/pages.yml) does the same before building the site. The pack is committed, because CI has no install to extract it from. `crates/web` has dependencies only on wasm32, so native workspace builds neither pull in the browser crates nor unify features with them.

**Tested** in headless Chrome (Playwright): load, set a key, Download.

## History
The first version had the user pick their Warcraft III folder and read it with the CASC reader through `File.slice()`. That's why [crates/casc](../crates/casc) has an async `Source` trait. Bundling the files replaced it, so the page needs no install and works on any browser. Commit `d83118c` has the folder-picker version.

## Later, if wanted
- Other locales' unit data in the pack (names and default hotkeys differ per locale).
- `wasm-opt` to shrink the WASM.
