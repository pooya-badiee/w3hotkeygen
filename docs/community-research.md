# Community research (Oct 2026)

Sources: Blizzard forums, Hive Workshop, GitHub.

## Competing tools
| Tool | Status | Gaps |
|---|---|---|
| jcfieldsdev hotkey editor (web) | De facto standard, semi-maintained | Bugs open since 2024: false Root/Uproot conflicts, no toggle-state positions, case-sensitive codes, campaign variants missing |
| clemenscodes warcraft-hotkey-editor (Rust/WASM, Dioxus) | Very active, tiny user base | Reforged only, full-file export (no merge), per-unit conflict check only |
| Blizzard built-in Grid / 2.0 keybind menu | Built in | No custom maps, no tile repositioning, breaks on patches |
| W3Champions launcher | Active | Ladder-focused, must keep running, not a CustomKeys editor |
| Static grid files, AHK tools, Warkeys | Stale | Fixed layouts, break after patches |

## Pain points (ranked)
1. **Patches overwrite or break custom keys:** rawcodes get renamed and tiles move.
2. **Non-QWERTY layouts:** the QWERTZ Y/Z swap, and AZERTY users switching their OS layout.
3. **Silent collisions:** the Hotkey / Unhotkey / Researchhotkey fields confuse people.
4. **Tooltips show the old key** (Classic).
5. **Campaign, neutral and custom-map units** aren't covered.
6. **Inventory, control-group and quickcast keys** are ignored by most tools.
7. **No bulk operations,** such as swapping rows across all units.
8. **Install confusion:** the file path and the enable toggle.
9. **Toggle states** can't be given separate slots.

## Grid vs classic commands
- Blizzard's built-in Grid moved Attack from A to R. Players pressing A out of habit got move instead of attack-move, and the buttons still showed the old letters. ([forum](https://us.forums.blizzard.com/en/warcraft3/t/cant-attack-buildings-with-hotkeys/14296))
- The built-in grid is pure QWER and can't be mixed (e.g. grid + A for Attack). ([forum](https://us.forums.blizzard.com/en/warcraft3/t/hotkey-issues/15956))
- The usual fix is third-party tools (W3Champions, Warkeys, KeyCraft) or hand-editing. ([Hive](https://www.hiveworkshop.com/threads/easy-way-to-change-all-hotkeys-to-fit-qwer-grid.299493))
- The default numpad inventory keys are another common reason people reach for tools.
- This is the basis for our default layout, "Grid + classic commands" (see [plan.md](plan.md)).

## Where this fits
The full editors already exist. What's missing is the opposite: one layout, applied to every
unit, regenerated from the installed game so a patch can't leave it stale, with no conflicts to
sort out by hand. Pain points 1, 3 and 8 are the ones this targets; 2 is covered by setting the
grid letters by hand.
