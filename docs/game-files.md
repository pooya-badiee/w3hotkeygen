# Game files

Code: [crates/assets/src/game_files.rs](../crates/assets/src/game_files.rs).

## Reader: our own, in pure Rust
[crates/casc](../crates/casc) is a small read-only reader, so the same code runs natively and in the browser (see [web.md](web.md)). It reads through a `Source` trait: the file system on native, browser `File` handles on the web.

How Warcraft III's storage is read. The encoding table isn't needed, because TVFS entries point straight at encoded keys:
1. `.build.info`: the active row's `Build Key` → `Data/config/xx/yy/<key>`, the build config.
2. Build config: `vfs-root` and `vfs-N` (content key, encoded key). An entry whose key is a `vfs-N` key is a nested tree, so `war3.w3mod:units\x` is `units/x` inside the tree `war3.w3mod`.
3. `Data/data/XXyyyyyyyy.idx` (version 7; only the newest version per bucket counts): 9-byte encoded key → archive, offset, size.
4. `Data/data/data.###`: a 30-byte local header, then BLTE (plain `N` and zlib `Z` frames).
5. TVFS: a prefix tree of name fragments. Leaves point through the VFS and container file tables at an encoded key. Nested trees load the first time a name needs them.

- It opens in about 13 ms on 3.0.0.
- **Checked against CascLib** (C++, MIT, which this used before) byte for byte on 12,010 files: units, `enus` units, fonts and all web UI art. CascLib stays the reference for the format. Commit `fd8a09c` has the comparison tool (`examples/compare.rs`, `--features casclib`).
- **casc-rs (pure Rust, GPL-3.0): rejected.** It only sees the top-level TVFS (291 entries: binaries plus `war3.w3mod`). It doesn't descend into the nested `war3.w3mod` VFS where all game data lives, and its TVFS parser is private.
- **Classic MPQ is not supported.** Classic installs aren't detected. The candidate crate is `war3-mpq`.

## Path syntax
Paths look like `war3.w3mod:webui\baseframes\test_texture-wood.png`. They're case-insensitive, and the separator is a backslash.

| Prefix | Contents |
|---|---|
| `war3.w3mod:` | base data |
| `war3.w3mod:_hd.w3mod:` | HD overrides |
| `war3.w3mod:_de.w3mod:` etc. | locale overrides |

## Unit and ability data (`war3.w3mod:units\`)
Checked on 3.0.0. Parsers are in [crates/core/src/data.rs](../crates/core/src/data.rs) (with `ini.rs` and `slk.rs`); the IO is in [crates/assets/src/game_data.rs](../crates/assets/src/game_data.rs). See command cards with `cargo run -p w3hk-assets --example units [-- <ids> | --all]`.

| What | File | Notes |
|---|---|---|
| Positions: `Buttonpos`, `Unbuttonpos`, `Researchbuttonpos` | `units\*func.txt` (`human`/`orc`/`undead`/`nightelf`/`neutral`/`common`/`item`/`campaign` × `ability`/`unit`/`upgrade`, plus `itemfunc.txt`, `commandfunc.txt`) | Unit sections also hold `Trains`, `Builds`, `Researches`, `Upgrade`, `Sellunits`, `Sellitems`, `Makeitems`, `Revive` |
| Names, `Hotkey`, `Unhotkey`, `Researchhotkey` | `war3.w3mod:_locales\<locale>.w3mod:units\*strings.txt` | **Not** in `units\`. Default hotkeys are per locale |
| Which abilities a unit has | `units\unitabilities.slk`: `abilList`, `heroAbilList` | Implicit buttons aren't listed: build (`AHbu` etc.), Rally (`ARal`), Cancel, commands |
| Moves / attacks / building / race | `unitbalance.slk` (`spd`, `isbldg`), `unitweapons.slk` (`weapsOn`, `weapTp1/2`), `unitdata.slk` (`race`) | |
| Summons, morphs, ability base codes | `abilitydata.slk` (`code`, `UnitID<n>`, `Data<X><n>`) | |
| Art, models (not needed for keys) | `unitskin.txt`, `abilityskin.txt`, `upgradeskin.txt` | No positions or keys |
| Blizzard's default `CustomKeys.txt` | `war3.w3mod:customkeys.txt` | Hotkeys only, no positions. Covers every data version (legacy `Aprg` beside current `Apg2`) and the neutral melee buildings. The hint fields in game-written files (`OffensiveStructure`, `IgnoreBuilds`, `ParentId`…) are copied from here |

### How a unit's card is built
- **Commands:** Move, Hold and Patrol if the unit moves (`spd > 0` and not a building). Attack if `weapsOn != 0`, and Stop if it moves or attacks. Attack Ground if an enabled weapon is `artillery`/`aline`. Heroes get `CmdSelectSkill`.
- **Abilities** from `abilList` and `heroAbilList`, at `Buttonpos`. Hero learn menu: `heroAbilList` at `Researchbuttonpos`, plus `CmdCancel`.
- **Workers:** `Builds=` gives the build menu (each building's own `Buttonpos`, plus `CmdCancel`); the main card gets the race's build ability (`AHbu`/`AObu`/`AUbu`/`AEbu`).
- **Buildings:** trained units, researched upgrades and `Upgrade=` targets, each at its own `Buttonpos`, plus `ARal` if it trains or revives.
- **Melee roster** (306 units on 3.0): everything reachable from the town halls and the neutral buildings in the template (Tavern, Goblin Merchant, Goblin Laboratory, Shipyard, Dragon Roosts, Mercenary Camps) through Trains/Builds/Upgrade/Sellunits, plus units an ability summons or morphs into.

### Quirks found
- **Data sets:** current melee is base `units\` plus fields suffixed `:melee,V1`, which override the plain field (17 hotkeys, e.g. `nmsn`: `Hotkey=G`, `Hotkey:melee,V1=S`, plus a few tips). `_balance\melee_v0.w3mod` and `_balance\custom_v0/v1.w3mod` override base for the other data versions, as do `:melee,V0` / `:custom,V0` / `:custom,V1` fields (e.g. `Hotkey:melee,V0=512` on `CmdCancel`). Those are ignored, as are graphics suffixes (`:hd`, `:sd`, `:de`).
- **Field names vary in case** (`ButtonPos`, `UnButtonpos`, `order`), so read them case-insensitively. `Reserachbuttonpos` is misspelled on four campaign abilities (`AUwf`, `AUw1`-`AUw3`).
- **A section can be split across files** (`Ahrl` in common and undead, `Aall` in common and neutral). Merge field by field.
- **Off-card positions hide buttons:** `Asas` has `Buttonpos=0,-11`.
- **Rawcode case:** `unitabilities.slk` lists `Acvs` for `nane`, but only `ACvs` exists. Looked up case-insensitively, like the game.
- **No default hotkey in the strings** on `AHbu`/`AObu`/`AUbu`/`AEbu` (the key is `CmdBuild<Race>`'s `B`), on `ARal` (`Y` in the template and on `CmdRally`), or on `Aave` (`T` in the template and on `[ubsp]`).
- **Passive abilities** (auras, `Ahlh`) have a `Buttonpos` but no `Hotkey`. Hero auras still have a `Researchhotkey`.
- **Same-cell defaults** on melee cards, which the game resolves by sliding a button aside: Grunt/Demolisher (`ogru`/`ocat` 0,0), Headhunter/Berserker (`ohun`/`otbk` 1,0), Siege Engine/Barrage (`hmtt`/`hrtt` 2,0), Burrow/`orbr` in the Peon build menu (0,1), Destroyer's `Advm`/`Aabs` (0,2), Pandaren Storm (`Adsm`/`ANwk`) and Earth (`ACpv`/`ANta`). Several are pairs that never show together (an upgrade swaps one for the other).
- **Ancients:** Root (`Aro1`, `Aro2`; code `Aroo`) `Buttonpos` is the Root button on the uprooted card; `Unbuttonpos` is Uproot on the rooted card. Rooted weapons are set per Root ability (`DataA` "Rooted Weapons", `DataB` "Uprooted Weapons"), so the data says rooted ancients have a weapon.
- **Attack on buildings (inferred):** a building with a weapon that also trains, researches or sells (Halls of the Dead, Tree of Life) gets no Attack/Stop, since Blizzard's positions for those buildings sit on the Stop/Attack cells. Blizzard's template hand-marks only the towers, Ziggurats and the Protector `OffensiveStructure=1`.
- **Militia** (`hmil`) copies the Peasant's `Builds=` list, but has no build menu (the game writes `IgnoreBuilds=1` for it).
- **Morph forms:** the other form is in `UnitID<n>` (`Astn` -> `ugrm`, `Abrf` -> `edcm`), except Call to Arms (`Amil`: `DataB1=hmil`). On that unit the ability shows its `Unbuttonpos`.
- **Neutral shop positions overlap:** the Goblin Laboratory's `ngsp`, `nzep` and Reveal (`Andt`) all have `Buttonpos=0,0`, and `ngir` has none; the game slides colliding buttons aside. The generator gives every ware its own cell. Shops also carry Select User (`Aneu`/`Ane2`: `Buttonpos=0,0`, `Unbuttonpos=3,2`). It shows its off-state (inferred): Blizzard puts wares on 0,0 but never on 3,2.
- **Hidden abilities with a position:** `Aall`, `Apxf`, `Aenc` (and `Aall` on every shop). Buttons with no position at all (`AInv`, `Abds`, `Aihn`, `Ault`, `Awan`…) are hidden by having none.

### Checked against CustomKeys.txt files
`cargo test -p w3hk-assets --test reference -- --nocapture` compares every button on a melee card (686 on 3.0):
- **Blizzard's template** (`war3.w3mod:customkeys.txt`; the test fails on a mismatch): 598 of 601 hotkeys match. The rest are `ARal` and `Aave` (defaults only in the template), and a stray `Hotkey=G` line after `[hctw]` in the template.
- **Your own game-written file** (read only; differences listed, not failed on). On mine, positions differ a lot, which fits a custom layout (most abilities moved from row 2 to row 0, commands and research moved by hand), and hotkeys differ only where I'd set grid keys.

## Reforged web UI (`war3.w3mod:webui\`)
Reforged's menus are an HTML/JS app. The assets are mostly **PNG**: 7,129 PNG, 195 JPG and 63 WebM.
- `gluemanager.js` (6MB) holds all the CSS, which gives the exact `border-image` slice values. Extract it and search for a file name to find how the game uses it.
- `baseframes\`: wood, metal frames, chains, dividers, and `wc3-hotkeys-menu-container.png`, the game's own 4x3 command-card grid.
- `widgets\buttons\`: blue, green and red button states, button frames, checkboxes.
- `icons\`: UI icons, e.g. `close-icon-{resting,hover,pressed}.png`.
- `abilityicons\{sd,sd2,hd}\`: about 5,700 ability icons as PNG. These are useful for the command-card editor.
- `common\homescreen.jpg`: the rainy main-menu art.

## Fonts (`war3.w3mod:fonts\`)

| File | Family | Use |
|---|---|---|
| `blq85web.ttf` | BlizQuadrata Web Bold | headings and buttons (likely the game's `MasterFontBold`) |
| `blq55.ttf` | BlizQuadrata | probably `MasterFont` |
| `frizqt__.ttf` | Friz Quadrata TT | body text, classic WC3 |
| `bl.ttf` | BlizzardGlobal | CJK and other scripts (7MB) |

## Classic UI art (not used yet)
BLP textures under `UI\Widgets\...` and `UI\Console\{Race}\...`. See `UI\war3skins.txt` for the race skin keys.
