# WC3 Grid Hotkeys

A small native app (Rust + egui) that generates Warcraft III `CustomKeys.txt` files and looks like the game's own UI.

## Docs hub

| Topic | Doc |
|---|---|
| Product plan (minimal, global apply), build order | [docs/plan.md](docs/plan.md) |
| Crates and layers | [docs/architecture.md](docs/architecture.md) |
| Finding the WC3 install | [docs/install-detection.md](docs/install-detection.md) |
| Reading game files (CASC, asset paths, fonts, unit and ability data) | [docs/game-files.md](docs/game-files.md) |
| WC3 look: theme, widgets, window | [docs/theme.md](docs/theme.md) |
| `CustomKeys.txt` format and engine quirks | [docs/customkeys-format.md](docs/customkeys-format.md) |
| Community pain points and competing tools | [docs/community-research.md](docs/community-research.md) |
| Build, toolchain, dev tools | [docs/development.md](docs/development.md) |
| Web version (egui on WASM, pure-Rust CASC reader) | [docs/web.md](docs/web.md) |

## Ground rules
- **Game assets:** the exe ships none; it reads art, icons, fonts and data from the user's install at runtime. The web version ships them in `site/public/game.pack`, extracted from an install (the owner's decision, 2026-10-11). Re-extract after a patch, see [docs/web.md](docs/web.md).
- **Keep the exe lean.** Check each new dependency's size cost before adding it.
- **Keep the core pure.** `crates/core` does no IO, so it can target WASM for the web version.
