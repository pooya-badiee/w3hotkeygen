use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use eframe::egui::{self, Color32, FontId, Rect, RichText, UiBuilder, vec2};
use w3hk_assets::{Found, GameFiles, Install, custom_keys_path, deep_scan, find_all, validate};
use w3hk_core::data::GameData;
use w3hk_core::{Keys, Slot};
use w3hk_ui::panels::{self, centered_row, small, small_link, subheading};
use w3hk_ui::theme::{self, ERROR, Theme};

use crate::config::Config;
use crate::devshot::DevShot;
use crate::keyfile;

pub struct App {
    config: Config,
    screen: Screen,
    theme: Option<Theme>,
    theme_loader: Option<ThemeLoader>,
    theme_error: Option<String>,
    devshot: Option<DevShot>,
    started: Instant,
    keys: Keys,
    /// The key box waiting for a key press.
    listening: Option<Slot>,
    /// The credits are shown in place of the screen.
    credits: bool,
}

enum Screen {
    Ready(Box<Ready>),
    Locate(Locate),
}

struct Ready {
    install: Install,
    /// How we got it, e.g. "Battle.net"; `None` when it was saved.
    origin: Option<String>,
    /// Other installs detected, so the user can switch.
    others: Vec<Found>,
    save_error: Option<String>,
    /// Where the game reads `CustomKeys.txt`, if known.
    keys_file: Option<PathBuf>,
    /// Result of the last Apply/Reset: Ok(message) or Err(message).
    status: Option<Result<String, String>>,
    /// When `status` was last set (egui time), so the status line can flash on every click.
    status_at: f64,
    /// Units and abilities, read on a worker thread. Apply needs them.
    data: Option<Result<GameData, String>>,
    data_loader: Option<Receiver<Result<GameData, String>>>,
}

impl Ready {
    fn new(
        ctx: &egui::Context,
        install: Install,
        origin: Option<String>,
        others: Vec<Found>,
        save_error: Option<String>,
    ) -> Self {
        let keys_file = custom_keys_path();
        let (tx, rx) = mpsc::channel();
        let (ctx, for_thread) = (ctx.clone(), install.clone());
        thread::spawn(move || {
            let files = GameFiles::open(&for_thread).map_err(|e| e.to_string());
            let _ = tx.send(files.and_then(|f| w3hk_assets::game_data::load(&f, "enus").map_err(|e| e.to_string())));
            ctx.request_repaint();
        });
        Ready { install, origin, others, save_error, keys_file, status: None, status_at: f64::NEG_INFINITY, data: None, data_loader: Some(rx) }
    }
}

/// Theme art decoded on a worker thread, uploaded on the UI thread.
struct ThemeLoader(Receiver<Result<theme::Decoded, String>>);

impl ThemeLoader {
    fn spawn(ctx: &egui::Context, install: Install) -> Self {
        let (tx, rx) = mpsc::channel();
        let ctx = ctx.clone();
        thread::spawn(move || {
            let files = GameFiles::open(&install).map_err(|e| e.to_string());
            let _ = tx.send(files.map(|f| theme::decode(|path| f.read(path))));
            ctx.request_repaint();
        });
        ThemeLoader(rx)
    }

    /// `Some` once loading finished.
    fn poll(&self, ctx: &egui::Context) -> Option<Result<Theme, String>> {
        Some(self.0.try_recv().ok()?.map(|d| Theme::upload(ctx, d)))
    }
}

#[derive(Default)]
struct Locate {
    input: String,
    error: Option<String>,
    scan: Option<Scan>,
}

struct Scan {
    cancel: Arc<AtomicBool>,
    current: Arc<Mutex<PathBuf>>,
    result: Receiver<Vec<Found>>,
}

impl Scan {
    fn start() -> Self {
        let cancel = Arc::new(AtomicBool::new(false));
        let current = Arc::new(Mutex::new(PathBuf::new()));
        let (tx, result) = mpsc::channel();
        let (c, cur) = (cancel.clone(), current.clone());
        thread::spawn(move || {
            let found = deep_scan(&c, |p| *cur.lock().unwrap() = p.to_path_buf());
            let _ = tx.send(found);
        });
        Scan { cancel, current, result }
    }
}

impl App {
    pub fn new(ctx: &egui::Context) -> Self {
        theme::install_fonts(ctx, None, None);
        theme::apply_visuals(ctx);
        let config = Config::load();
        let saved = config.wc3_path.as_deref().and_then(|p| validate(p).ok());
        let mut app = App {
            config,
            screen: Screen::Locate(Locate::default()),
            theme: None,
            theme_loader: None,
            theme_error: None,
            devshot: DevShot::from_env(),
            started: Instant::now(),
            keys: Keys::default(),
            listening: None,
            credits: false,
        };
        match saved {
            Some(install) => {
                let others = find_all().into_iter().filter(|f| f.install.root != install.root).collect();
                app.load_theme(ctx, &install);
                app.show_ready(Ready::new(ctx, install, None, others, None));
            }
            None => app.auto_detect(ctx),
        }
        // Dev aid: preview the locate screen or the credits.
        match std::env::var_os("W3HK_SCREEN") {
            Some(s) if s == "locate" => app.apply(ctx, Action::Change),
            Some(s) if s == "credits" => app.apply(ctx, Action::Credits),
            _ => {}
        }
        app
    }

    fn load_theme(&mut self, ctx: &egui::Context, install: &Install) {
        self.theme_error = None;
        self.theme_loader = Some(ThemeLoader::spawn(ctx, install.clone()));
    }

    fn auto_detect(&mut self, ctx: &egui::Context) {
        let mut found = find_all();
        if found.is_empty() {
            let input = self.config.wc3_path.as_ref().map(|p| p.display().to_string()).unwrap_or_default();
            self.screen = Screen::Locate(Locate {
                input,
                error: Some("Couldn't find Warcraft III automatically.".into()),
                scan: None,
            });
        } else {
            let first = found.remove(0);
            self.use_install(ctx, first.install, first.source.to_string(), found);
        }
    }

    fn use_install(&mut self, ctx: &egui::Context, install: Install, origin: String, others: Vec<Found>) {
        self.config.wc3_path = Some(install.root.clone());
        let save_error = self.config.save().err().map(|e| format!("Couldn't save settings: {e}"));
        self.load_theme(ctx, &install);
        self.show_ready(Ready::new(ctx, install, Some(origin), others, save_error));
    }

    /// Shows the main screen, with the keys currently in the user's file.
    fn show_ready(&mut self, mut ready: Ready) {
        let (keys, repaired) = load_keys(ready.keys_file.as_deref());
        self.keys = keys;
        ready.status = repaired.map(Ok);
        self.listening = None;
        self.screen = Screen::Ready(Box::new(ready));
    }

    fn try_path(&mut self, ctx: &egui::Context, path: &Path) {
        match validate(path) {
            Ok(install) => self.use_install(ctx, install, "chosen".into(), Vec::new()),
            Err(e) => {
                if let Screen::Locate(l) = &mut self.screen {
                    l.error = Some(e.to_string());
                }
            }
        }
    }
}

enum Action {
    None,
    Change,
    Switch(Found),
    Browse,
    UseInput,
    AutoDetect,
    StartScan,
    CancelScan,
    ScanDone(Vec<Found>),
    /// Start (or stop) listening for a new key for this slot.
    Listen(Slot),
    /// Write the keys into the user's file.
    ApplyKeys,
    /// Put back the file from before the first Apply.
    Reset,
    /// Show or hide the credits.
    Credits,
}

/// Shown after a successful Apply.
const APPLIED: &str =
    "Applied. In game, set Options > Input > Preset Keybindings to Custom, and restart the game if it's running.";

/// Longest we wait for game art before showing the fallback style.
const FIRST_PAINT_TIMEOUT: Duration = Duration::from_secs(3);

impl eframe::App for App {
    /// Shown before the game art loads; close to the frame's dark metal.
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.04, 0.035, 0.03, 1.0]
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        if let Some(result) = self.theme_loader.as_ref().and_then(|l| l.poll(&ctx)) {
            self.theme_loader = None;
            match result {
                Ok(t) => self.theme = Some(t),
                Err(e) => self.theme_error = Some(e),
            }
        }

        // Avoid a flash of the fallback style: show only the dark clear color
        // until the game art is ready.
        if self.theme.is_none() && self.theme_loader.is_some() && self.started.elapsed() < FIRST_PAINT_TIMEOUT {
            ctx.request_repaint_after(Duration::from_millis(16));
            return;
        }

        if let Some(slot) = self.listening
            && panels::capture_key(&ctx, &mut self.keys, slot)
        {
            self.listening = None;
        }
        if let Screen::Ready(r) = &mut self.screen
            && let Some(result) = r.data_loader.as_ref().and_then(|rx| rx.try_recv().ok())
        {
            r.data = Some(result);
            r.data_loader = None;
        }

        // Borderless window: the board *is* the window. It isn't transparent,
        // because Windows fills transparent areas with a grey glass.
        let board = ui.max_rect();
        let theme = self.theme.as_ref();

        // Dragging anywhere that isn't a widget moves the window.
        let drag = ui.interact(board,egui::Id::new("window-drag"), egui::Sense::click_and_drag());
        if drag.drag_started() {
            ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
        }

        let content = theme::board(theme, ui.painter(), board);
        let (close, minimize) = theme::window_controls(ui, theme, board);
        if close.clicked() {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        if minimize.clicked() {
            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
        }

        let mut action = Action::None;
        ui.scope_builder(UiBuilder::new().max_rect(content), |ui| {
            theme::heading(ui, "WC3 Grid Hotkeys", 24.0);
            ui.add_space(8.0);
            action = match &mut self.screen {
                _ if self.credits => {
                    if panels::credits(ui, theme, ALSO) { Action::Credits } else { Action::None }
                }
                Screen::Ready(r) => ready_ui(ui, theme, r, &self.keys, self.listening),
                Screen::Locate(l) => locate_ui(ui, theme, l),
            };
        });

        // Footer: disclaimer + theme status.
        let footer = match (&self.theme_loader, &self.theme_error) {
            (Some(_), _) => "Loading game art…".to_string(),
            (_, Some(e)) => format!("Game art unavailable: {e}"),
            _ => panels::DISCLAIMER.to_string(),
        };
        if panels::footer(ui, content, &footer) {
            action = Action::Credits;
        }

        if let Screen::Locate(Locate { scan: Some(scan), .. }) = &self.screen {
            if let Ok(found) = scan.result.try_recv() {
                action = Action::ScanDone(found);
            } else {
                ctx.request_repaint_after(Duration::from_millis(100));
            }
        }
        self.apply(&ctx, action);
        if let Some(shot) = &mut self.devshot {
            shot.tick(&ctx, self.theme_loader.is_none());
        }
    }
}

impl App {
    fn set_status(&mut self, ctx: &egui::Context, status: Option<Result<String, String>>) {
        if let Screen::Ready(r) = &mut self.screen {
            r.status = status;
            r.status_at = ctx.input(|i| i.time);
        }
    }

    fn apply(&mut self, ctx: &egui::Context, action: Action) {
        match action {
            Action::None => {}
            Action::Credits => {
                self.credits = !self.credits;
                self.listening = None;
            }
            Action::Listen(slot) => {
                self.listening = if self.listening == Some(slot) { None } else { Some(slot) };
                self.set_status(ctx, None);
            }
            Action::ApplyKeys => {
                let Screen::Ready(r) = &mut self.screen else { return };
                let (Some(file), Some(Ok(data))) = (&r.keys_file, &r.data) else { return };
                let out = w3hk_core::grid::generate(data, &self.keys);
                let status = if let Some(first) = out.conflicts.first() {
                    Err(format!("Not applied: the layout has {} conflicts ({first}).", out.conflicts.len()))
                } else {
                    match keyfile::apply(file, &out.text) {
                        Ok(()) => Ok(APPLIED.into()),
                        Err(e) => Err(format!("Couldn't write the keys file: {e}")),
                    }
                };
                self.set_status(ctx, Some(status));
            }
            Action::Reset => {
                let Screen::Ready(r) = &mut self.screen else { return };
                let Some(file) = r.keys_file.clone() else { return };
                match keyfile::reset(&file) {
                    Ok(()) => {
                        let (keys, repaired) = load_keys(Some(&file));
                        self.keys = keys;
                        let msg = "Restored the keys file you had before.";
                        let msg = repaired.map_or(msg.into(), |r| format!("{msg} {r}"));
                        self.set_status(ctx, Some(Ok(msg)));
                    }
                    Err(e) => self.set_status(ctx, Some(Err(format!("Couldn't restore the keys file: {e}")))),
                }
            }
            Action::Change => {
                let input = match &self.screen {
                    Screen::Ready(r) => r.install.root.display().to_string(),
                    Screen::Locate(l) => l.input.clone(),
                };
                self.screen = Screen::Locate(Locate { input, ..Default::default() });
            }
            Action::Switch(f) => {
                let mut others = match &mut self.screen {
                    Screen::Ready(r) => std::mem::take(&mut r.others),
                    Screen::Locate(_) => Vec::new(),
                };
                if let Screen::Ready(r) = &self.screen {
                    others.retain(|o| o.install.root != f.install.root);
                    others.push(Found { install: r.install.clone(), source: w3hk_assets::Source::DefaultFolder });
                }
                self.use_install(ctx, f.install, f.source.to_string(), others);
            }
            Action::Browse => {
                let start = match &self.screen {
                    Screen::Locate(l) if Path::new(&l.input).is_dir() => Some(PathBuf::from(&l.input)),
                    _ => None,
                };
                let mut dialog = rfd::FileDialog::new().set_title("Select your Warcraft III folder");
                if let Some(dir) = start {
                    dialog = dialog.set_directory(dir);
                }
                if let Some(dir) = dialog.pick_folder() {
                    if let Screen::Locate(l) = &mut self.screen {
                        l.input = dir.display().to_string();
                    }
                    self.try_path(ctx, &dir);
                }
            }
            Action::UseInput => {
                if let Screen::Locate(l) = &self.screen {
                    let path = PathBuf::from(l.input.trim());
                    self.try_path(ctx, &path);
                }
            }
            Action::AutoDetect => self.auto_detect(ctx),
            Action::StartScan => {
                if let Screen::Locate(l) = &mut self.screen {
                    l.error = None;
                    l.scan = Some(Scan::start());
                }
            }
            Action::CancelScan => {
                if let Screen::Locate(l) = &mut self.screen
                    && let Some(scan) = l.scan.take() {
                        scan.cancel.store(true, Ordering::Relaxed);
                    }
            }
            Action::ScanDone(mut found) => {
                if found.is_empty() {
                    if let Screen::Locate(l) = &mut self.screen {
                        l.scan = None;
                        l.error = Some("Drive scan finished without finding Warcraft III.".into());
                    }
                } else {
                    let first = found.remove(0);
                    self.use_install(ctx, first.install, first.source.to_string(), found);
                }
            }
        }
    }
}

/// The keys in the user's file; see [`panels::read_keys`].
fn load_keys(file: Option<&Path>) -> (Keys, Option<String>) {
    panels::read_keys(&file.map(keyfile::read).unwrap_or_default())
}

/// The credits' "Also" row.
const ALSO: &str = "image, rfd, winreg, miniz_oxide and the other Rust crates they use (MIT, Apache-2.0 and similar).";

fn ready_ui(ui: &mut egui::Ui, theme: Option<&Theme>, r: &Ready, keys: &Keys, listening: Option<Slot>) -> Action {
    let mut action = Action::None;
    let full = ui.max_rect();
    if let Some(e) = &r.save_error {
        ui.colored_label(ERROR, e);
    }

    let (clicked, panels) = panels::cards(ui, theme, keys, listening);
    if let Some(slot) = clicked {
        action = Action::Listen(slot);
    }

    // Everything below the cards is centred on them: buttons, then the
    // status line, then the footnote above the disclaimer.
    ui.add_space(10.0);
    let below = Rect::from_min_max(egui::pos2(panels.left(), ui.cursor().top()), egui::pos2(panels.right(), full.bottom()));
    ui.scope_builder(UiBuilder::new().max_rect(below), |ui| {
        centered_row(ui, "buttons", |ui| {
            let data_ready = matches!(r.data, Some(Ok(_)));
            let can_apply = keys.clashes().is_empty() && listening.is_none() && r.keys_file.is_some() && data_ready;
            if theme::button(ui, theme, "Apply", can_apply).clicked() {
                action = Action::ApplyKeys;
            }
            ui.add_space(8.0);
            let can_reset = r.keys_file.as_deref().is_some_and(crate::keyfile::has_backup);
            let reset = theme::button(ui, theme, "Reset", can_reset);
            if reset.on_hover_text("Put back the keys file you had before the first Apply").clicked() {
                action = Action::Reset;
            }
        });
        ui.add_space(4.0);
        let idle = panels::idle_status(r.data.as_ref(), keys);
        panels::status_line(ui, r.status.as_ref(), r.status_at, listening.is_some(), idle);
    });

    // Footnote above the disclaimer: the install, and others found.
    let foot = Rect::from_min_max(egui::pos2(panels.left(), full.bottom() - 50.0), egui::pos2(panels.right(), full.bottom() - 8.0));
    ui.scope_builder(UiBuilder::new().max_rect(foot).layout(egui::Layout::bottom_up(egui::Align::Min)), |ui| {
        ui.spacing_mut().item_spacing.y = 2.0;
        centered_row(ui, "install", |ui| {
            let version = r.install.version.as_deref().unwrap_or("");
            let origin = r.origin.as_ref().map_or_else(String::new, |o| format!("  ·  found via {o}"));
            let keys_file = r.keys_file.as_ref().map_or_else(|| "unknown".into(), |p| p.display().to_string());
            let install = format!("Warcraft III {version}  ·  {}{origin}", r.install.root.display());
            small(ui, install).on_hover_text(format!("Keys file: {keys_file}"));
            if small_link(ui, "Change").clicked() {
                action = Action::Change;
            }
        });
        if !r.others.is_empty() {
            centered_row(ui, "others", |ui| {
                ui.label(RichText::new("Also found:").size(13.0).color(Color32::from_gray(160)));
                for f in &r.others {
                    if small_link(ui, &f.install.root.display().to_string()).clicked() {
                        action = Action::Switch(f.clone());
                    }
                }
            });
        }
    });
    action
}

/// Example install path shown in the empty path box.
const INSTALL_HINT: &str = r"C:\Program Files (x86)\Warcraft III";

fn locate_ui(ui: &mut egui::Ui, theme: Option<&Theme>, l: &mut Locate) -> Action {
    let mut action = Action::None;
    subheading(ui, "Where is Warcraft III installed?");
    ui.label("Select the game folder: the one with .build.info and Data.");
    ui.add_space(12.0);
    let scanning = l.scan.is_some();
    ui.add_enabled_ui(!scanning, |ui| {
        let edit = ui.add(
            egui::TextEdit::singleline(&mut l.input)
                .hint_text(INSTALL_HINT)
                .font(FontId::proportional(15.0))
                .margin(vec2(10.0, 8.0))
                .desired_width(ui.available_width()),
        );
        if edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
            action = Action::UseInput;
        }
    });
    ui.add_space(12.0);
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        if theme::button(ui, theme, "Browse", !scanning).clicked() {
            action = Action::Browse;
        }
        if theme::button(ui, theme, "Use folder", !scanning && !l.input.trim().is_empty()).clicked() {
            action = Action::UseInput;
        }
        if theme::button(ui, theme, "Search again", !scanning).clicked() {
            action = Action::AutoDetect;
        }
        let scan = theme::button(ui, theme, "Scan drives", !scanning);
        if scan.on_hover_text("Slower: walks your drives looking for the game").clicked() {
            action = Action::StartScan;
        }
    });
    if let Some(e) = &l.error {
        ui.add_space(8.0);
        ui.colored_label(ERROR, e);
    }
    if let Some(scan) = &l.scan {
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label("Scanning…");
            if theme::button(ui, theme, "Cancel", true).clicked() {
                action = Action::CancelScan;
            }
        });
        let current = scan.current.lock().unwrap().display().to_string();
        ui.add(egui::Label::new(RichText::new(current).small().weak()).truncate());
    }
    action
}
