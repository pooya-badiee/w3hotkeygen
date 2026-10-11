//! Generates the grid `CustomKeys.txt` from the local install and prints any
//! conflicts. `cargo run -p w3hk-assets --example generate [-- out.txt]`

use w3hk_core::Keys;

fn main() {
    let found = w3hk_assets::find().expect("no Warcraft III install found (set W3HK_WC3_DIR)");
    let files = w3hk_assets::GameFiles::open(&found.install).expect("open game files");
    let data = w3hk_assets::game_data::load(&files, "enus").expect("load game data");
    let out = w3hk_core::grid::generate(&data, &Keys::default());
    eprintln!("{} lines, {} conflicts", out.text.lines().count(), out.conflicts.len());
    for c in &out.conflicts {
        eprintln!("  {c}");
    }
    match std::env::args().nth(1) {
        Some(path) => std::fs::write(path, &out.text).expect("write"),
        None => print!("{}", out.text),
    }
}
