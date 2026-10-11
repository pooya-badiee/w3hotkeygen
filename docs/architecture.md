# Architecture

A pure generator, a native layer that reads the game, and the app on top.

```
crates/app      native exe (egui/eframe, glow). Screens, config, the keys file.
crates/web      the web page's app (WASM), see web.md. Same screen, Download instead of Apply.
crates/ui       shared by app and web: the theme (theme.rs) and the screen parts
                (panels.rs: key cards, key capture, status line, footer, credits).
crates/assets   native only. Install detection + game file reading.
                game_data.rs reads the unit/ability files and hands them to core.
crates/core     pure: parses game data (data.rs, ini.rs, slk.rs) into units and command
                cards, and generates the grid CustomKeys.txt from it (grid.rs). load.rs
                lists the data files and loads them through a read callback.
                No IO, so it also compiles to WASM.
crates/casc     pure-Rust, read-only CASC reader over any file source (disk or
                browser), see game-files.md.
```

## App (`crates/app`)
- [main.rs](../crates/app/src/main.rs): borderless, fixed-size window (768x490). Not transparent: Windows fills transparent areas with a grey glass.
- [app.rs](../crates/app/src/app.rs):
  - Screens: `Ready` (install found) and `Locate` (ask the user).
  - Every UI interaction becomes an `Action`, which `App::apply` then handles.
  - Startup draws nothing until the theme loads (see [theme.md](theme.md)).
- [theme.rs](../crates/ui/src/theme.rs) (in `crates/ui`): game-art widgets with flat fallbacks. It decodes from a read function, so the exe and the web page feed it differently.
- [config.rs](../crates/app/src/config.rs): `%APPDATA%\w3hotkeygen\config.ini` holding the chosen `wc3_path`.
- [keyfile.rs](../crates/app/src/keyfile.rs): load/apply/reset the user's `CustomKeys.txt`, with a one-time backup.
- Game data loads on a worker thread when an install is chosen; Apply stays disabled until it's ready.
- [devshot.rs](../crates/app/src/devshot.rs): dev screenshot hook (see [development.md](development.md)).
- **Icon:** original art in [assets/](../crates/app/assets/), drawn by [tools/icon/make_icon.py](../tools/icon/make_icon.py) (needs Pillow). `icon.png` is the window icon; `build.rs` embeds `icon.ico` in the exe (`winresource`, build-time only).

## Startup flow (Ready screen loads the keys currently in the user's file)
1. Load the config. If its saved path validates, use it.
2. Otherwise run `find_all()`. If that finds an install, use it and save the path.
3. Otherwise show the Locate screen: path box, Browse, Search again, Scan drives.
4. Once an install is known, decode the theme on a worker thread, then upload it to the GPU on the UI thread.
