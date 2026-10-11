# WC3 Grid Hotkeys: Plan

> Written before most of the code. Where it disagrees with the topic docs (listed in the [README](../README.md)), they win: game files are read with our own pure-Rust CASC reader, not casc-rs, and the UI uses the Reforged web UI art.

## 1. Product

**Minimal and global.** One screen, a few choices, one Apply button. Every choice applies to every unit. There is **no per-unit editing**.

The pitch: grid hotkeys that set themselves up correctly and keep working after patches.

### The layout idea: grid + classic commands
- **Abilities follow the grid:** the key is the button's position on the 4x3 command card (`QWER / ASDF / ZXCV`).
- **Basic commands keep their classic keys forever:** Attack A, Stop S, Move M, Hold H, Patrol P.
- Only A and S clash with the grid, so Attack goes in the A slot and Stop in the S slot. Grid and habit then agree. M, H and P aren't grid keys, so those three buttons sit in fixed free slots, and those slots' grid letters go unused on units.
- **The layout, as built** ([crates/core/src/grid.rs](../crates/core/src/grid.rs)):
  ```
  Q       W     E     R
  Attack  Stop  Move  Hold
  Patrol  X     C     Learn (heroes), Rally (buildings); V by default
  ```
  Abilities keep the top and bottom rows (7 slots, 6 on heroes), enough for every melee unit. Hero abilities learn on the same key they cast with.
- **Placement:** positions are global per rawcode, so each button gets one cell that's free on every melee card it shows on. It keeps its default cell (moved down a row on units, where Blizzard's commands sat on the top row) if that's free everywhere; else it takes the first free cell (top row, bottom row, then the rest). Buildings keep Blizzard's layout where they can. Units are placed before buildings. Shop wares are placed the same way; Select User keeps the corner.
- **Why:** Blizzard's built-in Grid moved Attack to R. Players kept pressing A, their units walked instead of attacking, and the built-in grid can't be mixed with classic keys (see [community-research.md](community-research.md#grid-vs-classic-commands)).

### The screen (as built)
```
                 WC3 GRID HOTKEYS
Grid            Commands               Inventory
[Q][W][E][R]    Attack [A]  Stop [S]   [T][Y]
[A][S][D][F]    Move   [M]  Hold [H]   [Num4][Num5]
[Z][X][C][V]    Patrol [P]  Learn [V]  [Num1][Num2]
                Rally  [V]

[ APPLY ] [ RESET ]  status line
      Warcraft III 3.0 · C:\...\Warcraft III  Change
```

| Panel | Keys |
|---|---|
| Grid | 12 key boxes, one per command card cell. Default QWERTY; other keyboards (QWERTZ, AZERTY) or any custom grid are set by hand. |
| Commands | 7 key boxes: Attack, Stop, Move, Hold, Patrol, Learn, Rally. |
| Inventory | 6 key boxes. Default numpad `7 8 / 4 5 / 1 2` (the game's own default; no letter can clash). A digit sets that numpad key. |

- **The user edits the key boxes by hand:** click a box, press a letter; Delete or Backspace clears it (no key, shown as `-`, written as `Hotkey=0`); Esc cancels. A cleared box never clashes. The screen opens with the keys currently in the user's file. The grid is read back from the buttons' cells and keys (a cell's key is the one 3 in 4 of its buttons use), so a hand-made file opens as QWERTY.
- **Clashes are shown straight away:** a key used twice turns both boxes red and disables Apply. A command may share its own cell's grid key (Attack on A): only buildings without that command put an ability there. Learn and Rally share the corner.
- The install (and other installs found) is a footnote with a Change link, so the panels get the room.
- **Single keys only:** the game ignores `Modifier=` on load, so Alt+Q and the like are impossible.
- **Default inventory** is the numpad `7 8 / 4 5 / 1 2`. Letters beside the grid (`T Y / G J / B N`) clashed with the ability keys of any file not fully on the grid, e.g. B for Bladestorm.
- Choices are saved in `config.ini`, so after a patch the user opens the app and presses Apply.

### Apply and Reset
- **Apply** generates the whole `CustomKeys.txt` from the installed game's data and the user's keys, and replaces the user's file (creating `CustomKeyBindings/` if it's missing). Sections it doesn't write (campaign units, tooltips) fall back to the game's defaults. It refuses to write if the generator reports a conflict. The screen opens with the keys currently in the file.
- **Reset** puts back the file from before the first Apply. The backup is `CustomKeys.txt.w3hk-backup`, written once and removed by Reset; an empty backup means there was no file, so Reset deletes it.
- The status line reminds the user to enable it: Options > Input > Preset Keybindings = Custom, and to restart the game. The game reads the file once at startup, and its own hotkey menu rewrites the whole file on Save or Reset.

Because the file is regenerated from the installed game every time, a patch never leaves it stale. That replaces the old merge/migration plan.

## 2. Build order
1. **Done:** install detection, CASC game files, themed window, `CustomKeys.txt` path, Windows CI and release workflow.
2. **Settings screen + writer for the non-data keys** (done: key boxes, clash check, Apply, Reset): commands (`CmdAttack`, `CmdStop`, `CmdMove`, `CmdHoldPos`, `CmdPatrol`) and inventory (`itm1`-`itm6`). Key boxes, clash check, backup, write. This needs no game data.
3. **Read game data** (done: melee roster, command cards, default positions and keys; see [game-files.md](game-files.md#unit-and-ability-data-war3w3modunits)): units, their abilities, and each button's default position, from `units\*func.txt`, `*strings.txt` and the SLKs via CASC.
4. **Grid generator** (done; works in-game) for abilities, including the quirks in [customkeys-format.md](customkeys-format.md#engine-quirks): toggles (`Unhotkey`/`Unbuttonpos`), hero learn (`Researchhotkey`), worker build menus (`AHbu` etc. plus `CmdBuild*`), irreversible morphs, hidden abilities, multi-tier upgrades.
5. **Keyboard layouts** (QWERTZ, AZERTY): done by hand in the Grid panel; presets later if wanted.
6. **Verify in-game** (done: the applied file works in-game). The edge cases under [Risks](#5-risks) are still untested one by one.
7. **Web version** (done): the same app on WASM at `/app`, with the result downloaded instead of written. See [web.md](web.md).
8. Later, if wanted: Classic (MPQ) support, campaign and neutral units.

## 3. Out of scope
- Per-unit editing, drag and drop, unit browser.
- Conflict panels and rule builders. The generator must not produce conflicts, and the key boxes catch the user's own.
- Importing and merging existing files, sharing presets.
- Tooltip rewriting. Reforged ignores it for heroes and adds the key itself.

## 4. Tech stack
```
crates/app     native egui exe: the screen, config, file IO (backup + write)
crates/assets  native only: install detection, CustomKeys path, game file reading
crates/core    pure, no IO: game data model (done) + generate(choices) -> String
```
- `crates/core` stays pure so a web version stays possible later.
- **Game data:** read from the user's install at runtime (never shipped), then handed to core.
- **Release profile:** `opt-level = "z"`, `lto = "fat"`, `codegen-units = 1`, `panic = "abort"`, `strip = true`.
- **Tests:** generator golden files and roundtrip checks in core, without a UI. CI runs on Windows.

## 5. Risks
- **Data accuracy:** wrong slot ordering on collisions erodes trust. Mitigate with in-game verification.
- **Patch churn:** new rawcodes or moved defaults. Regenerating from the installed game covers most of it, and new quirks need a rule.
- **Unverified engine behavior:** spellbook inner cards, autocast Unhotkey, shop item Buttonpos, `itm1`-`itm6` behavior. Needs manual testing.
- **Windows only.** No macOS, Linux or Wine build.

## 6. Open questions
1. Keep B for Build on workers in "Grid + classic commands"? Build is currently on the grid (Q on every worker).
2. Classic (1.26/1.31) support: later, or never?
