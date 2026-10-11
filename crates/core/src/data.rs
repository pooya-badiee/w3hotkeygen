//! Game data: which buttons each unit has, and where they sit.
//!
//! Read from the game's own files, handed in as text (see [`Sources`]):
//! positions from `units\*func.txt`, names and default hotkeys from the
//! locale's `units\*strings.txt`, abilities from `units\unitabilities.slk`,
//! and whether a unit moves or attacks from the other unit SLKs.
//! See docs/game-files.md.

use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};

use crate::Key;
use crate::ini::{Ini, split_list};
use crate::slk::Slk;

/// A cell on the 4x3 command card: x 0-3 from the left, y 0-2 from the top.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Pos {
    pub x: u8,
    pub y: u8,
}

impl Pos {
    pub const COLS: u8 = 4;
    pub const ROWS: u8 = 3;

    pub const fn new(x: u8, y: u8) -> Pos {
        Pos { x, y }
    }

    /// Parses `x,y`. `None` if it's off the card: the game hides some buttons
    /// that way (`Buttonpos=0,-11`). Extra values are ignored; the game itself
    /// sometimes writes `Buttonpos=3,1,3,1`.
    pub fn parse(value: &str) -> Option<Pos> {
        let mut parts = value.split(',').map(|v| v.trim().parse::<i32>());
        let (x, y) = (parts.next()?.ok()?, parts.next()?.ok()?);
        let on_card = (0..Pos::COLS as i32).contains(&x) && (0..Pos::ROWS as i32).contains(&y);
        on_card.then(|| Pos::new(x as u8, y as u8))
    }
}

/// The game's defaults for one `CustomKeys.txt` section: an ability, unit,
/// upgrade, item or `Cmd*` command.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Button {
    pub name: String,
    pub buttonpos: Option<Pos>,
    /// The other state of a toggle or morph.
    pub unbuttonpos: Option<Pos>,
    /// The hero learn menu.
    pub researchbuttonpos: Option<Pos>,
    pub hotkey: Option<Key>,
    pub unhotkey: Option<Key>,
    pub researchhotkey: Option<Key>,
    /// Number of `Hotkey` tokens (`S,S,S`): upgrades need one per tier.
    pub tiers: usize,
}

impl Button {
    fn read(ini: &Ini, id: &str) -> Button {
        let pos = |field| ini.get(id, field).and_then(Pos::parse);
        let key = |field| ini.get(id, field).and_then(Key::parse);
        Button {
            name: ini.get(id, "Name").or(ini.get(id, "Tip")).unwrap_or_default().to_string(),
            buttonpos: pos("Buttonpos"),
            unbuttonpos: pos("Unbuttonpos"),
            // Four campaign abilities misspell it `Reserachbuttonpos`.
            researchbuttonpos: pos("Researchbuttonpos").or(pos("Reserachbuttonpos")),
            hotkey: key("Hotkey"),
            unhotkey: key("Unhotkey"),
            researchhotkey: key("Researchhotkey"),
            tiers: ini.get(id, "Hotkey").map_or(0, |v| v.split(',').count()),
        }
    }
}

/// A unit, hero or building, and what goes on its command card.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Unit {
    pub id: String,
    pub race: String,
    pub hero: bool,
    pub building: bool,
    /// Has Move, Hold Position and Patrol.
    pub moves: bool,
    /// Has Attack.
    pub attacks: bool,
    /// Has Attack Ground (an artillery weapon).
    pub attacks_ground: bool,
    /// `abilList`: the unit's own abilities, including hidden ones.
    pub abilities: Vec<String>,
    /// `heroAbilList`: abilities a hero learns.
    pub hero_abilities: Vec<String>,
    pub trains: Vec<String>,
    /// Buildings in the worker's build menu.
    pub builds: Vec<String>,
    /// Upgrades researched here.
    pub researches: Vec<String>,
    /// What this building upgrades into (`htow` -> `hkee`).
    pub upgrades: Vec<String>,
    pub sells_units: Vec<String>,
    /// `Sellitems` and `Makeitems`.
    pub sells_items: Vec<String>,
    /// An altar: revives heroes.
    pub revives: bool,
}

/// The game files [`GameData::parse`] needs, as text.
#[derive(Debug, Default)]
pub struct Sources<'a> {
    /// `units\*func.txt` and `units\commandfunc.txt`. Later files override
    /// earlier ones field by field.
    pub func: Vec<&'a str>,
    /// The locale's `units\*strings.txt`.
    pub strings: Vec<&'a str>,
    pub unit_abilities: &'a str,
    pub unit_balance: &'a str,
    pub unit_data: &'a str,
    pub unit_weapons: &'a str,
    pub ability_data: &'a str,
}

/// Town halls and the neutral buildings of melee maps: every melee unit is
/// built, trained, upgraded to, bought or summoned from one of these. The
/// neutral list is the one in the game's own template, `war3.w3mod:customkeys.txt`.
const MELEE_ROOTS: [&str; 30] = [
    "htow", "ogre", "unpl", "etol", // town halls
    "ntav", "ngme", "ngad", "nshp", // tavern, goblin merchant, laboratory, shipyard
    "ndrk", "ndru", "ndrz", "ndrg", "ndrr", "ndro", // dragon roosts
    "nmer", "nmr2", "nmr3", "nmr4", "nmr5", "nmr6", "nmr7", "nmr8", "nmr9", // mercenary camps
    "nmr0", "nmra", "nmrb", "nmrc", "nmrd", "nmre", "nmrf",
];

/// Abilities with a `Buttonpos` that never show a button. Left on the card
/// they'd make phantom collisions (see docs/customkeys-format.md).
pub const HIDDEN: [&str; 3] = ["Aall", "Apxf", "Aenc"];

/// A shop's Select User. It shows its off-state, in the corner (inferred):
/// Blizzard puts wares on its `Buttonpos` (0,0) but never on its
/// `Unbuttonpos` (3,2).
const SELECT_USER: [&str; 2] = ["Aneu", "Ane2"];

/// Abilities an ancient can only use uprooted.
const UPROOTED_ONLY: [&str; 1] = ["Aeat"];

/// Units whose `Builds=` list doesn't apply. Militia copies the Peasant's
/// list; the game writes `IgnoreBuilds=1` for it in CustomKeys.txt.
const IGNORE_BUILDS: [&str; 1] = ["hmil"];

#[derive(Debug, Default)]
pub struct GameData {
    buttons: HashMap<String, Button>,
    /// Lowercase ID -> ID, for references in the wrong case.
    lowercase: HashMap<String, String>,
    units: BTreeMap<String, Unit>,
    /// Units an ability summons or morphs into, from `abilitydata.slk`.
    creates: HashMap<String, Vec<String>>,
    /// Each ability's base code (`Aro1` -> `Aroo`), from `abilitydata.slk`.
    codes: HashMap<String, String>,
    /// A morph's other form (`Astn` -> `ugrm`): on that unit the ability
    /// shows its `Un` side.
    forms: HashMap<String, Vec<String>>,
}

impl GameData {
    pub fn parse(src: &Sources) -> GameData {
        let mut ini = Ini::default();
        for text in src.func.iter().chain(&src.strings) {
            ini.add(text);
        }
        let buttons: HashMap<String, Button> =
            ini.sections().map(|id| (id.to_string(), Button::read(&ini, id))).collect();

        let abilities = Slk::parse(src.unit_abilities);
        let balance = Slk::parse(src.unit_balance);
        let data = Slk::parse(src.unit_data);
        let weapons = Slk::parse(src.unit_weapons);
        let mut units = BTreeMap::new();
        for id in data.ids() {
            let list = |slk: &Slk, column| split_list(slk.get(id, column).unwrap_or(""));
            let weaps_on: u8 = weapons.get(id, "weapsOn").and_then(|v| v.parse().ok()).unwrap_or(0);
            let artillery = (1..=2).any(|i| {
                weaps_on & (1 << (i - 1)) != 0
                    && matches!(weapons.get(id, &format!("weapTp{i}")), Some("artillery" | "aline"))
            });
            let building = balance.get(id, "isbldg") == Some("1");
            let speed: f32 = balance.get(id, "spd").and_then(|v| v.parse().ok()).unwrap_or(0.0);
            let mut sells_items = ini.list(id, "Sellitems");
            sells_items.extend(ini.list(id, "Makeitems"));
            let unit = Unit {
                id: id.to_string(),
                race: data.get(id, "race").unwrap_or_default().to_string(),
                hero: id.starts_with(|c: char| c.is_ascii_uppercase()),
                building,
                // Rooted ancients have a speed but no Move until they uproot.
                moves: !building && speed > 0.0,
                attacks: weaps_on != 0,
                attacks_ground: artillery,
                abilities: list(&abilities, "abilList"),
                hero_abilities: list(&abilities, "heroAbilList"),
                trains: ini.list(id, "Trains"),
                builds: if IGNORE_BUILDS.contains(&id) { Vec::new() } else { ini.list(id, "Builds") },
                researches: ini.list(id, "Researches"),
                upgrades: ini.list(id, "Upgrade"),
                sells_units: ini.list(id, "Sellunits"),
                sells_items,
                revives: ini.get(id, "Revive") == Some("1"),
            };
            units.insert(id.to_string(), unit);
        }

        // Summons and morphs name their unit in `UnitID<level>`, or in a data
        // column (`Amil`: DataB1 = hmil). Any cell holding a unit ID counts.
        let ability_data = Slk::parse(src.ability_data);
        let columns: Vec<&str> =
            ability_data.columns().filter(|c| c.starts_with("UnitID") || c.starts_with("Data")).collect();
        let (mut creates, mut codes, mut forms) = (HashMap::new(), HashMap::new(), HashMap::new());
        for id in ability_data.ids() {
            let code = ability_data.get(id, "code");
            let units_in = |prefix: &str| {
                let mut found: Vec<String> = columns
                    .iter()
                    .filter(|c| c.starts_with(prefix))
                    .filter_map(|c| ability_data.get(id, c))
                    .flat_map(split_list)
                    .filter(|u| units.contains_key(u))
                    .collect();
                found.sort();
                found.dedup();
                found
            };
            let made = units_in("");
            if !made.is_empty() {
                creates.insert(id.to_string(), made);
            }
            // The other form is in `UnitID<level>`, except for Call to Arms.
            let form = units_in(if code == Some("Amil") { "DataB" } else { "UnitID" });
            if !form.is_empty() {
                forms.insert(id.to_string(), form);
            }
            if let Some(code) = code {
                codes.insert(id.to_string(), code.to_string());
            }
        }

        let lowercase = buttons.keys().map(|id| (id.to_lowercase(), id.clone())).collect();
        GameData { buttons, lowercase, units, creates, codes, forms }
    }

    /// A section's defaults. Falls back to a case-insensitive match, like
    /// the game: `nane` lists `Acvs`, but only `ACvs` has data.
    pub fn button(&self, id: &str) -> Option<&Button> {
        self.buttons.get(id).or_else(|| self.buttons.get(self.lowercase.get(&id.to_lowercase())?))
    }

    pub fn unit(&self, id: &str) -> Option<&Unit> {
        self.units.get(id)
    }

    pub fn units(&self) -> impl Iterator<Item = &Unit> {
        self.units.values()
    }

    /// Units an ability summons or morphs into.
    pub fn creates(&self, ability: &str) -> &[String] {
        self.creates.get(ability).map_or(&[], Vec::as_slice)
    }

    /// Every unit reachable from [`MELEE_ROOTS`]: built, trained, upgraded
    /// to, bought, summoned or morphed into. Sorted by ID.
    pub fn melee_units(&self) -> Vec<&Unit> {
        let mut seen: HashSet<&str> = MELEE_ROOTS.into_iter().collect();
        let mut queue: VecDeque<&str> = MELEE_ROOTS.into_iter().collect();
        while let Some(id) = queue.pop_front() {
            let Some(unit) = self.units.get(id) else { continue };
            let made = unit.abilities.iter().chain(&unit.hero_abilities).flat_map(|a| self.creates(a));
            let next =
                unit.trains.iter().chain(&unit.builds).chain(&unit.upgrades).chain(&unit.sells_units).chain(made);
            for next in next {
                if self.units.contains_key(next) && seen.insert(next) {
                    queue.push_back(next);
                }
            }
        }
        let mut units: Vec<&Unit> = seen.into_iter().filter_map(|id| self.units.get(id)).collect();
        units.sort_by(|a, b| a.id.cmp(&b.id));
        units
    }

    /// The ability's base code: `Aro1` and `Aro2` are both `Aroo` (Root).
    pub fn code<'a>(&'a self, ability: &'a str) -> &'a str {
        self.codes.get(ability).map_or(ability, String::as_str)
    }

    /// Other abilities built on this one's code: a hero's talent versions
    /// (`AUvg` -> `AUv1`, `AUv2`, `AUv3`). Sorted by ID.
    pub fn variants<'a>(&'a self, ability: &str) -> Vec<&'a str> {
        let mut out: Vec<&str> =
            self.codes.iter().filter(|(id, code)| *code == ability && *id != ability).map(|(id, _)| id.as_str()).collect();
        out.sort();
        out
    }

    /// The unit's Root ability, if it's an ancient.
    fn root<'a>(&self, unit: &'a Unit) -> Option<&'a str> {
        unit.abilities.iter().map(String::as_str).find(|a| self.code(a) == "Aroo")
    }

    /// The unit's command cards: its main card (rooted, for ancients), plus
    /// the uprooted card, the build menu for workers and the learn menu for
    /// heroes.
    pub fn cards(&self, unit: &Unit) -> Vec<Card> {
        let mut cards = vec![self.main_card(unit)];
        if let Some(root) = self.root(unit) {
            let mut card = Card::new(CardKind::Uprooted);
            for id in ["CmdMove", "CmdStop", "CmdHoldPos", "CmdAttack", "CmdPatrol"] {
                card.push(self, id, Field::Button);
            }
            for id in unit.abilities.iter().filter(|a| UPROOTED_ONLY.contains(&a.as_str())) {
                card.push(self, id, Field::Button);
            }
            card.push(self, root, Field::Button);
            cards.push(card);
        }
        if !unit.builds.is_empty() {
            let mut card = Card::new(CardKind::Build);
            for id in &unit.builds {
                card.push(self, id, Field::Button);
            }
            card.push(self, "CmdCancel", Field::Button);
            cards.push(card);
        }
        if !unit.hero_abilities.is_empty() {
            let mut card = Card::new(CardKind::Learn);
            for id in &unit.hero_abilities {
                card.push(self, id, Field::Research);
            }
            card.push(self, "CmdCancel", Field::Button);
            cards.push(card);
        }
        cards
    }

    fn main_card(&self, unit: &Unit) -> Card {
        let mut card = Card::new(CardKind::Main);
        let root = self.root(unit);
        // A building that trains, researches or sells shows no Attack/Stop
        // even with a weapon (Halls of the Dead, rooted ancients). Inferred:
        // the game flags only towers, Ziggurats and the Protector as
        // `OffensiveStructure` in the CustomKeys.txt it writes, and their
        // default positions overlap Stop/Attack.
        let busy = !(unit.trains.is_empty()
            && unit.researches.is_empty()
            && unit.sells_units.is_empty()
            && unit.sells_items.is_empty())
            || unit.revives;
        let attacks = unit.attacks && !(unit.building && busy);
        let mut ids: Vec<&str> = Vec::new();
        if unit.moves {
            ids.push("CmdMove");
        }
        if unit.moves || attacks {
            ids.push("CmdStop");
        }
        if unit.moves {
            ids.push("CmdHoldPos");
        }
        if attacks {
            ids.push("CmdAttack");
        }
        if unit.moves {
            ids.push("CmdPatrol");
        }
        if unit.attacks_ground {
            ids.push("CmdAttackGround");
        }
        if unit.hero {
            ids.push("CmdSelectSkill");
        }
        let abilities =
            unit.abilities.iter().filter(|a| Some(a.as_str()) != root && !UPROOTED_ONLY.contains(&a.as_str()));
        ids.extend(abilities.chain(&unit.hero_abilities).map(String::as_str));
        if !unit.builds.is_empty() {
            ids.push(build_ability(&unit.race));
        }
        let more = [&unit.trains, &unit.researches, &unit.upgrades, &unit.sells_units, &unit.sells_items];
        ids.extend(more.into_iter().flatten().map(String::as_str));
        if !unit.trains.is_empty() || unit.revives {
            ids.push("ARal");
        }
        for id in ids {
            // A morphed unit shows its morph's other side (Militia: Back to Work).
            let other_form = self.forms.get(id).is_some_and(|f| f.contains(&unit.id));
            let has_un = self.button(id).is_some_and(|b| b.unbuttonpos.is_some());
            let un = (other_form && has_un) || SELECT_USER.contains(&id);
            card.push(self, id, if un { Field::Un } else { Field::Button });
        }
        // Rooted, the Root ability shows its Uproot side.
        if let Some(root) = root {
            card.push(self, root, Field::Un);
        }
        card
    }
}

/// The worker's build button. The game reads its position and key from
/// these, not from `CmdBuild*` (see docs/customkeys-format.md).
fn build_ability(race: &str) -> &'static str {
    match race {
        "human" => "AHbu",
        "orc" => "AObu",
        "undead" => "AUbu",
        "nightelf" => "AEbu",
        "naga" => "AGbu",
        _ => "ANbu",
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CardKind {
    Main,
    /// An uprooted ancient.
    Uprooted,
    /// A worker's build menu.
    Build,
    /// A hero's learn menu.
    Learn,
}

/// Which of a section's positions a button uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Field {
    Button,
    /// The other state of a toggle (`Unbuttonpos`).
    Un,
    Research,
}

/// One button on a card.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Placed {
    /// The `CustomKeys.txt` section: a rawcode or `Cmd*`.
    pub id: String,
    pub pos: Pos,
    pub field: Field,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Card {
    pub kind: CardKind,
    /// In data order: commands, abilities, then trained units, upgrades and
    /// items. Two buttons can share a cell; the game then moves one aside.
    pub buttons: Vec<Placed>,
    /// On the unit but without a position on the card: hidden abilities
    /// (`AInv`, `Abds`, [`HIDDEN`]) or buttons with no data.
    pub unplaced: Vec<String>,
}

impl Card {
    fn new(kind: CardKind) -> Card {
        Card { kind, buttons: Vec::new(), unplaced: Vec::new() }
    }

    fn push(&mut self, data: &GameData, id: &str, field: Field) {
        let button = data.button(id).filter(|_| !HIDDEN.contains(&id));
        let pos = button.and_then(|b| match field {
            Field::Button => b.buttonpos,
            Field::Un => b.unbuttonpos,
            Field::Research => b.researchbuttonpos,
        });
        match pos {
            Some(pos) => self.buttons.push(Placed { id: id.to_string(), pos, field }),
            None => self.unplaced.push(id.to_string()),
        }
    }

    /// The buttons in one cell.
    pub fn at(&self, pos: Pos) -> impl Iterator<Item = &Placed> {
        self.buttons.iter().filter(move |b| b.pos == pos)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::slk::sheet;

    const FUNC: &str = "[CmdMove]\nButtonpos=0,0\n[CmdStop]\nButtonpos=1,0\n[CmdHoldPos]\nButtonpos=2,0\n\
                        [CmdAttack]\nButtonpos=3,0\n[CmdPatrol]\nButtonpos=0,1\n[CmdSelectSkill]\nButtonpos=3,1\n\
                        [CmdCancel]\nButtonPos=3,2\n[ARal]\nButtonpos=3,1\n\
                        [AHbu]\nButtonpos=0,2\n[Amil]\nButtonpos=1,2\nUnbuttonpos=2,2\n[AInv]\n\
                        [AHbz]\nButtonpos=0,2\nResearchbuttonpos=0,0\n[Asas]\nButtonpos=0,-11\n\
                        [Rhme]\nButtonpos=0,0\n\
                        [hpea]\nBuilds=htow,hbar\nButtonpos=0,0\n[hmil]\nBuilds=htow,hbar\n[htow]\nTrains=hpea\nUpgrade=hkee\nButtonpos=0,2\n\
                        [hkee]\nButtonpos=0,2\n[hbar]\nTrains=hfoo,Hamg\nResearches=Rhme\nButtonpos=1,0\n\
                        [hfoo]\nButtonpos=0,0\n[Hamg]\nButtonpos=0,2\n";
    const STRINGS: &str = "[Amil]\nName=Call to Arms\nHotkey=C\nUnhotkey=W\n\
                           [AHbz]\nName=Blizzard\nHotkey=B\nResearchhotkey=B\n[Rhme]\nHotkey=S,S,S\n";

    /// Unit rows: id, race, abilList, heroAbilList, spd, isbldg, weapsOn.
    fn parse(func: &str, strings: &str, units: &[[&str; 7]], ability_data: &[&[&str]]) -> GameData {
        let table = |columns: &[(usize, &'static str)]| {
            let mut rows: Vec<Vec<&str>> = vec![std::iter::once("id").chain(columns.iter().map(|c| c.1)).collect()];
            rows.extend(units.iter().map(|u| std::iter::once(u[0]).chain(columns.iter().map(|c| u[c.0])).collect()));
            sheet(&rows.iter().map(Vec::as_slice).collect::<Vec<_>>())
        };
        GameData::parse(&Sources {
            func: vec![func],
            strings: vec![strings],
            unit_abilities: &table(&[(2, "abilList"), (3, "heroAbilList")]),
            unit_balance: &table(&[(4, "spd"), (5, "isbldg")]),
            unit_data: &table(&[(1, "race")]),
            unit_weapons: &table(&[(6, "weapsOn")]),
            ability_data: &sheet(ability_data),
        })
    }

    fn sample() -> GameData {
        parse(
            FUNC,
            STRINGS,
            &[
                ["hpea", "human", "Amil", "_", "190", "0", "3"],
                ["Hamg", "human", "AInv", "AHbz", "320", "0", "1"],
                ["htow", "human", "_", "_", "-", "1", "0"],
                ["hbar", "human", "_", "_", "-", "1", "0"],
                ["hkee", "human", "_", "_", "-", "1", "0"],
                ["hfoo", "human", "_", "_", "270", "0", "1"],
                ["hmil", "human", "Amil", "_", "270", "0", "1"],
            ],
            &[&["alias", "code", "DataA1", "DataB1", "UnitID1"], &["Amil", "Amil", "hpea", "hmil", "_"]],
        )
    }

    fn layout(card: &Card) -> Vec<(&str, u8, u8)> {
        card.buttons.iter().map(|b| (b.id.as_str(), b.pos.x, b.pos.y)).collect()
    }

    #[test]
    fn parses_positions() {
        assert_eq!(Pos::parse("3, 2"), Some(Pos::new(3, 2)));
        assert_eq!(Pos::parse("0,-11"), None);
        assert_eq!(Pos::parse("4,0"), None);
        assert_eq!(Pos::parse("1"), None);
        assert_eq!(Pos::parse("3,1,3,1"), Some(Pos::new(3, 1)));
    }

    #[test]
    fn reads_buttons() {
        let data = sample();
        let amil = data.button("Amil").unwrap();
        assert_eq!(amil.name, "Call to Arms");
        assert_eq!((amil.buttonpos, amil.unbuttonpos), (Some(Pos::new(1, 2)), Some(Pos::new(2, 2))));
        assert_eq!((amil.hotkey, amil.unhotkey), (Some(Key::letter('C')), Some(Key::letter('W'))));
        let blizzard = data.button("AHbz").unwrap();
        assert_eq!(blizzard.researchbuttonpos, Some(Pos::new(0, 0)));
        assert_eq!(blizzard.researchhotkey, Some(Key::letter('B')));
        assert_eq!(data.button("Rhme").unwrap().tiers, 3);
        assert_eq!(data.button("Asas").unwrap().buttonpos, None);
        assert_eq!(data.button("amil").unwrap().name, "Call to Arms");
    }

    #[test]
    fn worker_cards() {
        let data = sample();
        let cards = data.cards(data.unit("hpea").unwrap());
        assert_eq!(cards.len(), 2);
        assert_eq!(
            layout(&cards[0]),
            [
                ("CmdMove", 0, 0),
                ("CmdStop", 1, 0),
                ("CmdHoldPos", 2, 0),
                ("CmdAttack", 3, 0),
                ("CmdPatrol", 0, 1),
                ("Amil", 1, 2),
                ("AHbu", 0, 2)
            ]
        );
        assert_eq!(cards[1].kind, CardKind::Build);
        assert_eq!(layout(&cards[1]), [("htow", 0, 2), ("hbar", 1, 0), ("CmdCancel", 3, 2)]);
    }

    #[test]
    fn hero_cards() {
        let data = sample();
        let cards = data.cards(data.unit("Hamg").unwrap());
        assert!(cards[0].buttons.iter().any(|b| b.id == "CmdSelectSkill"));
        assert_eq!(cards[0].at(Pos::new(0, 2)).next().unwrap().id, "AHbz");
        assert_eq!(cards[0].unplaced, ["AInv"]);
        assert_eq!(cards[1].kind, CardKind::Learn);
        assert_eq!(layout(&cards[1]), [("AHbz", 0, 0), ("CmdCancel", 3, 2)]);
        assert_eq!(cards[1].buttons[0].field, Field::Research);
    }

    #[test]
    fn building_cards() {
        let data = sample();
        let card = &data.cards(data.unit("htow").unwrap())[0];
        // No Move/Stop: buildings without a weapon have no commands.
        assert_eq!(layout(card), [("hpea", 0, 0), ("hkee", 0, 2), ("ARal", 3, 1)]);
        let card = &data.cards(data.unit("hbar").unwrap())[0];
        assert_eq!(layout(card), [("hfoo", 0, 0), ("Hamg", 0, 2), ("Rhme", 0, 0), ("ARal", 3, 1)]);
    }

    #[test]
    fn finds_melee_units() {
        let data = sample();
        assert_eq!(data.creates("Amil"), ["hmil", "hpea"]);
        let ids: Vec<&str> = data.melee_units().iter().map(|u| u.id.as_str()).collect();
        // Militia: no build menu, and Call to Arms shows Back to Work.
        let cards = data.cards(data.unit("hmil").unwrap());
        assert_eq!(cards.len(), 1);
        let amil = cards[0].buttons.iter().find(|b| b.id == "Amil").unwrap();
        assert_eq!((amil.pos, amil.field), (Pos::new(2, 2), Field::Un));
        // Militia comes in through Call to Arms.
        assert_eq!(ids, ["Hamg", "hbar", "hfoo", "hkee", "hmil", "hpea", "htow"]);
    }

    #[test]
    fn ancient_cards() {
        let func = "[CmdMove]\nButtonpos=0,0\n[CmdStop]\nButtonpos=1,0\n[CmdHoldPos]\nButtonpos=2,0\n\
                    [CmdAttack]\nButtonpos=3,0\n[CmdPatrol]\nButtonpos=0,1\n[ARal]\nButtonpos=3,1\n\
                    [Aro1]\nButtonpos=3,2\nUnbuttonpos=2,2\n[Aeat]\nButtonpos=0,2\n[Aall]\nButtonpos=0,0\n\
                    [etol]\nTrains=ewsp\n[ewsp]\nButtonpos=0,0\n";
        let data = parse(
            func,
            "[Aro1]\nHotkey=R\nUnhotkey=R\n",
            &[
                ["etol", "nightelf", "Aro1,Aeat,Aall", "_", "40", "1", "1"],
                ["etrp", "nightelf", "Aro1,Aeat", "_", "40", "1", "3"],
                ["ewsp", "nightelf", "_", "_", "270", "0", "0"],
            ],
            &[&["alias", "code"], &["Aro1", "Aroo"]],
        );
        let cards = data.cards(data.unit("etol").unwrap());
        // Rooted: no Attack/Stop on a building that trains, no Eat Tree, and
        // Root shows its Uproot side. Aall is hidden.
        assert_eq!(layout(&cards[0]), [("ewsp", 0, 0), ("ARal", 3, 1), ("Aro1", 2, 2)]);
        assert_eq!(cards[0].buttons[2].field, Field::Un);
        assert_eq!(cards[0].unplaced, ["Aall"]);
        assert_eq!(cards[1].kind, CardKind::Uprooted);
        assert_eq!(
            layout(&cards[1]),
            [
                ("CmdMove", 0, 0),
                ("CmdStop", 1, 0),
                ("CmdHoldPos", 2, 0),
                ("CmdAttack", 3, 0),
                ("CmdPatrol", 0, 1),
                ("Aeat", 0, 2),
                ("Aro1", 3, 2)
            ]
        );
        // A building with a weapon and nothing else to do keeps Attack/Stop.
        let card = &data.cards(data.unit("etrp").unwrap())[0];
        assert_eq!(layout(card), [("CmdStop", 1, 0), ("CmdAttack", 3, 0), ("Aro1", 2, 2)]);
    }
}
