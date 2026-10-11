//! Prints every Warcraft III install the finder can see.
//! `cargo run -p w3hk-assets --example find [-- --deep]`

use std::sync::atomic::AtomicBool;

fn main() {
    let deep = std::env::args().any(|a| a == "--deep");
    let found = if deep { w3hk_assets::deep_scan(&AtomicBool::new(false), |_| {}) } else { w3hk_assets::find_all() };
    if found.is_empty() {
        println!("No Warcraft III install found.");
    }
    if let Some(keys) = w3hk_assets::custom_keys_path() {
        println!("CustomKeys: {}", keys.display());
    }
    for f in found {
        let version = f.install.version.as_deref().unwrap_or("?");
        println!("{}  [{version}]  via {}", f.install.root.display(), f.source);
    }
}
