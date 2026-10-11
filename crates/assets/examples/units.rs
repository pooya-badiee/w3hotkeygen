//! Prints units' command cards as 4x3 grids, read from the local install.
//! `cargo run -p w3hk-assets --example units [-- <unit ids>... | --all] [--locale dede]`
//!
//! Each cell shows the section and its default hotkey. Two buttons on one
//! cell are joined with `+`: the game moves one of them aside.

use w3hk_core::data::{Card, CardKind, Field, GameData, HIDDEN, Pos, Unit};

const DEFAULT_UNITS: [&str; 4] = ["hpea", "hfoo", "Hamg", "hbar"];

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let locale = args.iter().position(|a| a == "--locale").and_then(|i| args.get(i + 1)).map_or("enus", |s| s);
    let found = w3hk_assets::find().expect("no Warcraft III install found (set W3HK_WC3_DIR)");
    let files = w3hk_assets::GameFiles::open(&found.install).expect("open game files");
    let data = w3hk_assets::game_data::load(&files, locale).expect("load game data");

    let melee = data.melee_units();
    println!("{} units, {} melee", data.units().count(), melee.len());
    let ids: Vec<&str> = if args.iter().any(|a| a == "--all") {
        melee.iter().map(|u| u.id.as_str()).collect()
    } else {
        let picked: Vec<&str> = args.iter().map(String::as_str).filter(|a| a.len() == 4).collect();
        if picked.is_empty() { DEFAULT_UNITS.to_vec() } else { picked }
    };
    for id in ids {
        match data.unit(id) {
            Some(unit) => print_unit(&data, unit),
            None => println!("\n{id}: no such unit"),
        }
    }
}

fn print_unit(data: &GameData, unit: &Unit) {
    let name = data.button(&unit.id).map_or("", |b| b.name.as_str());
    let kind = match (unit.hero, unit.building, unit.builds.is_empty()) {
        (true, ..) => "hero",
        (_, true, _) => "building",
        (_, _, false) => "worker",
        _ => "unit",
    };
    println!("\n== {} {name} ({}, {kind})", unit.id, unit.race);
    for card in data.cards(unit) {
        print_card(data, &card);
    }
    // Toggle states that sit somewhere else.
    for id in unit.abilities.iter().chain(&unit.hero_abilities).filter(|a| !HIDDEN.contains(&a.as_str())) {
        if let Some(b) = data.button(id).filter(|b| b.unbuttonpos.is_some() && b.unbuttonpos != b.buttonpos) {
            let pos = b.unbuttonpos.unwrap();
            println!("   {id} off-state at {},{} [{}]", pos.x, pos.y, key(b.unhotkey));
        }
    }
}

fn print_card(data: &GameData, card: &Card) {
    let title = match card.kind {
        CardKind::Main => "main",
        CardKind::Uprooted => "uprooted",
        CardKind::Build => "build menu",
        CardKind::Learn => "learn menu",
    };
    let label = |pos: Pos| {
        let cell: Vec<String> = card
            .at(pos)
            .map(|p| {
                let b = data.button(&p.id);
                let k = b.and_then(|b| match p.field {
                    Field::Button => b.hotkey,
                    Field::Un => b.unhotkey,
                    Field::Research => b.researchhotkey,
                });
                format!("{} {}", p.id, key(k))
            })
            .collect();
        cell.join(" + ")
    };
    let cells: Vec<Vec<String>> =
        (0..Pos::ROWS).map(|y| (0..Pos::COLS).map(|x| label(Pos::new(x, y))).collect()).collect();
    let width = cells.iter().flatten().map(String::len).max().unwrap_or(0).max(9);
    let rule = format!("   +{}", format!("{}+", "-".repeat(width + 2)).repeat(Pos::COLS as usize));
    println!("   {title}");
    println!("{rule}");
    for row in &cells {
        let line: String = row.iter().map(|c| format!(" {c:width$} |")).collect();
        println!("   |{line}");
        println!("{rule}");
    }
    if !card.unplaced.is_empty() {
        println!("   not on the card: {}", card.unplaced.join(", "));
    }
}

fn key(k: Option<w3hk_core::Key>) -> String {
    k.map_or("-".into(), |k| k.label())
}
