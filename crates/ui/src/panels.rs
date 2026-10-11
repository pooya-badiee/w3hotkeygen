//! The parts of the screen both the exe and the web page show: the three
//! key cards, key capture, the status line, the footer and the credits.

use eframe::egui::{self, Color32, Rect, RichText, UiBuilder, vec2};
use w3hk_core::data::GameData;
use w3hk_core::{Key, Keys, Slot};

use crate::theme::{self, Card, ERROR, GOLD, KeyState, TEXT, Theme};

/// Seconds a new status message takes to fade from white to its colour.
const STATUS_FLASH: f32 = 0.8;

/// While a key box is listening: a letter sets it, a digit sets that
/// numpad key (the top-row digits are control groups), Delete or
/// Backspace clears it, Esc cancels. `true` once it stops listening.
pub fn capture_key(ctx: &egui::Context, keys: &mut Keys, slot: Slot) -> bool {
    let pressed = ctx.input(|i| {
        i.events.iter().find_map(|e| match e {
            egui::Event::Key { key, pressed: true, .. } => Some(*key),
            _ => None,
        })
    });
    let Some(key) = pressed else { return false };
    if key == egui::Key::Escape {
        return true;
    }
    if matches!(key, egui::Key::Delete | egui::Key::Backspace) {
        keys.set(slot, Key::NONE);
        return true;
    }
    if let [c] = key.name().as_bytes() {
        let key = match c {
            b'A'..=b'Z' | b'a'..=b'z' => Key::letter(*c as char),
            b'0'..=b'9' => Key::numpad(c - b'0'),
            _ => return false,
        };
        keys.set(slot, key);
        return true;
    }
    false
}

/// The keys in a `CustomKeys.txt`, with clashing commands and inventory keys
/// put back to their defaults, and a status line saying which ones changed.
pub fn read_keys(text: &str) -> (Keys, Option<String>) {
    let mut keys = Keys::read(text);
    let changed = keys.repair();
    if changed.is_empty() {
        return (keys, None);
    }
    let list: Vec<String> =
        changed.iter().map(|&(slot, old)| format!("{} {} (was {})", slot.label(), keys.get(slot).label(), old.label())).collect();
    (keys, Some(format!("Some of your file's keys clashed with the grid, so these are back to default: {}.", list.join(", "))))
}

/// Three cards side by side, like the game's hotkey menu: the grid, the
/// commands (icons and key boxes in the card's frame) and the inventory.
/// Returns the slot clicked, and the cards' outline so what's below can line
/// up with them.
pub fn cards(ui: &mut egui::Ui, theme: Option<&Theme>, keys: &Keys, listening: Option<Slot>) -> (Option<Slot>, Rect) {
    let mut clicked = None;
    let clashes = keys.clashes();
    let state = |s: Slot| match () {
        _ if listening == Some(s) => KeyState::Listening,
        _ if clashes.contains(&s) => KeyState::Clash,
        _ => KeyState::Normal,
    };
    let key_cell = |ui: &mut egui::Ui, rect: Rect, slot: Slot| {
        let id = ui.id().with(format!("key {slot:?}"));
        theme::key_cell(ui, rect, id, &keys.get(slot).label(), state(slot)).clicked()
    };

    let scale = 0.9;
    let gap = 16.0;
    // The commands panel is the size of the grid card.
    let [grid_size, cmd_size, items_size] = [Card::Grid, Card::Grid, Card::Items].map(|c| c.size(scale));
    let width = grid_size.x + cmd_size.x + items_size.x + 2.0 * gap;
    let area = Rect::from_min_size(ui.cursor().min, vec2(ui.available_width(), grid_size.y.max(items_size.y)));
    let left = area.left() + (area.width() - width) / 2.0;
    let panels = Rect::from_min_max(egui::pos2(left, area.top()), egui::pos2(left + width, area.bottom()));
    let p = ui.painter().clone();

    let grid = theme::card(theme, &p, Card::Grid, egui::pos2(left, area.top()), scale, "Grid");
    for (rect, slot) in grid.into_iter().zip(Slot::GRID) {
        if key_cell(ui, rect, slot) {
            clicked = Some(slot);
        }
    }

    let cmd_min = egui::pos2(left + grid_size.x + gap, area.top());
    let commands = theme::card_panel(theme, &p, Rect::from_min_size(cmd_min, cmd_size), scale, "Commands");
    // Centred in the body: four rows of two icon (36) + key (40) pairs.
    let commands = Rect::from_center_size(commands.center(), vec2(192.0, 4.0 * 36.0 + 3.0 * 4.0));
    ui.scope_builder(UiBuilder::new().max_rect(commands), |ui| {
        ui.spacing_mut().item_spacing.y = 4.0;
        let commands: Vec<_> = Slot::COMMANDS.into_iter().zip(theme::COMMAND_ICONS).collect();
        for row in commands.chunks(2) {
            ui.horizontal(|ui| {
                for (i, &(slot, icon)) in row.iter().enumerate() {
                    if i > 0 {
                        ui.add_space(8.0);
                    }
                    theme::command_icon(ui, theme, icon, slot.label());
                    if theme::key_box(ui, &keys.get(slot).label(), state(slot)).clicked() {
                        clicked = Some(slot);
                    }
                }
            });
        }
    });

    let items_min = egui::pos2(cmd_min.x + cmd_size.x + gap, area.top());
    let items = theme::card(theme, &p, Card::Items, items_min, scale, "Inventory");
    for (rect, slot) in items.into_iter().zip(Slot::ITEMS) {
        if key_cell(ui, rect, slot) {
            clicked = Some(slot);
        }
    }
    ui.advance_cursor_after_rect(area);
    ui.add_space(6.0);
    let note = "Abilities take the key of their button's cell. Commands keep theirs.";
    ui.vertical_centered(|ui| ui.label(RichText::new(note).size(13.0).color(Color32::from_gray(160))));
    (clicked, panels)
}

/// The status line when there's no fresh result: loading, errors, clashes.
pub fn idle_status(data: Option<&Result<GameData, String>>, keys: &Keys) -> (String, Color32) {
    match data {
        None => ("Reading unit data…".into(), Color32::from_gray(160)),
        Some(Err(e)) => (format!("Couldn't read unit data: {e}"), ERROR),
        _ if !keys.clashes().is_empty() => ("Red keys are used twice, or by an ability on the grid.".into(), ERROR),
        _ => ("Click a key to change it.".into(), Color32::from_gray(160)),
    }
}

/// The line under the buttons. A fresh result (set at `status_at`, egui
/// time) flashes white and fades to its colour, so repeat clicks show.
pub fn status_line(
    ui: &mut egui::Ui,
    status: Option<&Result<String, String>>,
    status_at: f64,
    listening: bool,
    idle: (String, Color32),
) {
    let age = (ui.input(|i| i.time) - status_at) as f32;
    let flash = |color: Color32| {
        if age < STATUS_FLASH {
            ui.ctx().request_repaint();
        }
        let t = (age / STATUS_FLASH).clamp(0.0, 1.0);
        Color32::WHITE.lerp_to_gamma(color, t * t)
    };
    let (text, color) = match status {
        _ if listening => ("Press a letter or a number (numpad), Delete for no key, or Esc to cancel.".into(), GOLD),
        Some(Ok(msg)) => (msg.clone(), flash(TEXT)),
        Some(Err(msg)) => (msg.clone(), flash(ERROR)),
        None => idle,
    };
    ui.vertical_centered(|ui| ui.add(egui::Label::new(RichText::new(text).size(13.0).color(color)).wrap()));
}

/// The disclaimer line under `content`, with the Credits link. `true` when
/// Credits was clicked.
pub fn footer(ui: &mut egui::Ui, content: Rect, text: &str) -> bool {
    let mut credits = false;
    let foot = Rect::from_min_max(content.left_bottom() + vec2(0.0, -4.0), content.right_bottom() + vec2(0.0, 12.0));
    ui.scope_builder(UiBuilder::new().max_rect(foot), |ui| {
        centered_row(ui, "footer", |ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            ui.label(RichText::new(text).size(12.0).color(Color32::from_gray(150)));
            let color = ui.visuals().hyperlink_color;
            credits = ui.link(RichText::new("Credits").size(12.0).color(color)).clicked();
        });
    });
    credits
}

pub const DISCLAIMER: &str = "Unofficial fan tool. Not affiliated with or endorsed by Blizzard Entertainment.";

/// Who made what this app is built from, except the "Also" row, which
/// differs between the exe and the web page.
const CREDITS: [(&str, &str); 5] = [
    (
        "Game art",
        "Icons, art and fonts belong to Blizzard Entertainment. They're read from your Warcraft III install; none are included.",
    ),
    ("Game files", "Our own CASC reader, written with CascLib by Ladislav Zezula (MIT) as the reference."),
    ("Interface", "egui and eframe by Emil Ernerfelt and contributors (MIT or Apache-2.0)."),
    (
        "Fonts",
        "Before the game's fonts load: Ubuntu Light (Ubuntu Font Licence), Hack (MIT), Noto Emoji (OFL) and emoji-icon-font (MIT).",
    ),
    (
        "Thanks",
        "The Warcraft III hotkey community: clemenscodes' warcraft-hotkey-editor, tlo9's Warcraft-3-Keybinds and the jcfieldsdev hotkey editor.",
    ),
];

/// Shown in place of the screen. `also` lists the other crates. `true` when
/// Back was clicked.
pub fn credits(ui: &mut egui::Ui, theme: Option<&Theme>, also: &str) -> bool {
    subheading(ui, "Credits");
    ui.add_space(6.0);
    let mut rows: Vec<(&str, &str)> = CREDITS.to_vec();
    rows.insert(4, ("Also", also));
    egui::Grid::new("credits").num_columns(2).spacing(vec2(16.0, 10.0)).show(ui, |ui| {
        for (what, who) in rows {
            ui.label(RichText::new(what).size(14.0).color(GOLD));
            ui.add(egui::Label::new(RichText::new(who).size(14.0).color(TEXT)).wrap());
            ui.end_row();
        }
    });
    ui.add_space(10.0);
    let license = format!("WC3 Grid Hotkeys {}, licensed MIT or Apache-2.0.", env!("CARGO_PKG_VERSION"));
    ui.label(RichText::new(license).size(13.0).color(Color32::from_gray(160)));
    ui.add_space(12.0);
    theme::button(ui, theme, "Back", true).clicked()
}

pub fn subheading(ui: &mut egui::Ui, text: &str) {
    ui.label(RichText::new(text).size(18.0).color(GOLD));
}

pub fn small(ui: &mut egui::Ui, text: String) -> egui::Response {
    ui.add(egui::Label::new(RichText::new(text).size(13.0).color(Color32::from_gray(160))).truncate())
}

/// Sized text loses the link colour to the text override, so it's set here.
pub fn small_link(ui: &mut egui::Ui, text: &str) -> egui::Response {
    let color = ui.visuals().hyperlink_color;
    ui.link(RichText::new(text).size(13.0).color(color))
}

/// A horizontal row centred in the available width. egui lays rows out
/// left to right, so this centres on the width measured last frame.
pub fn centered_row(ui: &mut egui::Ui, id: &str, add: impl FnOnce(&mut egui::Ui)) {
    let id = ui.id().with(id);
    let width: f32 = ui.data(|d| d.get_temp(id)).unwrap_or(0.0);
    ui.horizontal(|ui| {
        let pad = ((ui.available_width() - width) / 2.0).max(0.0);
        ui.add_space(pad);
        let start = ui.cursor().left();
        add(ui);
        let used = ui.min_rect().right() - start;
        if (used - width).abs() > 0.5 {
            ui.data_mut(|d| d.insert_temp(id, used));
            ui.ctx().request_repaint();
        }
    });
}
