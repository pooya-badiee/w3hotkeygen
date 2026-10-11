//! Which game files hold the unit and ability data, and loading them. The
//! caller does the reading (CASC on native, the browser on the web), so this
//! stays free of IO.

use std::fmt;

use crate::data::{GameData, Sources};

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

const SLKS: [&str; 5] = ["unitabilities.slk", "unitbalance.slk", "unitdata.slk", "unitweapons.slk", "abilitydata.slk"];

fn base(name: &str) -> String {
    format!("war3.w3mod:units\\{name}")
}

fn localized(locale: &str, name: &str) -> String {
    format!("war3.w3mod:_locales\\{locale}.w3mod:units\\{name}")
}

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

/// Every file [`load`] may read for `locale`, so a caller that can't read on
/// demand can fetch them first. Some don't exist in every version.
pub fn paths(locale: &str) -> Vec<String> {
    let mut out: Vec<String> = STEMS.iter().map(|s| base(&format!("{s}func.txt"))).collect();
    for s in STEMS {
        let name = format!("{s}strings.txt");
        out.push(localized(locale, &name));
        if locale != "enus" {
            out.push(localized("enus", &name));
        }
    }
    out.extend(SLKS.map(base));
    out
}

/// Units, abilities and their default positions. Names and hotkeys come from
/// `locale` (e.g. `enus`, `dede`), falling back to `enus` file by file.
/// `read` returns a file's text, or `None` if it doesn't exist.
pub fn load(read: impl Fn(&str) -> Option<String>, locale: &str) -> Result<GameData, LoadError> {
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

    let [unit_abilities, unit_balance, unit_data, unit_weapons, ability_data] = [
        required(SLKS[0])?,
        required(SLKS[1])?,
        required(SLKS[2])?,
        required(SLKS[3])?,
        required(SLKS[4])?,
    ];
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_cover_the_locale_and_its_fallback() {
        let paths = paths("dede");
        assert!(paths.contains(&r"war3.w3mod:units\humanunitfunc.txt".to_string()));
        assert!(paths.contains(&r"war3.w3mod:_locales\dede.w3mod:units\humanunitstrings.txt".to_string()));
        assert!(paths.contains(&r"war3.w3mod:_locales\enus.w3mod:units\humanunitstrings.txt".to_string()));
        assert!(paths.contains(&r"war3.w3mod:units\abilitydata.slk".to_string()));
        assert_eq!(paths.len(), 22 * 3 + 5);
    }

    #[test]
    fn missing_files_are_reported() {
        assert!(matches!(load(|_| None, "enus"), Err(LoadError::Missing(_))));
    }
}
