#!/bin/sh
# Builds the web version (crates/web) into site/public/app/pkg, where the
# site's /app page loads it. Needs the wasm32 target (rust-toolchain.toml)
# and wasm-bindgen-cli at the same version as the wasm-bindgen crate:
#   cargo install wasm-bindgen-cli --version 0.2.129 --locked
set -e
cd "$(dirname "$0")/../.."
cargo build --release --target wasm32-unknown-unknown -p w3hk-web
wasm-bindgen --target web --no-typescript --out-dir site/public/app/pkg \
  target/wasm32-unknown-unknown/release/w3hk_web.wasm
