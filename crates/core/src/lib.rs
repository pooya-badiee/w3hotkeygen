//! Generates `CustomKeys.txt`. Pure: no IO, so it can also target WASM.
//!
//! Output matches what the game writes itself: UTF-8 without BOM, LF line
//! endings, a blank line after each section.
//!
//! Game data (units and their command cards) is in [`data`], parsed from
//! file contents the caller reads.

pub mod data;
pub mod grid;
pub mod ini;
pub mod slk;

use std::collections::BTreeMap;
use std::fmt::{self, Write};

use data::Pos;

/// A key as the game stores it: a Windows virtual-key code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Key(pub u16);

impl Key {
    /// No key: the button gets none. Written as `Hotkey=0`, so the game
    /// doesn't fall back to its default.
    pub const NONE: Key = Key(0);

    /// `c` must be an ASCII letter; its uppercase form is its key code.
    pub const fn letter(c: char) -> Key {
        Key(c.to_ascii_uppercase() as u16)
    }

    /// Numpad digit `n` (0-9).
    pub const fn numpad(n: u8) -> Key {
        Key(96 + n as u16)
    }

    /// Parses a `Hotkey=` value: a letter or a key code. Multi-level values
    /// (`A,A,A`) use the first entry.
    pub fn parse(value: &str) -> Option<Key> {
        let first = value.split(',').next()?.trim();
        match first.as_bytes() {
            [c] if c.is_ascii_alphabetic() => Some(Key::letter(*c as char)),
            _ => first.parse().ok().map(Key),
        }
    }

    /// Short name for the UI, e.g. `T` or `Num7`; `-` for no key.
    pub fn label(self) -> String {
        match self.0 {
            0 => "-".into(),
            96..=105 => format!("Num{}", self.0 - 96),
            _ => self.to_string(),
        }
    }
}

/// Letters are written as letters (`Hotkey=T`), everything else as its code
/// (`Hotkey=103` for numpad 7), as the game does.
impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            65..=90 => f.write_char(self.0 as u8 as char),
            code => write!(f, "{code}"),
        }
    }
}

/// One user-editable key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Slot {
    Attack,
    Stop,
    Move,
    Hold,
    Patrol,
    /// A hero's Learn button.
    Learn,
    /// A building's Rally Point.
    Rally,
    /// Inventory slot 0-5, left to right, top to bottom.
    Item(usize),
    /// Command card cell 0-11, left to right, top to bottom: the key every
    /// ability placed there gets.
    Grid(usize),
}

impl Slot {
    pub const COMMANDS: [Slot; 7] =
        [Slot::Attack, Slot::Stop, Slot::Move, Slot::Hold, Slot::Patrol, Slot::Learn, Slot::Rally];
    pub const ITEMS: [Slot; 6] =
        [Slot::Item(0), Slot::Item(1), Slot::Item(2), Slot::Item(3), Slot::Item(4), Slot::Item(5)];
    pub const GRID: [Slot; 12] = {
        let mut grid = [Slot::Grid(0); 12];
        let mut i = 0;
        while i < 12 {
            grid[i] = Slot::Grid(i);
            i += 1;
        }
        grid
    };

    pub fn label(self) -> &'static str {
        match self {
            Slot::Attack => "Attack",
            Slot::Stop => "Stop",
            Slot::Move => "Move",
            Slot::Hold => "Hold",
            Slot::Patrol => "Patrol",
            Slot::Learn => "Learn",
            Slot::Rally => "Rally",
            Slot::Item(_) => "Item",
            Slot::Grid(_) => "Grid",
        }
    }

    /// The command card cell the button sits in, on every unit. Abilities
    /// fill the cells commands leave free.
    pub const fn cell(self) -> Option<Pos> {
        match self {
            Slot::Attack => Some(Pos::new(0, 1)),
            Slot::Stop => Some(Pos::new(1, 1)),
            Slot::Move => Some(Pos::new(2, 1)),
            Slot::Hold => Some(Pos::new(3, 1)),
            Slot::Patrol => Some(Pos::new(0, 2)),
            Slot::Learn | Slot::Rally => Some(Pos::new(3, 2)),
            Slot::Item(_) => None,
            Slot::Grid(i) => Some(Pos::new(i as u8 % Pos::COLS, i as u8 / Pos::COLS)),
        }
    }

    /// Whether the two keys may be the same: they never show on one card.
    /// A command may use its own cell's grid key (Attack on A), which only
    /// buildings without that command use. Learn and Rally share the corner.
    fn apart(self, other: Slot) -> bool {
        let fixed = |s: Slot| !matches!(s, Slot::Item(_) | Slot::Grid(_));
        self == other || (self.cell() == other.cell() && (fixed(self) || fixed(other)))
    }

    /// Section name in `CustomKeys.txt`. Grid keys have none of their own.
    fn section(self) -> String {
        match self {
            Slot::Attack => "CmdAttack".into(),
            Slot::Stop => "CmdStop".into(),
            Slot::Move => "CmdMove".into(),
            Slot::Hold => "CmdHoldPos".into(),
            Slot::Patrol => "CmdPatrol".into(),
            Slot::Learn => "CmdSelectSkill".into(),
            Slot::Rally => "CmdRally".into(),
            Slot::Item(i) => format!("itm{}", i + 1),
            Slot::Grid(_) => unreachable!("grid keys have no section"),
        }
    }
}

/// For a `[section]` header line: the slot it belongs to, if any.
/// Section names are case-insensitive.
fn slot_for_header(line: &str) -> Option<Option<Slot>> {
    let name = line.trim().strip_prefix('[')?.strip_suffix(']')?;
    Some(Keys::file_slots().find(|s| s.section().eq_ignore_ascii_case(name)))
}

/// The value of a `Hotkey=` line. Field names are case-insensitive.
fn hotkey_value(line: &str) -> Option<&str> {
    let (field, value) = line.split_once('=')?;
    field.trim().eq_ignore_ascii_case("Hotkey").then_some(value)
}

/// The keys the user picks on the settings screen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Keys {
    /// In [`Slot::COMMANDS`] order.
    pub commands: [Key; 7],
    pub inventory: [Key; 6],
    /// In [`Slot::GRID`] order.
    pub grid: [Key; 12],
}

/// Default inventory keys: the numpad, laid out like the 2x3 inventory, as
/// in the game's own defaults. No letter can clash with them.
pub const INVENTORY_NUMPAD: [Key; 6] =
    [Key::numpad(7), Key::numpad(8), Key::numpad(4), Key::numpad(5), Key::numpad(1), Key::numpad(2)];

/// The grid on a QWERTY keyboard.
pub const GRID_QWERTY: [Key; 12] = {
    let letters = *b"QWERASDFZXCV";
    let mut grid = [Key(0); 12];
    let mut i = 0;
    while i < 12 {
        grid[i] = Key::letter(letters[i] as char);
        i += 1;
    }
    grid
};

/// The corner cell's letter, which Learn and Rally use by default.
const CORNER: Key = Key::letter('V');

/// Classic command keys, Learn and Rally on the corner cell, inventory on
/// the numpad.
impl Default for Keys {
    fn default() -> Self {
        Keys {
            commands: [Key::letter('A'), Key::letter('S'), Key::letter('M'), Key::letter('H'), Key::letter('P'), CORNER, CORNER],
            inventory: INVENTORY_NUMPAD,
            grid: GRID_QWERTY,
        }
    }
}

impl Keys {
    pub fn get(&self, slot: Slot) -> Key {
        match slot {
            Slot::Item(i) => self.inventory[i],
            Slot::Grid(i) => self.grid[i],
            cmd => self.commands[Slot::COMMANDS.iter().position(|&c| c == cmd).unwrap()],
        }
    }

    pub fn set(&mut self, slot: Slot, key: Key) {
        match slot {
            Slot::Item(i) => self.inventory[i] = key,
            Slot::Grid(i) => self.grid[i] = key,
            cmd => self.commands[Slot::COMMANDS.iter().position(|&c| c == cmd).unwrap()] = key,
        }
    }

    /// The slots with their own section in `CustomKeys.txt`.
    fn file_slots() -> impl Iterator<Item = Slot> {
        Slot::COMMANDS.into_iter().chain(Slot::ITEMS)
    }

    fn slots() -> impl Iterator<Item = Slot> {
        Self::file_slots().chain(Slot::GRID)
    }

    /// Slots whose key is also used by another slot that can show on the
    /// same card (see [`Slot::apart`]). No key never clashes.
    pub fn clashes(&self) -> Vec<Slot> {
        Self::slots()
            .filter(|&s| {
                let key = self.get(s);
                key != Key::NONE && Self::slots().any(|o| !s.apart(o) && self.get(o) == key)
            })
            .collect()
    }
}

impl Keys {
    /// The keys currently set in a `CustomKeys.txt`; defaults where it has none.
    pub fn read(text: &str) -> Keys {
        let mut keys = Keys::default();
        let mut current = None;
        for line in text.lines() {
            if let Some(slot) = slot_for_header(line) {
                current = slot;
            } else if let (Some(slot), Some(key)) = (current, hotkey_value(line).and_then(Key::parse)) {
                keys.set(slot, key);
            }
        }
        if let Some(grid) = read_grid(text) {
            keys.grid = grid;
        }
        keys
    }

    /// Puts clashing commands and inventory keys back to their defaults, so a
    /// file from another layout (Move on Z, say) opens ready to apply. The
    /// grid is kept. Returns each changed slot with its old key.
    pub fn repair(&mut self) -> Vec<(Slot, Key)> {
        let defaults = Keys::default();
        let mut changed = Vec::new();
        // A default can clash in turn (Hold back on H, an item on H), so repeat.
        for _ in 0..Self::file_slots().count() {
            let fix: Vec<Slot> = self
                .clashes()
                .into_iter()
                .filter(|&s| !matches!(s, Slot::Grid(_)) && self.get(s) != defaults.get(s))
                .collect();
            if fix.is_empty() {
                break;
            }
            for slot in fix {
                if !changed.iter().any(|&(s, _)| s == slot) {
                    changed.push((slot, self.get(slot)));
                }
                self.set(slot, defaults.get(slot));
            }
        }
        changed
    }
}

/// The grid a generated file was made with, from its buttons' cells and
/// keys. A cell's key is the one at least 3 in 4 of its buttons use; cells
/// without one get a QWERTY key. `None` if a key is on two cells, as in a
/// hand-made file. Cleared cells (no key) may be several.
fn read_grid(text: &str) -> Option<[Key; 12]> {
    let mut votes: [BTreeMap<u16, usize>; 12] = Default::default();
    let mut vote = |pos: Option<Pos>, key: Option<Key>| {
        if let (Some(pos), Some(key)) = (pos, key) {
            *votes[(pos.y * Pos::COLS + pos.x) as usize].entry(key.0).or_default() += 1;
        }
    };
    // Commands, items and Rally have the user's keys, not the grid's.
    let skip = |name: &str| {
        let name = name.to_ascii_lowercase();
        name.starts_with("cmd") || name.starts_with("itm") || name == "aral"
    };
    let (mut skipping, mut pos, mut key) = (true, None, None);
    for line in text.lines() {
        if let Some(name) = line.trim().strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            vote(pos.take(), key.take());
            skipping = skip(name);
        } else if let (false, Some((field, value))) = (skipping, line.split_once('=')) {
            match field.trim().to_ascii_lowercase().as_str() {
                "buttonpos" => pos = Pos::parse(value),
                "hotkey" => key = Key::parse(value),
                _ => {}
            }
        }
    }
    vote(pos, key);

    let mut grid: [Option<Key>; 12] = [None; 12];
    for (cell, votes) in votes.iter().enumerate() {
        let total: usize = votes.values().sum();
        if let Some((&key, &n)) = votes.iter().max_by_key(|(_, n)| **n)
            && n * 4 >= total * 3
        {
            grid[cell] = Some(Key(key));
        }
    }
    // Empty cells keep their QWERTY key, or take a QWERTY key no cell has.
    for i in 0..12 {
        if grid[i].is_none() && !grid.contains(&Some(GRID_QWERTY[i])) {
            grid[i] = Some(GRID_QWERTY[i]);
        }
    }
    for i in 0..12 {
        if grid[i].is_none() {
            grid[i] = GRID_QWERTY.into_iter().find(|k| !grid.contains(&Some(*k)));
        }
    }
    let grid = grid.map(|k| k.expect("12 QWERTY keys for 12 cells"));
    let distinct = grid.iter().enumerate().all(|(i, k)| *k == Key::NONE || !grid[..i].contains(k));
    distinct.then_some(grid)
}

fn render_section(out: &mut String, slot: Slot, keys: &Keys) {
    let _ = writeln!(out, "[{}]", slot.section());
    // `GameCommand=1,1,1` marks a system binding, as the game writes it.
    out.push_str("GameCommand=1,1,1\n");
    let _ = write!(out, "Hotkey={}\n\n", keys.get(slot));
}

/// Renders the inventory sections.
pub fn render_items(keys: &Keys) -> String {
    let mut out = String::new();
    for slot in Slot::ITEMS {
        render_section(&mut out, slot, keys);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_render_like_the_game() {
        assert_eq!(Key::letter('t').to_string(), "T");
        assert_eq!(Key::numpad(7).to_string(), "103");
        assert_eq!(Key::numpad(7).label(), "Num7");
        assert_eq!(Key(27).to_string(), "27");
    }

    #[test]
    fn renders_items_like_the_game() {
        let text = render_items(&Keys::default());
        assert!(text.starts_with("[itm1]\nGameCommand=1,1,1\nHotkey=103\n\n[itm2]"));
        assert!(text.ends_with("[itm6]\nGameCommand=1,1,1\nHotkey=98\n\n"));
        assert!(!text.contains('\r'));
    }

    #[test]
    fn parses_hotkey_values() {
        assert_eq!(Key::parse("t"), Some(Key::letter('T')));
        assert_eq!(Key::parse("A,A,A"), Some(Key::letter('A')));
        assert_eq!(Key::parse("103"), Some(Key::numpad(7)));
        assert_eq!(Key::parse(""), None);
    }

    const EXISTING: &str = "[a002]\nButtonpos=1,0\nHotkey=W\n\n\
                            [CmdAttack]\nButtonpos=0,1\nhotkey=A\nTip=(|cffffcc00A|r) Attack\n\n\
                            [cmdstop]\nButtonpos=1,1\n\n\
                            [itm1]\nGameCommand=1,1,1\nHotkey=T\n\n\
                            [jpnt]\nHotkey=J\n";

    #[test]
    fn reads_keys_from_file() {
        let keys = Keys::read(EXISTING);
        assert_eq!(keys.get(Slot::Item(0)), Key::letter('T'));
        // Not in the file: the numpad default.
        assert_eq!(keys.get(Slot::Item(1)), Key::numpad(8));
        assert_eq!(keys.get(Slot::Attack), Key::letter('A'));
        // Not in the file: default.
        assert_eq!(keys.get(Slot::Patrol), Key::letter('P'));
    }

    #[test]
    fn finds_clashes() {
        let mut keys = Keys::default();
        assert!(keys.clashes().is_empty());
        keys.set(Slot::Item(3), Key::letter('H'));
        assert_eq!(keys.clashes(), [Slot::Hold, Slot::Item(3)]);
        keys.set(Slot::Item(3), Key::letter('Q'));
        assert_eq!(keys.clashes(), [Slot::Item(3), Slot::Grid(0)]);
    }

    #[test]
    fn commands_may_use_their_own_cells_key() {
        let mut keys = Keys::default();
        keys.set(Slot::Grid(4), Key::letter('K'));
        assert!(keys.clashes().is_empty());
        keys.set(Slot::Move, Key::letter('K'));
        assert_eq!(keys.clashes(), [Slot::Move, Slot::Grid(4)]);
        keys.set(Slot::Move, Key::letter('M'));
        keys.set(Slot::Grid(6), Key::letter('K'));
        assert_eq!(keys.clashes(), [Slot::Grid(4), Slot::Grid(6)]);
    }

    #[test]
    fn reads_the_grid_from_a_generated_file() {
        let azerty = "[AHbz]\nButtonpos=0,0\nHotkey=A\n\n[Rhme]\nButtonpos=1,0\nHotkey=Z,Z,Z\n\n\
                      [hfoo]\nButtonpos=0,0\nHotkey=A\n\n[CmdAttack]\nButtonpos=0,1\nHotkey=A\n\n\
                      [Ahar]\nButtonpos=3,2\nHotkey=V\n\n[ARal]\nButtonpos=3,2\nHotkey=T\n\n\
                      [hkni]\nButtonpos=0,1\nHotkey=Q\n\n[Adef]\nButtonpos=0,2\nHotkey=W\n";
        let keys = Keys::read(azerty);
        assert_eq!(keys.get(Slot::Grid(0)), Key::letter('A'));
        assert_eq!(keys.get(Slot::Grid(1)), Key::letter('Z'));
        assert_eq!(keys.get(Slot::Grid(4)), Key::letter('Q'));
        assert_eq!(keys.get(Slot::Grid(11)), Key::letter('V'));
        // No buttons there: QWERTY.
        assert_eq!(keys.get(Slot::Grid(5)), Key::letter('S'));
        // Mixed keys on a cell, as in Blizzard's classic layout: QWERTY.
        let mixed = "[a]\nButtonpos=0,0\nHotkey=B\n[b]\nButtonpos=0,0\nHotkey=T\n";
        assert_eq!(Keys::read(mixed).grid, GRID_QWERTY);
        // A key on two cells: QWERTY.
        let twice = "[a]\nButtonpos=0,0\nHotkey=W\n[b]\nButtonpos=2,0\nHotkey=W\n";
        assert_eq!(Keys::read(twice).grid, GRID_QWERTY);
    }

    #[test]
    fn cleared_keys_never_clash() {
        let mut keys = Keys::default();
        keys.set(Slot::Move, Key::NONE);
        keys.set(Slot::Item(0), Key::NONE);
        keys.set(Slot::Grid(0), Key::NONE);
        keys.set(Slot::Grid(1), Key::NONE);
        assert!(keys.clashes().is_empty());
        assert_eq!(Key::NONE.to_string(), "0");
        assert_eq!(Key::parse("0"), Some(Key::NONE));
        let text = "[itm1]\nGameCommand=1,1,1\nHotkey=0\n\n\
                    [a]\nButtonpos=0,0\nHotkey=0\n[b]\nButtonpos=1,0\nHotkey=0\n[c]\nButtonpos=2,0\nHotkey=K\n";
        let read = Keys::read(text);
        assert_eq!(read.get(Slot::Item(0)), Key::NONE);
        assert_eq!(&read.grid[..3], [Key::NONE, Key::NONE, Key::letter('K')]);
    }

    #[test]
    fn repairs_another_layouts_commands() {
        // Commands from a common grid file: Move, Hold and Patrol on the bottom row.
        let text = "[CmdMove]\nHotkey=Z\n\n[CmdHoldPos]\nHotkey=X\n\n[CmdPatrol]\nHotkey=C\n\n\
                    [CmdSelectSkill]\nHotkey=F\n\n[CmdRally]\nHotkey=F\n\n[itm1]\nHotkey=H\n";
        let mut keys = Keys::read(text);
        assert!(!keys.clashes().is_empty());
        let changed = keys.repair();
        assert!(keys.clashes().is_empty());
        assert_eq!(keys, Keys::default());
        assert!(changed.contains(&(Slot::Move, Key::letter('Z'))));
        assert!(changed.contains(&(Slot::Item(0), Key::letter('H'))));
        // Nothing to repair: nothing changes.
        assert!(Keys::default().repair().is_empty());
    }

    #[test]
    fn learn_and_rally_own_the_corner() {
        let mut keys = Keys::default();
        keys.set(Slot::Learn, Key::letter('T'));
        keys.set(Slot::Rally, Key::letter('T'));
        assert!(keys.clashes().is_empty());
        keys.set(Slot::Rally, Key::letter('Q'));
        assert_eq!(keys.clashes(), [Slot::Rally, Slot::Grid(0)]);
        keys.set(Slot::Rally, Key::letter('T'));
        keys.set(Slot::Hold, Key::letter('T'));
        assert_eq!(keys.clashes(), [Slot::Hold, Slot::Learn, Slot::Rally]);
    }
}
