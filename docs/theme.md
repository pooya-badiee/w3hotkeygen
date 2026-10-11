# Theme (WC3 look)

Code: [crates/app/src/theme.rs](../crates/app/src/theme.rs). All art comes from the user's install (see [game-files.md](game-files.md)).

## Window
- Borderless, with a fixed size and no OS title bar. The board *is* the window.
- **Not transparent:** on Windows a transparent window gets a grey frosted-glass fill wherever we draw nothing, so the old shadow margin looked like glass. The window is now exactly the board.
- A soft drop shadow under the board.
- Drag anywhere that isn't a widget to move the window (`ViewportCommand::StartDrag`).
- Window controls drawn in game art:
  - Close: the gold X (`icons\close-icon-*`).
  - Minimize: a small blue button with a gold bar.
- **No flash of unstyled content:** eframe forces the window visible after the first frame, so the app shows only a dark clear color until the theme loads.
  - The wait is capped at 3s, after which the flat fallback is drawn.
  - When switching installs, the old theme stays up while the new one loads.

## Widgets
| Widget | Art | Slice (from game CSS) |
|---|---|---|
| `board` | `test_texture-wood.png` tiled + `war3_metal_frame_4k.png` | 600 slice, drawn at 200 (CSS uses 300 at 4k) |
| `card` | `wc3-hotkeys-menu-container.png` (4x3), `wc3-hotkeys-menu-items-container.png` (2x3) | none; cells are baked in, laid out per the game's CSS at 0.9 scale. Fewer rows crop the image |
| `card_panel` | grid card: title bar 3-sliced, body 9-sliced with only the outer frame | 12/5 CSS px |
| `key_cell`, `key_box`, `command_icon` | key in a card cell; key cap; icon in `portrait_frame_normal_4k` | n/a |
| `button` | `wc3_btn_blue_{normal,hover,down}_4k` in `button_frame_4k` | btn 50/110, frame 60 |
| `window_controls` | `close-icon-*`, `war3_btn_small_blue_*_4k` | image |
| `heading` | BlizQuadrata Bold, gold `#ffd428` + drop shadow | n/a |

- `nine_slice()` is CSS `border-image`. It shrinks the borders proportionally when the rect is too small.
- Textures use mipmaps, because the 4k art is drawn far below its native size.
- Every widget has a **flat fallback** (dark brown/blue/gold) for when there's no theme or a texture is missing.
- Font families: Proportional uses Friz Quadrata, and `theme::bold()` uses BlizQuadrata Bold. Both fall back to egui's defaults.

## Hotkey cards
- Grid, Commands and Inventory copy the game's own hotkey menu (`.HotkeysOptions-Details-Content` in `gluemanager.js`): 62px cells, gaps 3px/2px, title in the bar.
- Commands use `card_panel`: the grid card's title bar and outer frame around a black body (no cells), holding the icon (portrait frame) + key box pairs.
- Grid and Inventory cells show the key large and centred.
- Rejected for Commands: icons in card cells with the key in a corner tag, both as two rows and at their real cells on a full card.
- Clash: red border (the game's `.RedBorder`). Listening: gold border.

## Rejected looks
- **Homescreen background behind the board:** too busy; the frame alone looks better.
- **Hanging chains + clasps above the board:** cut, also too busy.
