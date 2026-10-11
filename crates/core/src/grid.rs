//! The grid generator: a full `CustomKeys.txt` where every button's key is
//! its cell on the 4x3 card (`QWER / ASDF / ZXCV`), and the basic commands
//! keep the user's keys in fixed cells:
//!
//! ```text
//! Q       W     E     R
//! Attack  Stop  Move  Hold
//! Patrol  X     C     Learn (heroes), Rally (buildings)
//! ```
//!
//! Positions are global per rawcode (see docs/customkeys-format.md), so each
//! button gets one cell that's free on every melee card it shows on.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt::Write;

use crate::data::{Button, CardKind, Field, GameData, Pos};
use crate::{Key, Keys, Slot};

/// The user's key for a cell.
fn grid_key(keys: &Keys, pos: Pos) -> Key {
    keys.grid[(pos.y * Pos::COLS + pos.x) as usize]
}

/// The basic commands, on every unit, in their [`Slot::cell`].
const COMMANDS: [(&str, Slot); 5] = [
    ("CmdAttack", Slot::Attack),
    ("CmdStop", Slot::Stop),
    ("CmdMove", Slot::Move),
    ("CmdHoldPos", Slot::Hold),
    ("CmdPatrol", Slot::Patrol),
];

/// A hero's Learn button and a building's Rally Point, in the corner.
const CORNER: [(&str, Slot); 2] = [("CmdSelectSkill", Slot::Learn), ("ARal", Slot::Rally)];

fn cell(slot: Slot) -> Pos {
    slot.cell().expect("commands have a cell")
}

/// Left alone: Esc in the corner of every menu.
const CANCEL: (&str, Pos) = ("CmdCancel", Pos::new(3, 2));

/// Free cells are tried in this order: top row, bottom row, then the rest.
const SEARCH: [Pos; 12] = [
    Pos::new(0, 0),
    Pos::new(1, 0),
    Pos::new(2, 0),
    Pos::new(3, 0),
    Pos::new(0, 2),
    Pos::new(1, 2),
    Pos::new(2, 2),
    Pos::new(3, 2),
    Pos::new(2, 1),
    Pos::new(3, 1),
    Pos::new(0, 1),
    Pos::new(1, 1),
];

/// Toggles whose off-state key is tied to a helper ID, as in Blizzard's
/// template: `UnhotkeyId=Ami1` plus `[Ami1] ParentId=Amil`.
const UN_HELPERS: [(&str, &str); 3] = [("Amil", "Ami1"), ("Amic", "Ami2"), ("Astn", "Ast1")];

/// Campaign heroes the game's own conflict check covers: the ones in its
/// template, `war3.w3mod:customkeys.txt`. Left out, their default cells clash
/// with the grid's (Grim Conviction on the Learn cell).
const CAMPAIGN_HEROES: [&str; 8] = ["Hctl", "Hctr", "Hjsm", "Hleo", "Ugrk", "Ugr2", "Ugr3", "Uany"];

/// The game reads a worker's build button from `AHbu` etc. but its key from
/// `CmdBuild<Race>`, so both get written. Same for Rally.
const ALIASES: [(&str, &str); 7] = [
    ("AHbu", "CmdBuildHuman"),
    ("AObu", "CmdBuildOrc"),
    ("AUbu", "CmdBuildUndead"),
    ("AEbu", "CmdBuildNightElf"),
    ("AGbu", "CmdBuild"),
    ("ANbu", "CmdBuild"),
    ("ARal", "CmdRally"),
];

/// A button's on-state (with its hero learn button), or its off-state.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct Group {
    id: String,
    un: bool,
}

impl Group {
    fn of(id: &str, field: Field) -> Group {
        Group { id: id.to_string(), un: field == Field::Un }
    }
}

/// One card of one unit, for the placement checks.
struct Slots {
    unit: String,
    groups: Vec<Group>,
    /// Commands live here, so default positions are moved down a row.
    shifted: bool,
}

/// A generated `CustomKeys.txt`.
#[derive(Debug)]
pub struct Generated {
    pub text: String,
    /// Two buttons on one cell, or one key twice, on some card. Empty when
    /// the layout works.
    pub conflicts: Vec<String>,
}

pub fn generate(data: &GameData, keys: &Keys) -> Generated {
    let cards = collect_cards(data);
    let mut on_cards: HashMap<&Group, Vec<usize>> = HashMap::new();
    for (i, card) in cards.iter().enumerate() {
        for g in &card.groups {
            on_cards.entry(g).or_default().push(i);
        }
    }

    let mut placed: BTreeMap<Group, Pos> = BTreeMap::new();
    // Per card: the cells in use.
    let mut taken: Vec<HashSet<Pos>> = vec![HashSet::new(); cards.len()];
    let mut place = |g: &Group, pos: Pos, taken: &mut Vec<HashSet<Pos>>| {
        for &i in on_cards.get(g).into_iter().flatten() {
            taken[i].insert(pos);
        }
        placed.insert(g.clone(), pos);
    };

    // Fixed cells first.
    let fixed = COMMANDS.iter().chain(&CORNER).map(|&(id, slot)| (id, cell(slot))).chain([CANCEL]);
    for (id, pos) in fixed {
        place(&Group::of(id, Field::Button), pos, &mut taken);
    }

    // Then every other button in card order: units before buildings, so
    // the cards with the least room choose first. A button keeps its
    // default cell (moved down a row on units) if it's free on all its
    // cards, else takes the first cell that is. If none is, it keeps its
    // default and shows up in the conflicts.
    let mut order: Vec<usize> = (0..cards.len()).collect();
    order.sort_by_key(|&i| !cards[i].shifted);
    let mut done: HashSet<&Group> = HashSet::new();
    for i in order {
        for g in &cards[i].groups {
            if !done.insert(g) || placed_fixed(g) {
                continue;
            }
            let shifted = on_cards[g].iter().any(|&c| cards[c].shifted);
            let preferred = default_pos(data, g).map(|p| if shifted { shift(p) } else { p });
            let free = |pos: &Pos| on_cards[g].iter().all(|&c| !taken[c].contains(pos));
            if let Some(pos) = preferred.into_iter().chain(SEARCH).find(free).or(preferred) {
                place(g, pos, &mut taken);
            }
        }
    }

    let conflicts = find_conflicts(data, keys, &cards, &placed);
    Generated { text: render(data, keys, &placed), conflicts }
}

fn placed_fixed(g: &Group) -> bool {
    !g.un && (g.id == CANCEL.0 || COMMANDS.iter().chain(&CORNER).any(|c| c.0 == g.id))
}

/// Blizzard's cards have the commands on the top row, so on units the rows
/// move down one: abilities from the bottom row go to the top.
fn shift(p: Pos) -> Pos {
    Pos::new(p.x, (p.y + 1) % Pos::ROWS)
}

fn default_pos(data: &GameData, g: &Group) -> Option<Pos> {
    let b = data.button(&g.id)?;
    if g.un { b.unbuttonpos } else { b.buttonpos.or(b.researchbuttonpos) }
}

/// The cards of every melee unit and [`CAMPAIGN_HEROES`], as groups. Shop
/// wares get cells like any other button, including the ones Blizzard left
/// without a position (the Goblin Merchant's Boots of Speed).
fn collect_cards(data: &GameData) -> Vec<Slots> {
    let mut cards = Vec::new();
    let mut units = data.melee_units();
    let extra: Vec<_> = CAMPAIGN_HEROES.iter().filter(|&&id| !units.iter().any(|u| u.id == id)).collect();
    units.extend(extra.into_iter().filter_map(|id| data.unit(id)));
    for unit in units {
        let wares: Vec<&String> = unit.sells_units.iter().chain(&unit.sells_items).collect();
        for card in data.cards(unit) {
            let shifted = match card.kind {
                CardKind::Uprooted => true,
                CardKind::Main | CardKind::Learn => !unit.building,
                CardKind::Build => false,
            };
            let mut groups: Vec<Group> = card
                .buttons
                .iter()
                .flat_map(|b| {
                    // A toggle with a helper ID is two buttons to the game's
                    // conflict check, so its off-state needs its own cell
                    // too (Call to Arms on the Keep).
                    let helper = b.field == Field::Button && UN_HELPERS.iter().any(|h| h.0 == b.id);
                    [Some(Group::of(&b.id, b.field)), helper.then(|| Group::of(&b.id, Field::Un))]
                })
                .flatten()
                .collect();
            if card.kind == CardKind::Main {
                let unplaced = wares.iter().filter(|w| card.unplaced.contains(w) && data.button(w).is_some());
                groups.extend(unplaced.map(|w| Group::of(w, Field::Button)));
            }
            groups.dedup();
            cards.push(Slots { unit: unit.id.clone(), groups, shifted });
        }
    }
    cards
}

/// The key a placed group gets: the user's key for commands, Learn and
/// Rally, Esc for Cancel, the cell's grid letter for everything else.
fn key_for(keys: &Keys, g: &Group, pos: Pos) -> Key {
    if let Some(&(_, slot)) = COMMANDS.iter().chain(&CORNER).find(|c| c.0 == g.id) {
        return keys.get(slot);
    }
    if g.id == CANCEL.0 { Key(27) } else { grid_key(keys, pos) }
}

fn find_conflicts(data: &GameData, keys: &Keys, cards: &[Slots], placed: &BTreeMap<Group, Pos>) -> Vec<String> {
    let mut out = Vec::new();
    for card in cards {
        let mut cells: BTreeMap<Pos, Vec<&str>> = BTreeMap::new();
        let mut used: BTreeMap<u16, Vec<&str>> = BTreeMap::new();
        for g in &card.groups {
            let Some(&pos) = placed.get(g) else { continue };
            cells.entry(pos).or_default().push(&g.id);
            if has_key(data, g) && key_for(keys, g, pos) != Key::NONE {
                used.entry(key_for(keys, g, pos).0).or_default().push(&g.id);
            }
        }
        for (pos, ids) in cells.into_iter().filter(|(_, ids)| ids.len() > 1) {
            out.push(format!("{}: {} share cell {},{}", card.unit, ids.join(", "), pos.x, pos.y));
        }
        for (key, ids) in used.into_iter().filter(|(_, ids)| ids.len() > 1) {
            out.push(format!("{}: {} share key {}", card.unit, ids.join(", "), Key(key)));
        }
    }
    out.sort();
    out.dedup();
    out
}

/// Whether the button takes a key at all: passives don't.
fn has_key(data: &GameData, g: &Group) -> bool {
    if g.id.starts_with("Cmd") || ALIASES.iter().any(|a| a.0 == g.id) {
        return true;
    }
    let Some(b) = data.button(&g.id) else {
        return false;
    };
    if g.un {
        b.unhotkey.is_some()
    } else {
        b.hotkey.is_some() || b.researchhotkey.is_some() || morph_target(data, &g.id).is_some()
    }
}

/// An irreversible morph with no key of its own reads it from the unit it
/// makes (`Aave` -> `ubsp`): the one with a key but no button of its own.
/// The unit it starts from (`uobs`) is trained, so it has a button.
fn morph_target<'a>(data: &'a GameData, id: &str) -> Option<&'a str> {
    data.button(id).filter(|b| b.hotkey.is_none() && b.researchhotkey.is_none())?;
    let made =
        data.creates(id).iter().filter(|u| data.button(u).is_some_and(|u| u.hotkey.is_some() && u.buttonpos.is_none()));
    let mut made = made.map(String::as_str);
    let unit = made.next()?;
    made.next().is_none().then_some(unit)
}

/// One `[section]`'s fields, in the order they're written.
type Fields = BTreeMap<&'static str, String>;

fn render(data: &GameData, keys: &Keys, placed: &BTreeMap<Group, Pos>) -> String {
    let mut sections: BTreeMap<String, Fields> = BTreeMap::new();
    let pos_text = |p: Pos| format!("{},{}", p.x, p.y);
    let repeat = |key: Key, tiers: usize| vec![key.to_string(); tiers.max(1)].join(",");

    for (g, &pos) in placed {
        if g.id == CANCEL.0 {
            continue;
        }
        let key = key_for(keys, g, pos);
        let b = data.button(&g.id);
        let tiers = b.map_or(1, |b| b.tiers);
        let fields = sections.entry(g.id.clone()).or_default();
        if g.un {
            fields.insert("Unbuttonpos", pos_text(pos));
            if b.is_some_and(|b| b.unhotkey.is_some()) {
                fields.insert("Unhotkey", key.to_string());
            }
            continue;
        }
        fields.insert("Buttonpos", pos_text(pos));
        if has_key(data, g) && b.is_none_or(|b| b.hotkey.is_some() || b.researchhotkey.is_none()) {
            fields.insert("Hotkey", repeat(key, tiers));
        }
        if let Some(b) = b {
            if b.researchbuttonpos.is_some() {
                fields.insert("Researchbuttonpos", pos_text(pos));
            }
            if b.researchhotkey.is_some() {
                fields.insert("Researchhotkey", key.to_string());
            }
            // A toggle whose off-state never shows on its own card keeps
            // the same cell and key (Defend, Town Hall Call to Arms).
            if b.unbuttonpos.is_some() && !placed.contains_key(&Group { id: g.id.clone(), un: true }) {
                fields.insert("Unbuttonpos", pos_text(pos));
                if b.unhotkey.is_some() {
                    fields.insert("Unhotkey", key.to_string());
                }
            }
        }
        if let Some(unit) = morph_target(data, &g.id) {
            fields.insert("Hotkey", key.to_string());
            let target = sections.entry(unit.to_string()).or_default();
            target.insert("Buttonpos", pos_text(pos));
            target.insert("Hotkey", key.to_string());
        }
    }

    // Talent versions of the campaign heroes' abilities take the base's cell.
    // Item abilities built on them (Mordo's Club on Anya's passive) don't.
    let is_ability = |b: &Button| b.hotkey.is_some() || b.researchhotkey.is_some() || b.researchbuttonpos.is_some();
    for unit in CAMPAIGN_HEROES.iter().filter_map(|id| data.unit(id)) {
        for id in &unit.hero_abilities {
            let Some(fields) = sections.get(id).cloned() else { continue };
            for variant in data.variants(id).into_iter().filter(|v| data.button(v).is_some_and(is_ability)) {
                sections.entry(variant.to_string()).or_insert_with(|| fields.clone());
            }
        }
    }
    for (id, alias) in ALIASES {
        if let Some(fields) = sections.get(id).cloned() {
            sections.entry(alias.to_string()).or_default().extend(fields);
        }
    }
    for (id, helper) in UN_HELPERS {
        if let Some(fields) = sections.get_mut(id).filter(|f| f.contains_key("Unhotkey")) {
            fields.insert("UnhotkeyId", helper.to_string());
            sections.entry(helper.to_string()).or_default().insert("ParentId", id.to_string());
        }
    }

    // Sorted case-insensitively, like the game writes it.
    let mut ids: Vec<&String> = sections.keys().collect();
    ids.sort_by_key(|id| id.to_lowercase());
    let mut out = String::new();
    for id in ids {
        let _ = writeln!(out, "[{id}]");
        for (field, value) in &sections[id] {
            let _ = writeln!(out, "{field}={value}");
        }
        out.push('\n');
    }
    out.push_str(&crate::render_items(keys));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::Sources;
    use crate::slk::sheet as slk;

    const FUNC: &str = "[CmdMove]\nButtonpos=0,0\n[CmdStop]\nButtonpos=1,0\n[CmdHoldPos]\nButtonpos=2,0\n\
                        [CmdAttack]\nButtonpos=3,0\n[CmdPatrol]\nButtonpos=0,1\n[CmdSelectSkill]\nButtonpos=3,1\n\
                        [CmdCancel]\nButtonpos=3,2\n[ARal]\nButtonpos=3,1\n\
                        [AHbu]\nButtonpos=0,2\n[Amil]\nButtonpos=1,2\nUnbuttonpos=2,2\n\
                        [Ahar]\nButtonpos=3,1\n[Ahrp]\nButtonpos=1,1\n[Adef]\nButtonpos=0,2\nUnbuttonpos=0,2\n\
                        [AHbz]\nButtonpos=0,2\nResearchbuttonpos=0,0\n[AHab]\nButtonpos=2,2\nResearchbuttonpos=2,0\n\
                        [Rhme]\nButtonpos=0,0\n\
                        [hpea]\nBuilds=htow,hbar\nButtonpos=0,0\n[htow]\nTrains=hpea\nButtonpos=0,0\n\
                        [hbar]\nTrains=hfoo,Hamg\nResearches=Rhme\nButtonpos=1,0\n\
                        [hfoo]\nButtonpos=0,0\n[Hamg]\nButtonpos=0,2\n[hmil]\n";
    const STRINGS: &str = "[Amil]\nHotkey=C\nUnhotkey=W\n[Ahar]\nHotkey=G\n[Ahrp]\nHotkey=R\n\
                           [Adef]\nHotkey=D\nUnhotkey=D\n[AHbz]\nHotkey=B\nResearchhotkey=B\n\
                           [AHab]\nResearchhotkey=R\n[Rhme]\nHotkey=S,S,S\n[hfoo]\nHotkey=F\n\
                           [Hamg]\nHotkey=A\n[htow]\nHotkey=H\n[hbar]\nHotkey=B\n[hpea]\nHotkey=P\n";

    fn sample() -> GameData {
        let abilities = slk(&[
            &["id", "abilList", "heroAbilList"],
            &["hpea", "Ahar,Ahrp,Amil", "_"],
            &["hmil", "Ahar,Ahrp,Amil", "_"],
            &["hfoo", "Adef", "_"],
            &["Hamg", "AInv", "AHbz,AHab"],
            &["htow", "_", "_"],
            &["hbar", "_", "_"],
        ]);
        let balance = slk(&[
            &["id", "spd", "isbldg"],
            &["hpea", "190", "0"],
            &["hmil", "270", "0"],
            &["hfoo", "270", "0"],
            &["Hamg", "320", "0"],
            &["htow", "-", "1"],
            &["hbar", "-", "1"],
        ]);
        let race = slk(&[
            &["id", "race"],
            &["hpea", "human"],
            &["hmil", "human"],
            &["hfoo", "human"],
            &["Hamg", "human"],
            &["htow", "human"],
            &["hbar", "human"],
        ]);
        let weapons = slk(&[
            &["id", "weapsOn"],
            &["hpea", "1"],
            &["hmil", "1"],
            &["hfoo", "1"],
            &["Hamg", "1"],
            &["htow", "0"],
            &["hbar", "0"],
        ]);
        let ability_data = slk(&[&["alias", "code", "DataB1"], &["Amil", "Amil", "hmil"]]);
        GameData::parse(&Sources {
            func: vec![FUNC],
            strings: vec![STRINGS],
            unit_abilities: &abilities,
            unit_balance: &balance,
            unit_data: &race,
            unit_weapons: &weapons,
            ability_data: &ability_data,
        })
    }

    fn section<'a>(text: &'a str, id: &str) -> &'a str {
        let start = text.find(&format!("[{id}]\n")).unwrap_or_else(|| panic!("no [{id}] in\n{text}"));
        let rest = &text[start..];
        &rest[..rest.find("\n\n").unwrap() + 1]
    }

    #[test]
    fn lays_out_the_grid() {
        let out = generate(&sample(), &Keys::default());
        assert_eq!(out.conflicts, Vec::<String>::new());
        let t = &out.text;
        assert_eq!(section(t, "CmdAttack"), "[CmdAttack]\nButtonpos=0,1\nHotkey=A\n");
        assert_eq!(section(t, "CmdMove"), "[CmdMove]\nButtonpos=2,1\nHotkey=M\n");
        assert_eq!(section(t, "CmdPatrol"), "[CmdPatrol]\nButtonpos=0,2\nHotkey=P\n");
        assert_eq!(section(t, "CmdSelectSkill"), "[CmdSelectSkill]\nButtonpos=3,2\nHotkey=V\n");
        // Hero abilities move from the bottom row to the top, and learn on
        // the same key. A passive gets only its learn key.
        assert_eq!(section(t, "AHbz"), "[AHbz]\nButtonpos=0,0\nHotkey=Q\nResearchbuttonpos=0,0\nResearchhotkey=Q\n");
        assert_eq!(section(t, "AHab"), "[AHab]\nButtonpos=2,0\nResearchbuttonpos=2,0\nResearchhotkey=E\n");
        // Peasant: Repair and Gather leave the command row.
        assert_eq!(section(t, "Ahrp"), "[Ahrp]\nButtonpos=1,2\nHotkey=X\n");
        assert_eq!(section(t, "Ahar"), "[Ahar]\nButtonpos=3,2\nHotkey=V\n");
        // The build button is also written as CmdBuildHuman.
        assert_eq!(section(t, "AHbu"), "[AHbu]\nButtonpos=0,0\nHotkey=Q\n");
        assert_eq!(section(t, "CmdBuildHuman"), "[CmdBuildHuman]\nButtonpos=0,0\nHotkey=Q\n");
        // Call to Arms: on on the Peasant, off (Back to Work) on the Militia.
        assert_eq!(
            section(t, "Amil"),
            "[Amil]\nButtonpos=1,0\nHotkey=W\nUnbuttonpos=2,0\nUnhotkey=E\nUnhotkeyId=Ami1\n"
        );
        assert_eq!(section(t, "Ami1"), "[Ami1]\nParentId=Amil\n");
        // Same-card toggle: both states on one cell.
        assert_eq!(section(t, "Adef"), "[Adef]\nButtonpos=0,0\nHotkey=Q\nUnbuttonpos=0,0\nUnhotkey=Q\n");
        // Buildings keep Blizzard's layout; upgrades get one key per tier.
        assert_eq!(section(t, "hfoo"), "[hfoo]\nButtonpos=0,0\nHotkey=Q\n");
        assert_eq!(section(t, "Rhme"), "[Rhme]\nButtonpos=1,0\nHotkey=W,W,W\n");
        // Rally takes the corner, like Learn, whatever its default cell.
        assert_eq!(section(t, "ARal"), "[ARal]\nButtonpos=3,2\nHotkey=V\n");
        assert_eq!(section(t, "CmdRally"), "[CmdRally]\nButtonpos=3,2\nHotkey=V\n");
        assert!(t.ends_with(&crate::render_items(&Keys::default())));
        assert!(!t.contains("CmdCancel"));
    }

    #[test]
    fn uses_the_users_command_keys() {
        let mut keys = Keys::default();
        keys.set(Slot::Patrol, Key::letter('K'));
        let out = generate(&sample(), &keys);
        assert_eq!(section(&out.text, "CmdPatrol"), "[CmdPatrol]\nButtonpos=0,2\nHotkey=K\n");
        keys.set(Slot::Learn, Key::letter('T'));
        keys.set(Slot::Rally, Key::letter('Y'));
        let out = generate(&sample(), &keys);
        assert_eq!(out.conflicts, Vec::<String>::new());
        assert_eq!(section(&out.text, "CmdSelectSkill"), "[CmdSelectSkill]\nButtonpos=3,2\nHotkey=T\n");
        assert_eq!(section(&out.text, "CmdRally"), "[CmdRally]\nButtonpos=3,2\nHotkey=Y\n");
    }

    #[test]
    fn uses_the_users_grid() {
        // AZERTY, with Attack on its cell's key.
        let mut keys = Keys { grid: (*b"AZERQSDFWXCV").map(|c| Key::letter(c as char)), ..Keys::default() };
        keys.set(Slot::Attack, Key::letter('Q'));
        assert!(keys.clashes().is_empty());
        let out = generate(&sample(), &keys);
        assert_eq!(out.conflicts, Vec::<String>::new());
        assert_eq!(section(&out.text, "AHbz"), "[AHbz]\nButtonpos=0,0\nHotkey=A\nResearchbuttonpos=0,0\nResearchhotkey=A\n");
        assert_eq!(section(&out.text, "Rhme"), "[Rhme]\nButtonpos=1,0\nHotkey=Z,Z,Z\n");
        assert_eq!(section(&out.text, "CmdAttack"), "[CmdAttack]\nButtonpos=0,1\nHotkey=Q\n");
        // The screen opens with the grid the file was made with.
        assert_eq!(Keys::read(&out.text), keys);
    }
}
