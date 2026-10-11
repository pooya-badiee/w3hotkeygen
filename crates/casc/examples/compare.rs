//! Reads files with both the pure-Rust reader and CascLib, and checks they
//! match byte for byte.
//! `cargo run -p w3hk-casc --features casclib --example compare -- <wc3 dir> [prefix...]`
//! Prefixes default to everything the app reads: units, fonts and web UI art.

use std::path::Path;
use std::time::Instant;

use w3hk_casc::{Storage, casclib};

fn main() {
    let mut args = std::env::args().skip(1);
    let dir = args.next().expect("usage: compare <wc3 dir> [prefix...]");
    let mut prefixes: Vec<String> = args.collect();
    if prefixes.is_empty() {
        prefixes = ["war3.w3mod:units/", "war3.w3mod:_locales/enus.w3mod:units/", "war3.w3mod:fonts/", "war3.w3mod:webui/"]
            .map(String::from)
            .into();
    }
    let started = Instant::now();
    let ours = Storage::open(Path::new(&dir)).expect("open (pure Rust)");
    println!("opened in {:?}", started.elapsed());
    let theirs = casclib::Storage::open(Path::new(&dir)).expect("open (CascLib)");

    let (mut same, mut failed) = (0, 0);
    for prefix in &prefixes {
        let names = ours.list(prefix).expect("list");
        println!("{prefix}: {} files", names.len());
        for name in names {
            let (a, b) = (ours.read(&name), theirs.read(&name));
            match (&a, &b) {
                (Ok(a), Ok(b)) if a == b => same += 1,
                _ => {
                    failed += 1;
                    let ours = a.as_ref().map(Vec::len).map_err(ToString::to_string);
                    let theirs = b.as_ref().map(Vec::len).map_err(ToString::to_string);
                    println!("  MISMATCH {name}: ours {ours:?}, CascLib {theirs:?}");
                }
            }
        }
    }
    println!("{same} identical, {failed} different, in {:?}", started.elapsed());
    if failed > 0 {
        std::process::exit(1);
    }
}
