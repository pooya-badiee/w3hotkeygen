//! Checks the game data against `CustomKeys.txt` files, on a real install.
//! Skipped (passes) when there's no install.
//!
//! - The game's own template, `war3.w3mod:customkeys.txt`, holds Blizzard's
//!   default hotkeys, so those must match.
//! - The user's file holds their own layout, so differences are listed, not
//!   failed on. `W3HK_CUSTOM_KEYS` overrides its path; it's only ever read.
//!
//! Run with `--nocapture` to see the differences.

use std::collections::BTreeSet;

use w3hk_core::Key;
use w3hk_core::data::{Field, GameData, Pos};
use w3hk_core::ini::Ini;

fn open() -> Option<w3hk_assets::GameFiles> {
    let Some(found) = w3hk_assets::find() else {
        eprintln!("skipped: no Warcraft III install");
        return None;
    };
    match w3hk_assets::GameFiles::open(&found.install) {
        Ok(files) => Some(files),
        Err(e) => {
            eprintln!("skipped: {e}");
            None
        }
    }
}

/// The game writes some sections in a different case from its data (`[afsh]`
/// for `Afsh`), so compare section names case-insensitively.
fn parse_lowercase(text: &str) -> Ini {
    let lower = |l: &str| if l.starts_with('[') { l.to_lowercase() } else { l.to_string() };
    Ini::parse(&text.lines().map(lower).collect::<Vec<_>>().join("\n"))
}

fn show_pos(pos: Option<Pos>) -> String {
    pos.map_or("-".into(), |p| format!("{},{}", p.x, p.y))
}

fn show_key(key: Option<Key>) -> String {
    key.map_or("-".into(), |k| k.to_string())
}

#[derive(Default)]
struct Report {
    buttons: usize,
    pos_same: usize,
    key_same: usize,
    pos_differ: Vec<String>,
    key_differ: Vec<String>,
    /// Sections with a hotkey that differs: `ARal`, `Aave`.
    key_differ_ids: Vec<String>,
    absent: Vec<String>,
}

/// Compares every button on a melee card with `file`.
fn compare(data: &GameData, file: &Ini) -> Report {
    let mut placed = BTreeSet::new();
    for unit in data.melee_units() {
        for card in data.cards(unit) {
            placed.extend(card.buttons.into_iter().map(|b| (b.id, b.field)));
        }
    }
    let mut r = Report { buttons: placed.len(), ..Report::default() };
    for (id, field) in &placed {
        let b = data.button(id).expect("placed buttons have data");
        let (pos_field, key_field, pos, key) = match field {
            Field::Button => ("Buttonpos", "Hotkey", b.buttonpos, b.hotkey),
            Field::Un => ("Unbuttonpos", "Unhotkey", b.unbuttonpos, b.unhotkey),
            Field::Research => ("Researchbuttonpos", "Researchhotkey", b.researchbuttonpos, b.researchhotkey),
        };
        let section = id.to_lowercase();
        match file.get(&section, pos_field) {
            None => r.absent.push(format!("{id}.{pos_field}")),
            Some(theirs) if Pos::parse(theirs) == pos => r.pos_same += 1,
            Some(theirs) => r.pos_differ.push(format!("{id}.{pos_field} {theirs} (default {})", show_pos(pos))),
        }
        match file.get(&section, key_field) {
            Some(theirs) if Key::parse(theirs) == key => r.key_same += 1,
            Some(theirs) => {
                r.key_differ.push(format!("{id}.{key_field} {theirs} (default {})", show_key(key)));
                r.key_differ_ids.push(id.clone());
            }
            None => {}
        }
    }
    println!("{} buttons on melee cards", r.buttons);
    println!("positions: {} match, {} differ, {} not in the file", r.pos_same, r.pos_differ.len(), r.absent.len());
    println!("hotkeys: {} match, {} differ", r.key_same, r.key_differ.len());
    println!("\nhotkey differences:\n  {}", r.key_differ.join("\n  "));
    r
}

#[test]
fn melee_roster() {
    let Some(files) = open() else { return };
    let data = w3hk_assets::game_data::load(&files, "enus").expect("load game data");
    let melee = data.melee_units();
    assert!(melee.len() > 250, "only {} melee units", melee.len());
    for id in ["hpea", "hfoo", "Hamg", "hbar", "opeo", "uaco", "ewsp", "etol", "ngme", "nzep"] {
        assert!(melee.iter().any(|u| u.id == id), "{id} missing from the melee roster");
    }
}

#[test]
fn hotkeys_match_shipped_template() {
    let Some(files) = open() else { return };
    let data = w3hk_assets::game_data::load(&files, "enus").expect("load game data");
    let text = files.read("war3.w3mod:customkeys.txt").expect("shipped customkeys.txt");
    let r = compare(&data, &parse_lowercase(&String::from_utf8_lossy(&text)));
    // Known: these defaults are only in the template (the strings have none),
    // and the template has a stray `Hotkey=G` after `[hctw]`.
    let known = ["ARal", "Aave", "hctw"];
    let unexpected: Vec<&String> = r.key_differ_ids.iter().filter(|id| !known.contains(&id.as_str())).collect();
    assert!(unexpected.is_empty(), "hotkeys differ from the template: {unexpected:?}");
    assert!(r.key_same > 500, "only {} hotkeys compared", r.key_same);
}

#[test]
fn defaults_vs_user_file() {
    let Some(files) = open() else { return };
    let path = std::env::var_os("W3HK_CUSTOM_KEYS").map(Into::into).or_else(w3hk_assets::custom_keys_path);
    let Some(text) = path.as_ref().and_then(|p| std::fs::read_to_string(p).ok()) else {
        eprintln!("skipped: no CustomKeys.txt");
        return;
    };
    let data = w3hk_assets::game_data::load(&files, "enus").expect("load game data");
    let r = compare(&data, &parse_lowercase(&text));
    println!("\nposition differences:\n  {}", r.pos_differ.join("\n  "));
    println!("\nno position in the file: {}", r.absent.join(", "));
}

/// Reads the generated file back and checks every melee card: no two buttons
/// on one cell, no key twice. Independent of the generator's own check, and
/// it includes shop wares, which keep Blizzard's keys.
#[test]
fn generated_grid_has_no_conflicts() {
    let Some(files) = open() else { return };
    let data = w3hk_assets::game_data::load(&files, "enus").expect("load game data");
    let out = w3hk_core::grid::generate(&data, &w3hk_core::Keys::default());
    assert!(out.conflicts.is_empty(), "generator reports conflicts: {:#?}", out.conflicts);

    let file = parse_lowercase(&out.text);
    let mut problems = BTreeSet::new();
    for unit in data.melee_units() {
        for card in data.cards(unit) {
            let mut cells = std::collections::BTreeMap::<Pos, Vec<&str>>::new();
            let mut keys = std::collections::BTreeMap::<u16, Vec<&str>>::new();
            for b in &card.buttons {
                let d = data.button(&b.id).unwrap();
                let (pos_field, key_field, default_key) = match b.field {
                    Field::Button => ("Buttonpos", "Hotkey", d.hotkey),
                    Field::Un => ("Unbuttonpos", "Unhotkey", d.unhotkey),
                    Field::Research => ("Researchbuttonpos", "Researchhotkey", d.researchhotkey),
                };
                let section = b.id.to_lowercase();
                let key = file.get(&section, key_field).and_then(Key::parse).or(default_key);
                if let Some(key) = key {
                    keys.entry(key.0).or_default().push(&b.id);
                }
                let pos = file.get(&section, pos_field).and_then(Pos::parse).unwrap_or(b.pos);
                cells.entry(pos).or_default().push(&b.id);
            }
            for (pos, ids) in cells.iter().filter(|(_, ids)| ids.len() > 1) {
                problems.insert(format!("{}: {ids:?} on {}", unit.id, show_pos(Some(*pos))));
            }
            for (key, ids) in keys.iter().filter(|(_, ids)| ids.len() > 1) {
                problems.insert(format!("{}: {ids:?} on key {}", unit.id, Key(*key)));
            }
        }
    }
    assert!(
        problems.is_empty(),
        "{} conflicts:\n{}",
        problems.len(),
        problems.into_iter().collect::<Vec<_>>().join("\n")
    );
}
