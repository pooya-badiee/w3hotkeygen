//! Extracts every game file the web version uses (theme art, fonts, unit
//! data) from the local install into one zlib-compressed pack for the site
//! to serve. GitHub Pages doesn't compress unknown file types, so it's done here.
//! `cargo run -p w3hk-ui --example pack -- site/public/game.pack`

use w3hk_core::pack;

fn main() {
    let out = std::env::args().nth(1).expect("usage: pack <out file>");
    let found = w3hk_assets::find().expect("no Warcraft III install found (set W3HK_WC3_DIR)");
    let files = w3hk_assets::GameFiles::open(&found.install).expect("open game files");
    let version = found.install.version.unwrap_or_default();
    let mut entries = vec![(pack::VERSION.to_string(), version.clone().into_bytes())];
    for path in w3hk_ui::theme::paths().into_iter().chain(w3hk_core::load::paths("enus")) {
        match files.read(&path) {
            Some(data) => entries.push((path, data)),
            None => eprintln!("  missing: {path}"),
        }
    }
    let raw = pack::write(entries.iter().map(|(n, d)| (n.as_str(), d.as_slice())));
    let bytes = miniz_oxide::deflate::compress_to_vec_zlib(&raw, 10);
    std::fs::write(&out, &bytes).expect("write pack");
    eprintln!("{} files from Warcraft III {version}, {} bytes ({} unpacked) -> {out}", entries.len() - 1, bytes.len(), raw.len());
    let mut sizes: Vec<_> = entries.iter().map(|(n, d)| (d.len(), n)).collect();
    sizes.sort();
    for (size, name) in sizes.iter().rev().take(8) {
        eprintln!("  {size:>9}  {name}");
    }
}
