# Asset credits

Every file in this folder is either made by a script in `scripts/assets/` or comes from a source below. Every source is Creative Commons Zero (CC0, public domain: https://creativecommons.org/publicdomain/zero/1.0/) except the font, which is under the SIL Open Font License 1.1.

| Files | Source | Made by |
|---|---|---|
| `models/` | Pirate Kit 2.1 by Kenney (https://kenney.nl/assets/pirate-kit), rearranged to the model rules of the game | `scripts/assets/build_models.py` |
| `sounds/buy.ogg`, `sounds/refuse.ogg` | Interface Sounds by Kenney (https://kenney.nl/assets/interface-sounds) | `scripts/assets/build_sounds.py` |
| the other files in `sounds/` | synthesized, no outside material | `scripts/assets/build_sounds.py` |
| `icons/` | pictograms from Board Game Icons and Game Icons by Kenney (https://kenney.nl/assets/board-game-icons, https://kenney.nl/assets/game-icons); cannons, ships, shots and tiles are drawn by the script | `scripts/assets/build_icons.py` |
| `ui/`, `textures/water.png` | drawn by the script, no outside material | `scripts/assets/build_ui.py` |
| `music/start.ogg` | Pirate Game Tune by Tozan (https://opengameart.org/content/pirate-game-tune), unchanged | copied |
| `music/battle.ogg` | Boss Battle Music by Juhani Junkala / SubspaceAudio (https://opengameart.org/content/boss-battle-music), the seamless loop | `ffmpeg -i <wav> -ac 2 -ar 44100 -c:a vorbis -strict -2 -q:a 5 battle.ogg` |
| `fonts/ui.ttf` | Fira Sans Medium, Copyright (c) 2012-2015 The Mozilla Foundation and Telefonica S.A. (https://github.com/google/fonts/tree/main/ofl/firasans), unchanged. SIL Open Font License 1.1; the license text is `fonts/OFL.txt` and stays next to the font. | copied |

`scripts/assets/raster.py` holds the drawing code that the icon and UI scripts share.
