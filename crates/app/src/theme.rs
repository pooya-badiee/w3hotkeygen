//! WC3 Reforged look, built from the game's own web UI art (`webui/`) and
//! fonts, read from the user's install at runtime. Nothing is bundled.
//!
//! Slice values come from the game's CSS (`webui/gluemanager.js`).
//! Every widget also has a flat fallback, used before the install is known
//! or if an asset is missing.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver};
use std::thread;

use eframe::egui::{
    self, Align2, Color32, ColorImage, FontData, FontDefinitions, FontFamily, FontId, Mesh, Painter, Pos2, Rect,
    Response, Sense, Shape, Stroke, TextureFilter, TextureHandle, TextureOptions, TextureWrapMode, Ui, Vec2, pos2,
    vec2,
};
use w3hk_assets::{GameFiles, Install};

pub const GOLD: Color32 = Color32::from_rgb(0xff, 0xd4, 0x28);
const GOLD_HOVER: Color32 = Color32::from_rgb(0xff, 0xdd, 0x33);
const GOLD_PRESSED: Color32 = Color32::from_rgb(0xb5, 0x95, 0x13);
pub const TEXT: Color32 = Color32::from_rgb(0xe6, 0xdc, 0xc4);
pub const ERROR: Color32 = Color32::from_rgb(0xff, 0x6a, 0x4d);

/// Font family for headings and buttons (the game's "MasterFontBold").
fn bold() -> FontFamily {
    FontFamily::Name("wc3-bold".into())
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum Tex {
    Wood,
    MetalFrame,
    BtnNormal,
    BtnHover,
    BtnDown,
    BtnDisabled,
    BtnFrame,
    Close,
    CloseHover,
    ClosePressed,
    SmallBtn,
    SmallBtnHover,
    SmallBtnPressed,
    IconFrame,
    /// The hotkey menu's 4x3 command card, cells drawn in.
    CardGrid,
    /// The hotkey menu's 2x3 inventory, cells drawn in.
    CardItems,
    /// A command-card icon, by file name in `webui\abilityicons\hd\`.
    Icon(&'static str),
}

/// Icons of the basic commands, named like their `CustomKeys.txt` sections.
pub const COMMAND_ICONS: [&str; 7] =
    ["cmdattack", "cmdstop", "cmdmove", "cmdholdpos", "cmdpatrol", "cmdselectskill", "cmdrally"];

impl Tex {
    const ART: [Tex; 16] = [
        Tex::Wood,
        Tex::MetalFrame,
        Tex::BtnNormal,
        Tex::BtnHover,
        Tex::BtnDown,
        Tex::BtnDisabled,
        Tex::BtnFrame,
        Tex::Close,
        Tex::CloseHover,
        Tex::ClosePressed,
        Tex::SmallBtn,
        Tex::SmallBtnHover,
        Tex::SmallBtnPressed,
        Tex::IconFrame,
        Tex::CardGrid,
        Tex::CardItems,
    ];

    fn all() -> impl Iterator<Item = Tex> {
        Tex::ART.into_iter().chain(COMMAND_ICONS.map(Tex::Icon))
    }

    fn path(self) -> String {
        let path = match self {
            Tex::Wood => r"war3.w3mod:webui\baseframes\test_texture-wood.png",
            Tex::MetalFrame => r"war3.w3mod:webui\baseframes\war3_metal_frame_4k.png",
            Tex::BtnNormal => r"war3.w3mod:webui\widgets\buttons\wc3_btn_blue_normal_4k.png",
            Tex::BtnHover => r"war3.w3mod:webui\widgets\buttons\wc3_btn_blue_hover_4k.png",
            Tex::BtnDown => r"war3.w3mod:webui\widgets\buttons\wc3_btn_blue_down_4k.png",
            Tex::BtnDisabled => r"war3.w3mod:webui\widgets\buttons\wc3_btn_disabled_normal_4k.png",
            Tex::BtnFrame => r"war3.w3mod:webui\widgets\buttons\button_frame_4k.png",
            Tex::Close => r"war3.w3mod:webui\icons\close-icon-resting.png",
            Tex::CloseHover => r"war3.w3mod:webui\icons\close-icon-hover.png",
            Tex::ClosePressed => r"war3.w3mod:webui\icons\close-icon-pressed.png",
            Tex::SmallBtn => r"war3.w3mod:webui\widgets\buttons\war3_btn_small_blue_normal_4k.png",
            Tex::SmallBtnHover => r"war3.w3mod:webui\widgets\buttons\war3_btn_small_blue_hover_4k.png",
            Tex::SmallBtnPressed => r"war3.w3mod:webui\widgets\buttons\war3_btn_small_blue_pressed_4k.png",
            Tex::IconFrame => r"war3.w3mod:webui\widgets\playerwidget\portrait_frame_normal_4k.png",
            Tex::CardGrid => r"war3.w3mod:webui\baseframes\wc3-hotkeys-menu-container.png",
            Tex::CardItems => r"war3.w3mod:webui\baseframes\wc3-hotkeys-menu-items-container.png",
            Tex::Icon(name) => return format!(r"war3.w3mod:webui\abilityicons\hd\{name}.png"),
        };
        path.into()
    }

    fn options(self) -> TextureOptions {
        let wrap_mode = if self == Tex::Wood { TextureWrapMode::Repeat } else { TextureWrapMode::ClampToEdge };
        // Mipmaps: the 4k art is drawn far smaller than its native size.
        TextureOptions { wrap_mode, mipmap_mode: Some(TextureFilter::Linear), ..TextureOptions::LINEAR }
    }
}

const FONT_BOLD: &str = r"war3.w3mod:fonts\blq85web.ttf";
const FONT_BODY: &str = r"war3.w3mod:fonts\frizqt__.ttf";

struct Texture {
    handle: TextureHandle,
    size: Vec2,
}

pub struct Theme {
    tex: HashMap<Tex, Texture>,
}

/// Decoded on a worker thread, uploaded on the UI thread.
struct Decoded {
    images: Vec<(Tex, ColorImage)>,
    bold: Option<Vec<u8>>,
    body: Option<Vec<u8>>,
}

pub struct ThemeLoader {
    rx: Receiver<Result<Decoded, String>>,
}

impl ThemeLoader {
    pub fn spawn(ctx: &egui::Context, install: Install) -> Self {
        let (tx, rx) = mpsc::channel();
        let ctx = ctx.clone();
        thread::spawn(move || {
            let _ = tx.send(decode(&install));
            ctx.request_repaint();
        });
        ThemeLoader { rx }
    }

    /// `Some` once loading finished. Installs fonts as a side effect.
    pub fn poll(&self, ctx: &egui::Context) -> Option<Result<Theme, String>> {
        let decoded = match self.rx.try_recv().ok()? {
            Ok(d) => d,
            Err(e) => return Some(Err(e)),
        };
        install_fonts(ctx, decoded.bold, decoded.body);
        let tex = decoded
            .images
            .into_iter()
            .map(|(t, img)| {
                let size = vec2(img.size[0] as f32, img.size[1] as f32);
                let handle = ctx.load_texture(format!("{t:?}"), img, t.options());
                (t, Texture { handle, size })
            })
            .collect();
        Some(Ok(Theme { tex }))
    }
}

fn decode(install: &Install) -> Result<Decoded, String> {
    let files = GameFiles::open(install).map_err(|e| e.to_string())?;
    let images = Tex::all()
        .filter_map(|t| {
            let img = image::load_from_memory(&files.read(&t.path())?).ok()?.to_rgba8();
            let size = [img.width() as usize, img.height() as usize];
            Some((t, ColorImage::from_rgba_unmultiplied(size, &img)))
        })
        .collect();
    Ok(Decoded { images, bold: files.read(FONT_BOLD), body: files.read(FONT_BODY) })
}

/// Sets up font families. Called at startup with no game fonts (so the
/// `wc3-bold` family exists), and again once they're loaded.
pub fn install_fonts(ctx: &egui::Context, bold_font: Option<Vec<u8>>, body_font: Option<Vec<u8>>) {
    let mut fonts = FontDefinitions::default();
    let defaults = fonts.families[&FontFamily::Proportional].clone();
    let mut bold_family = defaults.clone();
    if let Some(bytes) = bold_font {
        fonts.font_data.insert("wc3-bold".into(), Arc::new(FontData::from_owned(bytes)));
        bold_family.insert(0, "wc3-bold".into());
    }
    if let Some(bytes) = body_font {
        fonts.font_data.insert("wc3-body".into(), Arc::new(FontData::from_owned(bytes)));
        fonts.families.get_mut(&FontFamily::Proportional).unwrap().insert(0, "wc3-body".into());
    }
    fonts.families.insert(bold(), bold_family);
    ctx.set_fonts(fonts);
}

/// Dark, gold-accented egui visuals so stock widgets (text edits, labels)
/// blend with the textured ones.
pub fn apply_visuals(ctx: &egui::Context) {
    let mut v = egui::Visuals::dark();
    v.override_text_color = Some(TEXT);
    v.extreme_bg_color = Color32::from_rgb(0x0c, 0x0a, 0x08);
    v.panel_fill = Color32::from_rgb(0x12, 0x10, 0x0e);
    v.selection.bg_fill = Color32::from_rgb(0x6b, 0x55, 0x10);
    v.selection.stroke = Stroke::new(1.0, GOLD);
    v.widgets.inactive.bg_stroke = Stroke::new(1.0, Color32::from_rgb(0x4a, 0x42, 0x38));
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, Color32::from_rgb(0x8a, 0x76, 0x40));
    v.text_cursor.stroke = Stroke::new(2.0, GOLD);
    ctx.set_visuals(v);
    ctx.global_style_mut(|s| {
        s.text_styles.insert(egui::TextStyle::Body, FontId::proportional(15.0));
        s.text_styles.insert(egui::TextStyle::Button, FontId::proportional(15.0));
        s.text_styles.insert(egui::TextStyle::Small, FontId::proportional(12.0));
    });
}

// ---------------------------------------------------------------------------
// Painting primitives

/// CSS `border-image` equivalent. `slice` is in texels and `border` in points,
/// both as [left, top, right, bottom].
fn nine_slice(p: &Painter, rect: Rect, t: &Texture, slice: [f32; 4], border: [f32; 4], fill: bool) {
    nine_slice_part(p, rect, t, Rect::from_min_size(Pos2::ZERO, t.size), slice, border, fill);
}

/// [`nine_slice`] of the `src` part of the texture (in texels).
fn nine_slice_part(p: &Painter, rect: Rect, t: &Texture, src: Rect, slice: [f32; 4], border: [f32; 4], fill: bool) {
    // Shrink borders proportionally when the rect is too small for them.
    let sx = (rect.width() / (border[0] + border[2])).min(1.0);
    let sy = (rect.height() / (border[1] + border[3])).min(1.0);
    let xs = [rect.left(), rect.left() + border[0] * sx, rect.right() - border[2] * sx, rect.right()];
    let ys = [rect.top(), rect.top() + border[1] * sy, rect.bottom() - border[3] * sy, rect.bottom()];
    let us = [src.left(), src.left() + slice[0], src.right() - slice[2], src.right()].map(|x| x / t.size.x);
    let vs = [src.top(), src.top() + slice[1], src.bottom() - slice[3], src.bottom()].map(|y| y / t.size.y);
    let mut mesh = Mesh::with_texture(t.handle.id());
    for row in 0..3 {
        for col in 0..3 {
            if row == 1 && col == 1 && !fill {
                continue;
            }
            let r = Rect::from_min_max(pos2(xs[col], ys[row]), pos2(xs[col + 1], ys[row + 1]));
            let uv = Rect::from_min_max(pos2(us[col], vs[row]), pos2(us[col + 1], vs[row + 1]));
            mesh.add_rect_with_uv(r, uv, Color32::WHITE);
        }
    }
    p.add(Shape::mesh(mesh));
}

/// Repeats a texture across `rect` at `scale` points per texel.
fn tiled(p: &Painter, rect: Rect, t: &Texture, scale: f32) {
    let uv = Rect::from_min_size(Pos2::ZERO, rect.size() / (t.size * scale));
    p.image(t.handle.id(), rect, uv, Color32::WHITE);
}

fn shadow_text(p: &Painter, pos: Pos2, anchor: Align2, text: &str, font: FontId, color: Color32) {
    p.text(pos + vec2(0.0, 2.0), anchor, text, font.clone(), Color32::from_black_alpha(220));
    p.text(pos, anchor, text, font, color);
}

// ---------------------------------------------------------------------------
// Widgets. `theme` is `None` until game art is loaded.

/// Wooden board in a riveted metal frame, like the main menu panels.
/// Returns the content rect.
pub fn board(theme: Option<&Theme>, p: &Painter, rect: Rect) -> Rect {
    match theme.and_then(|t| Some((t.tex.get(&Tex::Wood)?, t.tex.get(&Tex::MetalFrame)?))) {
        Some((wood, frame)) => {
            tiled(p, rect.shrink(10.0), wood, 0.6);
            // CSS: border-image 600 ... border-image-width 300px (at 4k); 1/3 for us.
            nine_slice(p, rect, frame, [600.0; 4], [200.0; 4], false);
        }
        None => {
            p.rect(rect, 6.0, Color32::from_rgb(0x2a, 0x1e, 0x16), Stroke::new(3.0, Color32::from_gray(90)), egui::StrokeKind::Inside);
        }
    }
    rect.shrink(44.0)
}

/// Window control drawn in game art: `textures` are [rest, hover, pressed].
fn window_control(ui: &mut Ui, theme: Option<&Theme>, rect: Rect, textures: [Tex; 3], glyph: &str) -> Response {
    let resp = ui.interact(rect, ui.id().with(glyph), Sense::click());
    let state = if resp.is_pointer_button_down_on() { 2 } else if resp.hovered() { 1 } else { 0 };
    let p = ui.painter();
    match theme.and_then(|t| t.tex.get(&textures[state])) {
        Some(t) => {
            p.image(t.handle.id(), rect, Rect::from_min_max(Pos2::ZERO, pos2(1.0, 1.0)), Color32::WHITE);
        }
        None => {
            let color = if state == 0 { GOLD } else { GOLD_HOVER };
            p.text(rect.center(), Align2::CENTER_CENTER, glyph, FontId::proportional(20.0), color);
        }
    }
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Close (gold X) and minimize buttons in the board's top-right corner.
/// Returns (close, minimize) responses.
pub fn window_controls(ui: &mut Ui, theme: Option<&Theme>, board: Rect) -> (Response, Response) {
    let close_rect = Rect::from_min_size(pos2(board.right() - 70.0, board.top() + 36.0), vec2(28.0, 28.0));
    let min_rect = Rect::from_min_size(close_rect.min - vec2(40.0, -1.0), vec2(30.0, 26.0));
    let close = window_control(ui, theme, close_rect, [Tex::Close, Tex::CloseHover, Tex::ClosePressed], "✕");
    let minimize =
        window_control(ui, theme, min_rect, [Tex::SmallBtn, Tex::SmallBtnHover, Tex::SmallBtnPressed], "–");
    if theme.is_some_and(|t| t.tex.contains_key(&Tex::SmallBtn)) {
        // Gold bar glyph on the blue button.
        let bar = Rect::from_center_size(min_rect.center() + vec2(0.0, 3.0), vec2(12.0, 3.0));
        ui.painter().rect_filled(bar.translate(vec2(0.0, 1.0)), 1.0, Color32::BLACK);
        ui.painter().rect_filled(bar, 1.0, GOLD);
    }
    (close, minimize)
}

pub fn heading(ui: &mut Ui, text: &str, size: f32) {
    let font = FontId::new(size, bold());
    let galley_w = ui.fonts_mut(|f| f.layout_no_wrap(text.to_uppercase(), font.clone(), GOLD).size().x);
    let (rect, _) = ui.allocate_exact_size(vec2(galley_w.max(ui.available_width()), size * 1.3), Sense::hover());
    shadow_text(ui.painter(), rect.center(), Align2::CENTER_CENTER, &text.to_uppercase(), font, GOLD);
}

/// The blue Reforged menu button in its dark frame.
pub fn button(ui: &mut Ui, theme: Option<&Theme>, text: &str, enabled: bool) -> Response {
    let label = text.to_uppercase();
    let font = FontId::new(15.0, bold());
    let text_w = ui.fonts_mut(|f| f.layout_no_wrap(label.clone(), font.clone(), GOLD).size().x);
    let size = vec2((text_w + 56.0).max(150.0), 46.0);
    let sense = if enabled { Sense::click() } else { Sense::hover() };
    let (rect, resp) = ui.allocate_exact_size(size, sense);
    if !ui.is_rect_visible(rect) {
        return resp;
    }
    let pressed = enabled && resp.is_pointer_button_down_on();
    let hovered = enabled && resp.hovered();
    let p = ui.painter();
    let face = rect.shrink(5.0);
    let tex = |t| theme.and_then(|th| th.tex.get(&t));
    let state = match (enabled, pressed, hovered) {
        (false, ..) => Tex::BtnDisabled,
        (_, true, _) => Tex::BtnDown,
        (_, _, true) => Tex::BtnHover,
        _ => Tex::BtnNormal,
    };
    match (tex(Tex::BtnFrame), tex(state)) {
        (Some(frame), Some(btn)) => {
            // CSS: frame 60 / 30px; button 50 110 / 25px 55px (halved for our scale).
            nine_slice(p, rect, frame, [60.0; 4], [12.0; 4], true);
            nine_slice(p, face, btn, [110.0, 50.0, 110.0, 50.0], [26.0, 12.0, 26.0, 12.0], true);
        }
        _ => {
            p.rect_filled(rect, 4.0, Color32::from_rgb(0x14, 0x12, 0x10));
            let fill = match state {
                Tex::BtnDisabled => Color32::from_gray(50),
                Tex::BtnDown => Color32::from_rgb(0x12, 0x2a, 0x5c),
                Tex::BtnHover => Color32::from_rgb(0x24, 0x4c, 0x9c),
                _ => Color32::from_rgb(0x1a, 0x3a, 0x80),
            };
            p.rect(face, 3.0, fill, Stroke::new(1.0, Color32::from_rgb(0x5a, 0x8a, 0xe0)), egui::StrokeKind::Inside);
        }
    }
    let color = match state {
        Tex::BtnDisabled => Color32::from_gray(130),
        Tex::BtnDown => GOLD_PRESSED,
        Tex::BtnHover => GOLD_HOVER,
        _ => GOLD,
    };
    let offset = if pressed { vec2(0.0, 2.0) } else { Vec2::ZERO };
    shadow_text(p, face.center() + offset, Align2::CENTER_CENTER, &label, font, color);
    if enabled { resp.on_hover_cursor(egui::CursorIcon::PointingHand) } else { resp }
}

/// A panel of key cells like the game's hotkey menu: a title bar over a
/// grid of framed cells, all one image. The command card is 4x3 and the
/// inventory 2x3.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Card {
    Grid,
    Items,
}

/// One card's layout in CSS px (`.HotkeysOptions-Details-Content` and
/// `-Inventory-Content`): image size, padding [left, top] and columns.
struct CardSpec {
    tex: Tex,
    size: Vec2,
    pad: Vec2,
    cols: usize,
}

const CELL: f32 = 62.0;
/// CSS `grid-gap: 3px 2px`.
const CELL_GAP: Vec2 = vec2(2.0, 3.0);

impl Card {
    fn spec(self) -> CardSpec {
        match self {
            Card::Grid => CardSpec { tex: Tex::CardGrid, size: vec2(266.0, 233.0), pad: vec2(7.0, 35.0), cols: 4 },
            Card::Items => CardSpec { tex: Tex::CardItems, size: vec2(138.0, 235.0), pad: vec2(6.0, 36.0), cols: 2 },
        }
    }

    /// Size in points, at `scale` points per CSS px.
    pub fn size(self, scale: f32) -> Vec2 {
        self.spec().size * scale
    }
}

/// Draws the card at `min` with `title` in its top bar. Returns the cell
/// rects, left to right, top to bottom.
pub fn card(theme: Option<&Theme>, p: &Painter, kind: Card, min: Pos2, scale: f32, title: &str) -> Vec<Rect> {
    let s = kind.spec();
    let rect = Rect::from_min_size(min, kind.size(scale));
    let cells: Vec<Rect> = (0..3 * s.cols)
        .map(|i| {
            let at = s.pad + vec2((i % s.cols) as f32, (i / s.cols) as f32) * (vec2(CELL, CELL) + CELL_GAP);
            Rect::from_min_size(min + at * scale, vec2(CELL, CELL) * scale)
        })
        .collect();
    match theme.and_then(|t| t.tex.get(&s.tex)) {
        Some(t) => {
            p.image(t.handle.id(), rect, Rect::from_min_max(Pos2::ZERO, pos2(1.0, 1.0)), Color32::WHITE);
        }
        None => {
            let frame = Color32::from_gray(90);
            p.rect(rect, 4.0, Color32::from_rgb(0x0e, 0x0c, 0x0a), Stroke::new(1.5, frame), egui::StrokeKind::Inside);
            let bar = Rect::from_min_max(rect.min + vec2(5.0, 5.0) * scale, pos2(rect.right() - 5.0 * scale, cells[0].top() - 4.0 * scale));
            p.rect(bar, 3.0, Color32::from_gray(24), Stroke::new(1.0, frame), egui::StrokeKind::Inside);
            for &c in &cells {
                p.rect(c, 2.0, Color32::BLACK, Stroke::new(1.5, frame), egui::StrokeKind::Inside);
            }
        }
    }
    // CSS: 21px small caps, gold, centred in the bar.
    let bar_center = pos2(rect.center().x, min.y + (s.pad.y - 1.0) * scale / 2.0 + 1.0);
    shadow_text(p, bar_center, Align2::CENTER_CENTER, title, FontId::proportional(18.0), GOLD);
    cells
}

/// A card's frame and title bar with no cells, for free-form content.
/// Returns the body inside the frame.
pub fn card_panel(theme: Option<&Theme>, p: &Painter, rect: Rect, scale: f32, title: &str) -> Rect {
    let s = Card::Grid.spec();
    match theme.and_then(|t| t.tex.get(&s.tex)) {
        Some(t) => {
            // In CSS px: the title bar (rounded ends) above the cells, then
            // just the outer frame around a black body.
            let texel = t.size / s.size;
            let bar_h = s.pad.y - 2.0;
            let (bar, body) = rect.split_top_bottom_at_y(rect.top() + bar_h * scale);
            p.rect_filled(body.shrink2(vec2(4.0, 0.0) * scale).with_max_y(body.bottom() - 4.0 * scale), 0.0, Color32::BLACK);
            let part = |y0: f32, y1: f32| Rect::from_min_max(pos2(0.0, y0 * texel.y), pos2(t.size.x, y1 * texel.y));
            let slice = |b: [f32; 4]| [b[0] * texel.x, b[1] * texel.y, b[2] * texel.x, b[3] * texel.y];
            let bar_b = [12.0, 0.0, 12.0, 0.0];
            nine_slice_part(p, bar, t, part(0.0, bar_h), slice(bar_b), bar_b.map(|b| b * scale), true);
            let body_b = [5.0, 0.0, 5.0, 5.0];
            nine_slice_part(p, body, t, part(bar_h, s.size.y), slice(body_b), body_b.map(|b| b * scale), false);
        }
        None => {
            let frame = Color32::from_gray(90);
            p.rect(rect, 4.0, Color32::from_rgb(0x0e, 0x0c, 0x0a), Stroke::new(1.5, frame), egui::StrokeKind::Inside);
        }
    }
    let bar_center = pos2(rect.center().x, rect.top() + (s.pad.y - 1.0) * scale / 2.0 + 1.0);
    shadow_text(p, bar_center, Align2::CENTER_CENTER, title, FontId::proportional(18.0), GOLD);
    Rect::from_min_max(rect.min + vec2(5.0, s.pad.y - 2.0) * scale, rect.max - vec2(5.0, 5.0) * scale)
}

/// How a key cell is drawn.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum KeyState {
    Normal,
    /// Waiting for the user to press a key.
    Listening,
    Clash,
}

/// One key in a card cell, large and centred. Click to rebind.
pub fn key_cell(ui: &mut Ui, rect: Rect, id: egui::Id, key: &str, state: KeyState) -> Response {
    let resp = ui.interact(rect, id, Sense::click());
    let p = ui.painter();
    // The card's frame bevel takes ~8% of the cell on each side.
    let inner = rect.shrink(rect.width() * 0.08);
    if resp.hovered() {
        p.rect_filled(inner, 0.0, Color32::from_white_alpha(14));
    }
    let (label, color, border) = match state {
        KeyState::Listening => ("…", GOLD, Some(GOLD)),
        KeyState::Clash => (key, ERROR, Some(ERROR)),
        KeyState::Normal => (key, Color32::WHITE, None),
    };
    let size = if label.chars().count() > 1 { 17.0 } else { 26.0 };
    shadow_text(p, inner.center(), Align2::CENTER_CENTER, label, FontId::new(size, bold()), color);
    // Like the game's `.RedBorder` for clashes; gold while listening.
    if let Some(c) = border {
        p.rect_stroke(inner, 1.0, Stroke::new(2.5, c), egui::StrokeKind::Inside);
    }
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// A command's icon from the game in a metal frame, like the game's hotkey
/// menu draws them, with `label` on hover. Falls back to the label as text
/// before the art loads.
pub fn command_icon(ui: &mut Ui, theme: Option<&Theme>, icon: &'static str, label: &str) -> Response {
    let Some(t) = theme.and_then(|t| t.tex.get(&Tex::Icon(icon))) else {
        return ui.add_sized([44.0, 32.0], egui::Label::new(label));
    };
    let (rect, resp) = ui.allocate_exact_size(vec2(36.0, 36.0), Sense::hover());
    let p = ui.painter();
    let full = Rect::from_min_max(Pos2::ZERO, pos2(1.0, 1.0));
    match theme.and_then(|t| t.tex.get(&Tex::IconFrame)) {
        Some(frame) => {
            // The art is off-centre: in its 218px image the opening is x 33-182,
            // y 28-173, and the metal spans y 2-208 (a thick bottom shadow).
            // Centre the metal on `rect`, and put a square icon a few texels
            // under the opening's edges.
            let scale = rect.width() / frame.size.x;
            let frame_rect = rect.translate(vec2(0.0, (109.0 - 105.0) * scale));
            let at = |x: f32, y: f32| frame_rect.min + vec2(x, y) * scale;
            p.image(t.handle.id(), Rect::from_min_max(at(30.0, 23.0), at(185.0, 178.0)), full, Color32::WHITE);
            p.image(frame.handle.id(), frame_rect, full, Color32::WHITE);
        }
        None => {
            p.image(t.handle.id(), rect, full, Color32::WHITE);
            p.rect_stroke(rect, 0.0, Stroke::new(1.0, Color32::BLACK), egui::StrokeKind::Inside);
        }
    }
    resp.on_hover_text(label)
}

/// A small square showing one hotkey, like a key cap. Click to rebind.
pub fn key_box(ui: &mut Ui, text: &str, state: KeyState) -> Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(40.0, 32.0), Sense::click());
    let p = ui.painter();
    let border = match state {
        KeyState::Listening => GOLD,
        KeyState::Clash => ERROR,
        KeyState::Normal if resp.hovered() => Color32::from_gray(150),
        KeyState::Normal => Color32::from_gray(85),
    };
    p.rect(rect, 3.0, Color32::from_rgb(0x0b, 0x09, 0x08), Stroke::new(1.5, border), egui::StrokeKind::Inside);
    let (label, color) = match state {
        KeyState::Listening => ("…", GOLD),
        KeyState::Clash => (text, ERROR),
        KeyState::Normal => (text, TEXT),
    };
    let size = if label.len() > 1 { 13.0 } else { 17.0 };
    shadow_text(p, rect.center(), Align2::CENTER_CENTER, label, FontId::new(size, bold()), color);
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}
