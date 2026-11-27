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

## 3D models

`models/<folder>/<name>.glb` (binary glTF 2.0). The name follows the same rule as icons.

| Folder | One file for each | Built-in size to match |
|---|---|---|
| `models/hulls/` | hull (`sloop.glb`, `man_of_war.glb`, ...) | about 1.3 long. The game makes hulls with more HP larger. |
| `models/creeps/` | creep kind (`cutter.glb`, `brig.glb`, `sea_raider.glb`) | about 0.7 long. The game makes kinds with more HP larger. |
| `models/structures/` | `tower.glb`, `harbor.glb`, `base.glb`, `port.glb` | tower 1 wide and 2 high, harbor 2 x 2, base 2.4 x 2.4, port 1.6 x 1.6 |
| `models/shots/` | weapon (`deck_gun.glb`, `siege_mortar.glb`, `cutter_gun.glb`, ...) and attack kind (`shell.glb`, `scatter.glb`, `bomb.glb`, `flame.glb`) | about 0.3 across. The game makes the shots of stronger weapons larger. |

| `models/scenery/` | `island.glb` | radius 1, bottom at the water. The game scales it to the radius of each island. |

A shot uses the model of its weapon. If that file is missing it uses the model of its attack kind, and if that is missing too it is a ball. A shot model points the way it flies. A scatter shot is drawn as three small copies.

Rules for every model:

- The bow points along +X, up is +Y, and the bottom is at Y = 0 (the water).
- The origin is the middle of the model on the water.
- A material named `team` is replaced by the color of the team (red, blue, the neutral color, or gray for a sunk ship; orange for a fire shot). Every other material keeps its own look.
- The first scene of the file is used.

A missing model is drawn with the built-in shape.

## Textures

| File | Use |
|---|---|
| `textures/water.png` | The water. It repeats; one copy covers 8 x 8 tiles. Make it seamless. |
| `textures/island.png` | The built-in island shape. It is stretched once over the top of each island and wraps around the side. Not used when `models/scenery/island.glb` exists. |

Without these files the water and the islands are flat colors.

## Sounds and music

Ogg Vorbis files.

| File | Played when |
|---|---|
| `sounds/fire/<weapon>.ogg` | that weapon fires (`sounds/fire/deck_gun.ogg`, ...) |
| `sounds/fire_shell.ogg` | a plain shell is fired by a weapon without its own sound |
| `sounds/fire_scatter.ogg` | the same for a scatter shot |
| `sounds/fire_bomb.ogg` | the same for a bomb |
| `sounds/fire_flame.ogg` | the same for a fire shot |
| `sounds/blast.ogg` | a scatter shot or a bomb bursts |
| `sounds/sink.ogg` | a ship sinks |
| `sounds/wreck.ogg` | a structure is destroyed |
| `sounds/buy.ogg` | you spend gold |
| `sounds/refuse.ogg` | the server refuses your order |
| `sounds/level_up.ogg` | your ship gains a level |
| `sounds/victory.ogg`, `sounds/defeat.ogg` | the match ends |
| `music/start.ogg` | the start screen (loops) |
| `music/battle.ogg` | in a room (loops) |

Sounds with a place (firing, bursts, sinking, wrecks) are quieter the farther they are from the middle of the view, and silent beyond 28 tiles. At most 6 new sounds start in one frame. Music plays at 45% of the volume of the sounds.

A browser starts sound only after the first click or key press on the page.

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
