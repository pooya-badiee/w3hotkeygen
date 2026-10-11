//! Reads unit and ability data out of the game files. Parsing is in
//! `w3hk_core::data`; this only finds and reads the files.

use std::fmt;

use w3hk_core::data::{GameData, Sources};

use crate::GameFiles;

/// The `*func.txt` / `*strings.txt` file stems, in load order. Campaign files
/// go first so the melee files win where both define a section.
const STEMS: [&str; 22] = [
    "campaignability",
    "campaignunit",
    "campaignupgrade",
    "commonability",
    "itemability",
    "item",
    "humanability",
    "humanunit",
    "humanupgrade",
    "orcability",
    "orcunit",
    "orcupgrade",
    "undeadability",
    "undeadunit",
    "undeadupgrade",
    "nightelfability",
    "nightelfunit",
    "nightelfupgrade",
    "neutralability",
    "neutralunit",
    "neutralupgrade",
    "command",
];

#[derive(Debug)]
pub enum LoadError {
    /// A file the data can't do without.
    Missing(String),
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LoadError::Missing(path) => write!(f, "Game file not found: {path}"),
        }
    }
}

impl std::error::Error for LoadError {}

/// Units, abilities and their default positions. Names and hotkeys come from
/// `locale` (e.g. `enus`, `dede`), falling back to `enus` file by file.
pub fn load(files: &GameFiles, locale: &str) -> Result<GameData, LoadError> {
    let read = |path: &str| files.read(path).map(|b| String::from_utf8_lossy(&b).into_owned());
    let base = |name: &str| format!("war3.w3mod:units\\{name}");
    let localized = |loc: &str, name: &str| format!("war3.w3mod:_locales\\{loc}.w3mod:units\\{name}");
    let required = |name: &str| read(&base(name)).ok_or_else(|| LoadError::Missing(base(name)));

    let func: Vec<String> = STEMS.iter().filter_map(|s| read(&base(&format!("{s}func.txt")))).collect();
    let strings: Vec<String> = STEMS
        .iter()
        .filter_map(|s| {
            let name = format!("{s}strings.txt");
            read(&localized(locale, &name)).or_else(|| read(&localized("enus", &name)))
        })
        .collect();
    if func.is_empty() {
        return Err(LoadError::Missing(base("*func.txt")));
    }
    if strings.is_empty() {
        return Err(LoadError::Missing(localized(locale, "*strings.txt")));
    }

    let unit_abilities = required("unitabilities.slk")?;
    let unit_balance = required("unitbalance.slk")?;
    let unit_data = required("unitdata.slk")?;
    let unit_weapons = required("unitweapons.slk")?;
    let ability_data = required("abilitydata.slk")?;
    Ok(GameData::parse(&Sources {
        func: func.iter().map(String::as_str).collect(),
        strings: strings.iter().map(String::as_str).collect(),
        unit_abilities: &unit_abilities,
        unit_balance: &unit_balance,
        unit_data: &unit_data,
        unit_weapons: &unit_weapons,
        ability_data: &ability_data,
    }))
}
