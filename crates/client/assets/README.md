# Client assets

Put a file here with the name below and build again. The client uses every file it finds and draws a placeholder for the rest. No code change is needed.

## Icons

`icons/<folder>/<name>.png`, square, 64 x 64 or larger. The name is the item name in lower case, with every character that is not a letter or a digit replaced by `_` (for example `Man-of-War` is `man_of_war`, `Twin Deck Guns` is `twin_deck_guns`).

| Folder | One file for each | Shown in |
|---|---|---|
| `icons/weapons/` | weapon | shop rows, mount slots |
| `icons/gear/` | gear item | shop rows, mount slots |
| `icons/supplies/` | supply | shop rows, mount slots |
| `icons/hulls/` | hull | Shipyard tab |
| `icons/skills/` | skill | skill slots |
| `icons/trades/` | trade good | Trade tab, mount slots |
| `icons/hire/` | hired creep kind | hire rows |

A missing icon is drawn as a square in the color of its kind with the initials of the name.

## Panels and font

| File | Use |
|---|---|
| `ui/panel.png` | Background of the top bar, the ship panel, the shop window, the scoreboard and the start panel. Nine-slice: the outer 12 pixels on each side are the border and are not stretched. |
| `ui/start.png` | Full-screen picture behind the start panel. It is stretched to the window. |
| `ui/logo.png` | Title picture at the top of the start panel, 96 pixels high. |
| `fonts/ui.ttf` | Font of all text. |

Without these files, panels are a flat dark color, the start screen has no picture and shows the title as text, and the text uses the built-in font.

## Sizes on screen

| Slot | Size in pixels |
|---|---|
| Shop row icon | 32 x 32 |
| Mount slot icon | 28 x 28 |
| Skill slot icon | 36 x 36 |

Colors, text sizes and panel sizes are in `src/ui.rs`.
