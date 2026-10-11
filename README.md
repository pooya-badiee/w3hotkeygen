# WC3 Grid Hotkeys

Grid hotkeys for Warcraft III Reforged that keep Attack on A.

Blizzard's built-in Grid preset puts every button on `QWER / ASDF / ZXCV`, which is great until you
press A out of twenty years of habit and your army walks into the enemy instead of attack-moving.
This tool writes a `CustomKeys.txt` where abilities follow the grid and the basic commands keep
their classic keys:

```text
Q       W     E     R
Attack  Stop  Move  Hold
Patrol  X     C     Learn / Rally
```

Abilities sit on Q W E R X C (and V, when a unit has no Learn or Rally there) and take that
cell's key. Attack, Stop, Move, Hold and Patrol stay on A, S, M, H and P.

It reads the unit and ability data from your own install, so the file matches whatever patch
you're on. After a patch, open it and press Apply again.

## Using it

1. Download `w3hk.exe` from [Releases](https://github.com/pooya-badiee/w3hotkeygen/releases) (Windows only).
2. Run it. It finds the game on its own; if it can't, point it at the folder.
3. Change any keys you want (QWERTZ/AZERTY players: set the grid letters by hand), then press **Apply**.
4. In game: Options > Input > Preset Keybindings > **Custom**, then restart the game.

**Reset** puts back whatever `CustomKeys.txt` you had before the first Apply.

Heads-up: the game's own keybind menu rewrites the whole file when you save in it, so don't mix
the two.

## What it doesn't do

No per-unit editing, no campaign or custom-map units, no Classic (1.29 and older). If you want a
full editor, jcfieldsdev's web hotkey editor and
[clemenscodes' warcraft-hotkey-editor](https://github.com/clemenscodes/warcraft-hotkey-editor)
are both good.

## Building

Needs Rust (version pinned in `rust-toolchain.toml`), CMake and MSVC for the CascLib build.

```sh
cargo run -p w3hk
cargo test --workspace
```

More in [docs/development.md](docs/development.md). The rest of `docs/` is notes on how the
game stores things:

- [customkeys-format.md](docs/customkeys-format.md): the file format, and the engine quirks that took the longest to figure out
- [game-files.md](docs/game-files.md): reading unit data out of CASC, and how a command card is built
- [install-detection.md](docs/install-detection.md): where Reforged hides itself on Windows
- [architecture.md](docs/architecture.md), [theme.md](docs/theme.md), [plan.md](docs/plan.md), [community-research.md](docs/community-research.md)

## Legal

No Blizzard art, icons or fonts are in this repo or the exe. The app reads them from your install
at runtime. Not affiliated with or endorsed by Blizzard Entertainment.

Licensed under either of [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option.
