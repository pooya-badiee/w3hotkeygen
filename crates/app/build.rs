//! Embeds the exe icon, version info (Explorer's Details tab) and manifest.

fn main() {
    println!("cargo:rerun-if-changed=assets/icon.ico");
    println!("cargo:rerun-if-changed=assets/w3hk.manifest");
    let mut res = winresource::WindowsResource::new();
    res.set_icon("assets/icon.ico")
        .set("ProductName", "WC3 Grid Hotkeys")
        .set("FileDescription", "WC3 Grid Hotkeys")
        .set("OriginalFilename", "w3hk.exe")
        .set("LegalCopyright", "Copyright (c) 2026 pooya. MIT or Apache-2.0.")
        .set_manifest_file("assets/w3hk.manifest");
    res.compile().expect("embed exe resources");
}
