//! The web page's screen: the exe's main screen, with Download in place of
//! Apply and Reset. The game files come from the bundled pack.

use std::collections::HashMap;

use eframe::egui::{self, Rect, UiBuilder};
use w3hk_core::data::GameData;
use w3hk_core::{Keys, Slot};
use w3hk_ui::panels::{self, centered_row, small};
use w3hk_ui::theme::{self, Theme};

use crate::browser;

pub struct WebApp {
    theme: Theme,
    /// Units and abilities. Download needs them.
    data: Result<GameData, String>,
    /// The game version the bundled files came from.
    version: String,
    keys: Keys,
    /// The key box waiting for a key press.
    listening: Option<Slot>,
    /// The credits are shown in place of the screen.
    credits: bool,
    /// Result of the last Download: Ok(message) or Err(message).
    status: Option<Result<String, String>>,
    /// When `status` was last set (egui time), so the status line can flash on every click.
    status_at: f64,
}

/// Shown after a successful Download. The page explains the rest under the window.
const DOWNLOADED: &str = r"Downloaded CustomKeys.txt: put it in Documents\Warcraft III\CustomKeyBindings.";

/// The credits' "Game art" row.
const ART: &str =
    "Icons, art, fonts and unit data belong to Blizzard Entertainment, extracted from Warcraft III and included here.";

/// The credits' "Also" row.
const ALSO: &str = "image, wasm-bindgen, miniz_oxide and the other Rust crates they use (MIT, Apache-2.0 and similar).";

enum Action {
    None,
    Listen(Slot),
    Download,
    Credits,
}

impl WebApp {
    pub fn new(ctx: &egui::Context, files: HashMap<String, Vec<u8>>) -> Self {
        theme::apply_visuals(ctx);
        let theme = Theme::upload(ctx, theme::decode(|path| files.get(path).cloned()));
        let text = |path: &str| files.get(path).map(|b| String::from_utf8_lossy(b).into_owned());
        let data = w3hk_core::load::load(text, "enus").map_err(|e| e.to_string());
        let version = text(w3hk_core::pack::VERSION).unwrap_or_default();
        let (keys, repaired) = panels::read_keys(&browser::saved_keys().unwrap_or_default());
        WebApp {
            theme,
            data,
            version,
            keys,
            listening: None,
            credits: false,
            status: repaired.map(Ok),
            status_at: f64::NEG_INFINITY,
        }
    }

    fn apply(&mut self, ctx: &egui::Context, action: Action) {
        match action {
            Action::None => {}
            Action::Listen(slot) => {
                self.listening = if self.listening == Some(slot) { None } else { Some(slot) };
                self.set_status(ctx, None);
            }
            Action::Download => {
                let Ok(data) = &self.data else { return };
                let out = w3hk_core::grid::generate(data, &self.keys);
                let status = if let Some(first) = out.conflicts.first() {
                    Err(format!("Not downloaded: the layout has {} conflicts ({first}).", out.conflicts.len()))
                } else {
                    match browser::download("CustomKeys.txt", &out.text) {
                        Ok(()) => {
                            browser::save_keys(&out.text);
                            Ok(DOWNLOADED.into())
                        }
                        Err(e) => Err(format!("Couldn't download the keys file: {e:?}")),
                    }
                };
                self.set_status(ctx, Some(status));
            }
            Action::Credits => {
                self.credits = !self.credits;
                self.listening = None;
            }
        }
    }

    fn set_status(&mut self, ctx: &egui::Context, status: Option<Result<String, String>>) {
        self.status = status;
        self.status_at = ctx.input(|i| i.time);
    }

    fn ready_ui(&self, ui: &mut egui::Ui) -> Action {
        let theme = Some(&self.theme);
        let mut action = Action::None;
        let full = ui.max_rect();
        let (clicked, panels) = panels::cards(ui, theme, &self.keys, self.listening);
        if let Some(slot) = clicked {
            action = Action::Listen(slot);
        }

        // Below the cards, centred on them: the button, then the status line,
        // then the footnote above the disclaimer.
        ui.add_space(10.0);
        let below = Rect::from_min_max(egui::pos2(panels.left(), ui.cursor().top()), egui::pos2(panels.right(), full.bottom()));
        ui.scope_builder(UiBuilder::new().max_rect(below), |ui| {
            centered_row(ui, "buttons", |ui| {
                let can_download = self.keys.clashes().is_empty() && self.listening.is_none() && self.data.is_ok();
                let download = theme::button(ui, theme, "Download", can_download);
                if download.on_hover_text("Save CustomKeys.txt with these keys").clicked() {
                    action = Action::Download;
                }
            });
            ui.add_space(4.0);
            let idle = panels::idle_status(Some(&self.data), &self.keys);
            panels::status_line(ui, self.status.as_ref(), self.status_at, self.listening.is_some(), idle);
        });

        let foot = Rect::from_min_max(egui::pos2(panels.left(), full.bottom() - 30.0), egui::pos2(panels.right(), full.bottom() - 8.0));
        ui.scope_builder(UiBuilder::new().max_rect(foot).layout(egui::Layout::bottom_up(egui::Align::Min)), |ui| {
            centered_row(ui, "version", |ui| {
                small(ui, format!("Unit data from Warcraft III {}", self.version));
            });
        });
        action
    }
}

impl eframe::App for WebApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.04, 0.035, 0.03, 1.0]
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        if let Some(slot) = self.listening
            && panels::capture_key(&ctx, &mut self.keys, slot)
        {
            self.listening = None;
        }

        let content = theme::board(Some(&self.theme), ui.painter(), ui.max_rect());
        let mut action = Action::None;
        ui.scope_builder(UiBuilder::new().max_rect(content), |ui| {
            theme::heading(ui, "WC3 Grid Hotkeys", 24.0);
            ui.add_space(8.0);
            action = if self.credits {
                if panels::credits(ui, Some(&self.theme), ART, ALSO) { Action::Credits } else { Action::None }
            } else {
                self.ready_ui(ui)
            };
        });
        if panels::footer(ui, content, panels::DISCLAIMER) {
            action = Action::Credits;
        }
        self.apply(&ctx, action);
    }
}
