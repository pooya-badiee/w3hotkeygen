#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod config;
mod devshot;
mod keyfile;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("WC3 Grid Hotkeys")
            .with_inner_size([768.0, 490.0])
            .with_decorations(false)
            .with_resizable(false)
            .with_icon(icon()),
        ..Default::default()
    };
    eframe::run_native("WC3 Grid Hotkeys", options, Box::new(|cc| Ok(Box::new(app::App::new(&cc.egui_ctx)))))
}

/// Window and taskbar icon (original art, see `tools/icon`).
fn icon() -> eframe::egui::IconData {
    let img = image::load_from_memory(include_bytes!("../assets/icon.png")).expect("valid icon").into_rgba8();
    let (width, height) = img.dimensions();
    eframe::egui::IconData { rgba: img.into_raw(), width, height }
}
