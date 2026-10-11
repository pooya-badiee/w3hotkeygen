# CustomKeys.txt format

Research notes (Oct 2026). Verify uncertain items in-game.

## Location
- **Before 1.28:** `<install>\CustomKeys.txt`. Enable it in Options > Gameplay > "Custom Keyboard Shortcuts".
- **1.28+ and Reforged:** `Documents\Warcraft III\CustomKeyBindings\CustomKeys.txt`.
  - The folder may need creating.
  - macOS: `~/Library/Application Support/Blizzard/Warcraft III/CustomKeyBindings/`.
  - Watch for OneDrive-redirected Documents folders.
  - Under Wine/Proton it's the Documents folder inside the game's prefix.
  - How we pick the path: [install-detection.md](install-detection.md#user-files-custom_keys_path).
- **Reforged toggle:** Options > Input > Preset Keybindings = Custom.

## Syntax
- **As the game writes it (Reforged 3.0, checked against a real file):** UTF-8 without BOM, **LF** line endings, a blank line after each section, sections sorted case-insensitively.
- INI-like. Section headers are `[ID]`, a 4-char rawcode (`hpea`, `AHbz`, `Rhme`) or a `Cmd*` name.
- **Section names are matched case-insensitively (inferred).** The game writes about 150 sections in a different case from its data (`[afsh]` for `Afsh`, `[acmi]` for `ACmi`, `[AETL]` for `Aetl`, `[NChg]` for `nchg`), and never one section in two cases.
- Lines are `Key=Value`, and comments start with `//`. Field names are case-insensitive.
- **Hotkey fields:**
  - `Hotkey`
  - `Unhotkey`: the off state of toggles and morphs.
  - `Researchhotkey`: the hero learn menu.
  - Letters are uppercase. Numbers are Windows VK codes: 27 = Esc, 112-123 = F1-F12, 96-105 = numpad. The game writes letters as letters and everything else as codes.
  - 512 is a legacy Esc alias (uncertain).
  - We write `Hotkey=0` for "no key" (VK 0 is no key; a missing `Hotkey=` would fall back to the default). Unverified in-game.
- **Position fields:** `Buttonpos`, `Unbuttonpos`, `Researchbuttonpos` take `x,y`. x is 0-3 from the left, y is 0-2 from the top. The grid is a fixed 4x3.
- **Tooltip fields:** `Tip`, `Untip`, `Researchtip` (`%d` = level), `Revivetip`, `Awakentip`, `Ubertip`.
  - The highlight is `|cffffcc00X|r`.
  - Values can't contain quotes.
- **Multi-level values are comma-separated.** Upgrades need one token per tier (`Hotkey=S,S,S`).
- **Commands:**
  - `CmdMove`, `CmdAttack`, `CmdAttackGround`, `CmdStop`, `CmdHoldPos`, `CmdPatrol`
  - `CmdBuild*`, `CmdCancel*`, `CmdRally`, `CmdSelectSkill`, `CmdPurchase`
- **Reforged 2.0.x system bindings:**
  - Sections `itm1`-`itm6`, `her1`-`her3`, `Ctr0`-`Ctr9`, `Menu`, and so on.
  - Each has a class flag. The game writes `[itm1]`-`[itm6]` as `GameCommand=1,1,1` + `Hotkey=`. `Modifier=` is ignored on load.

## Engine quirks
- **Bindings are global per rawcode.** Two abilities on the same cell of one card collide, and the game slides one of them aside in an order it decides itself. Conflicts must be checked per unit, across every unit that carries the ability.
- **Toggle states:** `Buttonpos` and `Unbuttonpos` can be on different cards (morphs) or on the same card (the town hall's Call to Arms, `Amic`).
  - The Peasant's Call to Arms (`Amil`) is a morph: Back to Work (`Unbuttonpos`) is on the Militia's card (`hmil`), a separate unit.
  - Root (`Aro1`/`Aro2`): `Buttonpos` is Root (uprooted card), `Unbuttonpos` is Uproot (rooted card).
  - The Un key of `Amil`, `Amic` and `Astn` is tied to a pseudo-ID: `UnhotkeyId=Ami1`/`Ami2`/`Ast1`, plus a section like `[Ami1]` with `ParentId=Amil`. Blizzard's template calls `Ast1` the "Stone Form Unhotkey Helper". Probably the off-state is a separate command internally.
  - The in-game conflict check treats the helper as its own button: `Amic` with both states on one cell and key is reported as "Back to Work / Call To Arms (Town Hall)". Give the off-state its own cell, as Blizzard does. Plain same-card toggles (`Adef`) can share one.
- **The in-game conflict check covers the campaign heroes in the template** (`Hctl`, `Hctr`, `Hjsm`, `Hleo`, `Ugrk`, `Ugr2`, `Ugr3`, `Uany`), not just melee units. Their abilities live in `campaignabilityfunc.txt`; their talent versions (`AUv1`-`AUv3` for Grim Conviction `AUvg`) share the base's code in `abilitydata.slk`.
- **Duplicate template sections:** `[AUwf]` is in the template twice (`W`, then `Q`). Left unwritten, the game reported it clashing with Soul Lantern (`AUla`, `W`), so it probably takes the first.
- **Worker build buttons:** the game reads them from `AHbu` / `AObu` / `AUbu` / `AEbu`, not from `CmdBuild*`. Write both.
  - The data gives `AHbu` etc. a `Buttonpos` (0,2) but no `Hotkey`. The default `B` is only on `CmdBuild<Race>`. `ARal` / `CmdRally` work the same way (`Y`).
  - Militia (`hmil`) inherits the Peasant's `Builds=` list but has no build menu. The game writes `IgnoreBuilds=1` for it.
- **Irreversible morphs** read their key from the produced unit, e.g. `Aave` reads from `[ubsp]`. Consistent with the data: `Aave` has no default `Hotkey`, `ubsp` has `T`.
- **Hidden abilities** cause phantom conflicts: `Aall`, `Apxf` on `hphx`, and `Aenc`. `Aall` is on every shop (Arcane Vault, Voodoo Lounge, Ancient of Wonders). Abilities with no `Buttonpos` at all (`AInv`, `Abds`, `Aihn`…) and off-card positions (`Asas`: `0,-11`) are hidden too.
- **Case-mismatched IDs:** the 3.0 data defines only `ACvs`; `Acvs` appears just as a reference in `unitabilities.slk` (creep `nane`). Not a real pair of duplicates.
- **Reforged tooltips:** hero tooltips ignore customization, and the hotkey is auto-appended. Rewriting tips is mostly cosmetic.
- **Same-cell defaults:** some melee cards have two buttons on one cell in Blizzard's own data (Grunt/Demolisher, Siege Engine/Barrage). The list is in [game-files.md](game-files.md#quirks-found).
- **Buildings with a weapon:** see [game-files.md](game-files.md#quirks-found).
- **Hint fields:** the game copies these from its template (`war3.w3mod:customkeys.txt`) into the user's file; they aren't in the unit data. `OffensiveStructure=1` (towers, Ziggurats, Protector), `CanAttackGround=1` (artillery units, Cannon Tower, and the `Remg` upgrade), `Attackless=1` (Wisp), `CanPrioritizeAir=1` (Gargoyle), `IgnoreBuilds=1` (Militia), `MerchantAvailable=1` (73 items). Probably inputs to the game's conflict checker.
- **Multi-value positions:** the game wrote `Buttonpos=3,1,3,1` for `Nfir`. We read the first `x,y`.
- **2.0.0 (Nov 2024):** added an in-game keybind menu, and some Buttonpos values from older files stopped applying.
- **2.0.3 (Jul 2025):**
  - Many more bindable keys.
  - A Conflicts panel.
  - Grid supports non-QWERTY layouts.
  - Passive hotkeys removed.
- **Unverified:**
  - Spellbook inner-card positions.
  - Autocast `Unhotkey` semantics.
  - Whether shop item `Buttonpos` is honored. Blizzard's data overlaps them (several sold units at 0,0, some with no position); the generator assumes the game honors them and gives each ware its own cell.

## Data sources
- **Reforged:** `units\*func.txt`, the locale's `units\*strings.txt`, `units\abilityskin.txt` (`Art=` icon paths) and the SLKs, read through CASC. Details: [game-files.md](game-files.md#unit-and-ability-data-war3w3modunits).
- **Baselines:** `clemenscodes/warcraft-hotkey-editor` `templates/CustomKeys.txt` (modern), and `tlo9/Warcraft-3-Keybinds` (classic).
