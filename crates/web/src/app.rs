//! The web page's screen: the exe's main screen, with Download in place of
//! Apply and Reset. Everything loads up front, because the browser reads files
//! asynchronously.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use eframe::egui::{self, Rect, UiBuilder};
use w3hk_casc::Casc;
use w3hk_core::data::GameData;
use w3hk_core::{Keys, Slot};
use w3hk_ui::panels::{self, centered_row, small, small_link};
use w3hk_ui::theme::{self, ERROR, Theme};

use crate::browser::{self, FileSource};

/// The game's art and unit data, or why the folder couldn't be read.
type Loaded = Result<(theme::Decoded, Result<GameData, String>), String>;

pub struct WebApp {
    loading: Rc<RefCell<Option<Loaded>>>,
    theme: Option<Theme>,
    /// Units and abilities. Download needs them.
    data: Option<Result<GameData, String>>,
    /// The folder couldn't be read at all.
    error: Option<String>,
    /// The picked folder's name.
    folder: String,
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

/// The credits' "Also" row.
const ALSO: &str = "image, wasm-bindgen, miniz_oxide and the other Rust crates they use (MIT, Apache-2.0 and similar).";

enum Action {
    None,
    Listen(Slot),
    Download,
    /// Back to the folder picker.
    Change,
    Credits,
}

impl WebApp {
    pub fn new(ctx: &egui::Context, source: FileSource, folder: String) -> Self {
        theme::install_fonts(ctx, None, None);
        theme::apply_visuals(ctx);
        let (keys, repaired) = panels::read_keys(&browser::saved_keys().unwrap_or_default());
        let loading = Rc::new(RefCell::new(None));
        let (cell, ctx2) = (loading.clone(), ctx.clone());
        wasm_bindgen_futures::spawn_local(async move {
            let loaded = load(source).await;
            *cell.borrow_mut() = Some(loaded);
            ctx2.request_repaint();
        });
        WebApp {
            loading,
            theme: None,
            data: None,
            error: None,
            folder,
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
                let Some(Ok(data)) = &self.data else { return };
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
            Action::Change => browser::reload(),
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
}

/// Reads everything the screen needs out of the picked folder.
async fn load(source: FileSource) -> Loaded {
    let casc = Casc::open(source).await.map_err(|e| format!("Couldn't read the game files: {e}"))?;
    let mut files = HashMap::new();
    for path in theme::paths().into_iter().chain(w3hk_core::load::paths("enus")) {
        if let Ok(bytes) = casc.read(&path).await {
            files.insert(path, bytes);
        }
    }
    let decoded = theme::decode(|path| files.get(path).cloned());
    let text = |path: &str| files.get(path).map(|b| String::from_utf8_lossy(b).into_owned());
    let data = w3hk_core::load::load(text, "enus").map_err(|e| e.to_string());
    Ok((decoded, data))
}

impl eframe::App for WebApp {
    /// Shown before the game art loads; close to the frame's dark metal.
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.04, 0.035, 0.03, 1.0]
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        if let Some(loaded) = self.loading.borrow_mut().take() {
            match loaded {
                Ok((decoded, data)) => {
                    self.theme = Some(Theme::upload(&ctx, decoded));
                    self.data = Some(data);
                }
                Err(e) => self.error = Some(e),
            }
        }
        if let Some(slot) = self.listening
            && panels::capture_key(&ctx, &mut self.keys, slot)
        {
            self.listening = None;
        }

        let theme = self.theme.as_ref();
        let content = theme::board(theme, ui.painter(), ui.max_rect());
        let mut action = Action::None;
        ui.scope_builder(UiBuilder::new().max_rect(content), |ui| {
            theme::heading(ui, "WC3 Grid Hotkeys", 24.0);
            ui.add_space(8.0);
            action = match (&self.error, theme) {
                (Some(e), _) => error_ui(ui, e),
                (None, None) => {
                    ui.add_space(120.0);
                    ui.vertical_centered(|ui| ui.label("Reading game files…"));
                    Action::None
                }
                _ if self.credits => {
                    if panels::credits(ui, theme, ALSO) { Action::Credits } else { Action::None }
                }
                _ => self.ready_ui(ui, theme),
            };
        });
        if panels::footer(ui, content, panels::DISCLAIMER) {
            action = Action::Credits;
        }
        self.apply(&ctx, action);
    }
}

fn error_ui(ui: &mut egui::Ui, error: &str) -> Action {
    ui.add_space(60.0);
    let mut action = Action::None;
    ui.vertical_centered(|ui| {
        ui.colored_label(ERROR, error);
        ui.add_space(8.0);
        ui.label("Pick the Warcraft III folder: the one with Data and .build.info in it.");
        ui.add_space(16.0);
        if theme::button(ui, None, "Pick folder", true).clicked() {
            action = Action::Change;
        }
    });
    action
}

impl WebApp {
    fn ready_ui(&self, ui: &mut egui::Ui, theme: Option<&Theme>) -> Action {
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
                let ready = matches!(self.data, Some(Ok(_)));
                let can_download = self.keys.clashes().is_empty() && self.listening.is_none() && ready;
                let download = theme::button(ui, theme, "Download", can_download);
                if download.on_hover_text("Save CustomKeys.txt with these keys").clicked() {
                    action = Action::Download;
                }
            });
            ui.add_space(4.0);
            let idle = panels::idle_status(self.data.as_ref(), &self.keys);
            panels::status_line(ui, self.status.as_ref(), self.status_at, self.listening.is_some(), idle);
        });

        let foot = Rect::from_min_max(egui::pos2(panels.left(), full.bottom() - 30.0), egui::pos2(panels.right(), full.bottom() - 8.0));
        ui.scope_builder(UiBuilder::new().max_rect(foot).layout(egui::Layout::bottom_up(egui::Align::Min)), |ui| {
            centered_row(ui, "install", |ui| {
                small(ui, format!("Game files from your “{}” folder", self.folder));
                if small_link(ui, "Change").clicked() {
                    action = Action::Change;
                }
            });
        });
        action
    }
}
