# Development

## Toolchain
- **Rust 1.95,** pinned in [rust-toolchain.toml](../rust-toolchain.toml). eframe 0.36 requires it.
- **Release profile:** `opt-level="z"`, fat LTO, `codegen-units=1`, `panic=abort`, strip.
- **Size:** the exe (MSVC) is about 5.8MB.
- **Platforms:** Windows (MSVC) only.
- **CI:** [.github/workflows/ci.yml](../.github/workflows/ci.yml) runs `cargo test --workspace` and a release build on Windows.
- **Release:** push a `v*` tag. [.github/workflows/release.yml](../.github/workflows/release.yml) builds `w3hk.exe`, attaches it and its SHA-256 to a GitHub Release, and adds a build provenance attestation.
- **Exe resources** ([crates/app/build.rs](../crates/app/build.rs)): icon, version info, and a manifest ([w3hk.manifest](../crates/app/assets/w3hk.manifest)) that runs as the user (`asInvoker`, no UAC prompt). The exe is unsigned; never pack it (UPX etc.), which antivirus flags.

## Commands
```sh
cargo run -p w3hk                                   # app
cargo test -p w3hk-assets                           # finder tests
cargo run -p w3hk-assets --example find [-- --deep] # print detected installs + CustomKeys path
cargo run -p w3hk-assets --example units [-- hpea Hamg | --all] [--locale dede]  # command cards as 4x3 grids
cargo run -p w3hk-assets --example generate [-- out.txt]   # generate the grid file, list conflicts
cargo test -p w3hk-assets --test reference -- --nocapture  # data vs. the user's CustomKeys.txt (read only), generated file has no conflicts
```

## Website
[site/](../site/) is the download page, a single static Astro page, live at <https://pooya-badiee.github.io/w3hotkeygen/>. [pages.yml](../.github/workflows/pages.yml) publishes it on every push to `main` that touches `site/`. Its download links point at the latest GitHub Release; the repo URL is the `REPO` constant in [site.ts](../site/src/site.ts).
```sh
cd site && npm install && npm run dev   # local preview
npm run build                           # type check (astro check), then static output in site/dist
```

## Dev env vars
| Var | Effect |
|---|---|
| `W3HK_WC3_DIR` | Force the install path, checked first |
| `W3HK_SCREENSHOT=out.png` | Save a screenshot once the theme has loaded, then exit |
| `W3HK_SCREEN=locate` | Open straight to the Locate screen |
| `W3HK_SCREEN=credits` | Open straight to the Credits |

## Notes
- Anything you extract from the game for poking around stays local. Never commit it.
