//! The web page: the same screen as the exe, compiled to WASM. The game's art,
//! fonts and unit data ship with the page (`game.pack`), so it needs no
//! install. The keys file is downloaded instead of written. See docs/web.md.
#![cfg(target_arch = "wasm32")]

mod app;
mod browser;

use wasm_bindgen::prelude::*;

/// Runs the app on `canvas`. `pack` is `game.pack`: zlib-compressed
/// [`w3hk_core::pack`] of the game files.
#[wasm_bindgen]
pub async fn start(canvas: web_sys::HtmlCanvasElement, pack: Vec<u8>) -> Result<(), JsValue> {
    let raw = miniz_oxide::inflate::decompress_to_vec_zlib(&pack).map_err(|_| "game.pack isn't zlib data")?;
    let files = w3hk_core::pack::read(&raw).ok_or("game.pack is damaged")?;
    // The page keeps running for as long as it's open.
    let runner: &'static eframe::WebRunner = Box::leak(Box::new(eframe::WebRunner::new()));
    runner
        .start(
            canvas,
            eframe::WebOptions::default(),
            Box::new(move |cc| Ok(Box::new(app::WebApp::new(&cc.egui_ctx, files)))),
        )
        .await
}
