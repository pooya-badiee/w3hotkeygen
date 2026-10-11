//! The web page: the same screen as the exe, compiled to WASM. The page asks
//! for the Warcraft III folder, then calls [`start`] with its files. The keys
//! file is downloaded instead of written. See docs/web.md.
#![cfg(target_arch = "wasm32")]

mod app;
mod browser;

use wasm_bindgen::prelude::*;

/// Runs the app on `canvas`. `files` maps each path in the install folder,
/// relative to it with `/` (e.g. `Data/data/data.003`), to its `File`;
/// `folder` is the folder's name, shown in the footnote.
#[wasm_bindgen]
pub async fn start(canvas: web_sys::HtmlCanvasElement, files: js_sys::Map, folder: String) -> Result<(), JsValue> {
    // The page keeps running for as long as it's open.
    let runner: &'static eframe::WebRunner = Box::leak(Box::new(eframe::WebRunner::new()));
    let source = browser::FileSource::new(files);
    runner
        .start(
            canvas,
            eframe::WebOptions::default(),
            Box::new(move |cc| Ok(Box::new(app::WebApp::new(&cc.egui_ctx, source, folder)))),
        )
        .await
}
