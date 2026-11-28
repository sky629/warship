#!/usr/bin/env python3
"""아이콘을 `crates/client/assets/icons`에 만듭니다. ffmpeg가 필요합니다.

사용법: python3 scripts/assets/build_icons.py <Board Game Icons를 푼 폴더> <Game Icons를 푼 폴더>

아이콘은 종류의 색의 둥근 네모 위에 밝은 그림 문자를 놓은 것입니다. 대포, 배, 포탄 같은 그림 문자는 이 스크립트가 그리고,
나머지는 Kenney의 Board Game Icons와 Game Icons(CC0)의 그림 문자를 가져옵니다.
"""

import math
import sys
from pathlib import Path

from raster import Canvas, box, capsule, circle, mask_of, minus, moved, polygon, scaled, union

OUT = Path(__file__).resolve().parents[2] / "crates/client/assets/icons"
SIZE = 96
INK = (0.97, 0.94, 0.86)
SHADOW = (0.0, 0.0, 0.0)

# 종류마다의 바탕색입니다. 아이콘이 없을 때의 칸의 색과 같습니다.
TILES = {
    "weapons": (0.55, 0.25, 0.2),
    "gear": (0.2, 0.4, 0.5),
    "supplies": (0.25, 0.5, 0.3),
    "hulls": (0.35, 0.3, 0.55),
    "skills": (0.55, 0.45, 0.15),
    "trades": (0.5, 0.35, 0.2),
    "hire": (0.4, 0.4, 0.45),
}


def cannon(length, girth, at=0.5, tilt=14.0):
    """왼쪽 아래에서 오른쪽 위를 향하는 포신입니다. 가운데의 세로 자리가 `at`입니다."""
    c, s = math.cos(math.radians(tilt)), math.sin(math.radians(tilt))
    ax, ay = 0.5 - c * length / 2, at + s * length / 2
    bx, by = 0.5 + c * length / 2, at - s * length / 2
    barrel = capsule(ax, ay, bx, by, girth / 2, girth * 0.36)
    muzzle = capsule(bx - c * 0.03, by + s * 0.03, bx, by, girth * 0.5)
    knob = circle(ax - c * girth * 0.55, ay + s * girth * 0.55, girth * 0.24)
    return union(barrel, muzzle, knob)


def carriage(at=0.5):
    wheel = minus(circle(0.36, at + 0.2, 0.13), circle(0.36, at + 0.2, 0.045))
    return union(wheel, box(0.47, at + 0.13, 0.2, 0.035, 0.02, -8.0))


def gun(length, girth, count=1):
    """포신 `count`개를 위아래로 놓고 포가 하나를 붙입니다."""
    gap = girth * 1.25
    tops = [0.44 - gap * (count - 1) / 2 + gap * index for index in range(count)]
    return scaled(union(*[cannon(length, girth, top) for top in tops], carriage(tops[-1])), 0.8)


def swivel(count=1):
    """받침대 위의 가는 포신입니다."""
    gap = 0.11
    tops = [0.38 - gap * (count - 1) / 2 + gap * index for index in range(count)]
    post = union(capsule(0.5, tops[-1] + 0.05, 0.5, 0.8, 0.035), box(0.5, 0.82, 0.16, 0.035, 0.02))
    return scaled(union(*[cannon(0.6, 0.085, top, 10.0) for top in tops], post), 0.86)


def mortar(size):
    """위를 향하는 짧고 굵은 포신과 받침입니다."""
    barrel = capsule(0.42, 0.66, 0.42 + 0.2 * size, 0.66 - 0.3 * size, 0.13 * size, 0.11 * size)
    return union(barrel, box(0.45, 0.8, 0.27, 0.05, 0.03))


def bomb(cx, cy, r):
    fuse = capsule(cx + r * 0.6, cy - r * 0.75, cx + r * 1.15, cy - r * 1.35, r * 0.13)
    return union(circle(cx, cy, r), fuse, circle(cx + r * 1.2, cy - r * 1.45, r * 0.2))


def pellets(cx, cy, r):
    return union(circle(cx, cy - r * 1.1, r), circle(cx - r * 1.2, cy + r * 0.9, r), circle(cx + r * 1.2, cy + r * 0.9, r))


def rocket(cx, cy, length):
    c = s = math.sqrt(0.5)
    ax, ay, bx, by = cx - c * length / 2, cy + s * length / 2, cx + c * length / 2, cy - s * length / 2
    tip = polygon([(bx + c * 0.1, by - s * 0.1), (bx - s * 0.06, by - c * 0.06), (bx + s * 0.06, by + c * 0.06)])
    fins = polygon([(ax - s * 0.09, ay - c * 0.09), (ax + s * 0.09, ay + c * 0.09), (ax + c * 0.1, ay - s * 0.1)])
    return union(capsule(ax, ay, bx, by, 0.04), tip, fins)


def chain_shot():
    links = [circle(0.36 + 0.07 * index, 0.6 - 0.05 * index, 0.028) for index in range(5)]
    return union(circle(0.26, 0.68, 0.13), circle(0.74, 0.34, 0.13), *links)


def moon():
    return minus(circle(0.48, 0.5, 0.3), circle(0.6, 0.42, 0.26))


def boat(masts, lateen=False):
    """배의 옆모습입니다. 돛대가 `masts`개이고, `lateen`이면 세모꼴 돛입니다."""
    hull = polygon([(0.1, 0.62), (0.9, 0.62), (0.76, 0.8), (0.22, 0.8)])
    bowsprit = capsule(0.86, 0.62, 0.97, 0.52, 0.015)
    places = {1: [0.5], 2: [0.36, 0.62], 3: [0.27, 0.5, 0.73]}[masts]
    heights = {1: [0.46], 2: [0.38, 0.44], 3: [0.32, 0.44, 0.34]}[masts]
    width = {1: 0.17, 2: 0.115, 3: 0.095}[masts]
    parts = [hull, bowsprit]
    for x, tall in zip(places, heights):
        top = 0.6 - tall
        parts.append(capsule(x, 0.62, x, top - 0.04, 0.012))
        if lateen:
            parts.append(polygon([(x - width * 0.5, 0.57), (x + width * 1.1, 0.57), (x - width * 0.3, top)]))
        else:
            parts.append(box(x, top + tall / 2 - 0.02, width, tall / 2 - 0.02, 0.025))
    return union(*parts)


def pips(count):
    """아래쪽 가장자리에 놓는 등급의 점입니다."""
    return union(*[circle(0.5 + (index - (count - 1) / 2) * 0.09, 0.9, 0.028) for index in range(count)])


def drawn(shape):
    return ("drawn", shape)


def board(name, scale=0.72, lift=0.0):
    """Board Game Icons의 그림 문자입니다. `lift`만큼 위로 올려 놓습니다."""
    return ("board", name, scale, lift)


def game(name, scale=0.9):
    """Game Icons의 그림 문자입니다. 이 팩의 그림에는 둘레에 여백이 있어서 크게 놓습니다."""
    return ("game", name, scale, 0.0)


# 폴더, 파일 이름, 그림 문자들입니다. 그림 문자는 차례로 겹쳐 그립니다.
ICONS = [
    ("weapons", "deck_gun", [drawn(gun(0.62, 0.17))]),
    ("weapons", "swivel_gun", [drawn(swivel())]),
    ("weapons", "long_nine", [drawn(gun(0.82, 0.12))]),
    ("weapons", "carronade", [drawn(moved(gun(0.5, 0.25), -0.06, 0.0)), drawn(pellets(0.8, 0.24, 0.05))]),
    ("weapons", "mortar", [drawn(mortar(1.0)), drawn(bomb(0.76, 0.3, 0.085))]),
    ("weapons", "twin_deck_guns", [drawn(gun(0.62, 0.15, 2))]),
    ("weapons", "chain_gun", [drawn(swivel(3))]),
    ("weapons", "heavy_carronade", [drawn(moved(gun(0.5, 0.21, 2), -0.06, 0.0)), drawn(pellets(0.82, 0.2, 0.05))]),
    ("weapons", "siege_mortar", [drawn(mortar(1.25)), drawn(bomb(0.78, 0.26, 0.11))]),
    ("weapons", "fire_pot", [board("fire", 0.46, lift=0.2), drawn(union(circle(0.5, 0.72, 0.16), box(0.5, 0.57, 0.13, 0.03, 0.02)))]),
    ("weapons", "culverin", [drawn(gun(0.74, 0.15)), drawn(pips(2))]),
    ("weapons", "rocket_rack", [drawn(union(rocket(0.34, 0.5, 0.42), rocket(0.52, 0.52, 0.42), rocket(0.7, 0.54, 0.42)))]),
    ("weapons", "greek_fire", [board("fire", 0.8)]),
    ("weapons", "long_twelve", [drawn(gun(0.86, 0.14)), drawn(pips(3))]),
    ("weapons", "grand_battery", [drawn(gun(0.66, 0.13, 3))]),
    ("gear", "oak_planking", [board("resource_planks")]),
    ("gear", "extra_sails", [board("flag_triangle")]),
    ("gear", "shipwrights", [game("wrench")]),
    ("gear", "copper_sheathing", [board("shield")]),
    ("supplies", "repair_kit", [game("plus")]),
    ("supplies", "spare_canvas", [board("flag_square")]),
    ("supplies", "sandbags", [board("pouch")]),
    ("trades", "timber", [board("resource_lumber")]),
    ("trades", "salt", [board("rhombus")]),
    ("trades", "spice", [board("resource_wheat")]),
    ("trades", "silk", [board("award")]),
    ("skills", "patch_up", [game("plus")]),
    ("skills", "grapeshot", [drawn(pellets(0.5, 0.52, 0.13))]),
    ("skills", "full_sail", [game("fastForward")]),
    ("skills", "broadside", [board("exploding")]),
    ("skills", "iron_hull", [board("shield")]),
    ("skills", "field_repairs", [game("wrench")]),
    ("skills", "chain_shot", [drawn(chain_shot())]),
    ("skills", "night_run", [drawn(moon())]),
    ("hire", "sea_raider", [board("skull")]),
    ("hulls", "sloop", [drawn(boat(1)), drawn(pips(1))]),
    ("hulls", "lugger", [drawn(boat(1, lateen=True)), drawn(pips(1))]),
    ("hulls", "cutter", [drawn(boat(1, lateen=True)), drawn(pips(2))]),
    ("hulls", "brig", [drawn(boat(2)), drawn(pips(2))]),
    ("hulls", "corvette", [drawn(boat(2, lateen=True)), drawn(pips(2))]),
    ("hulls", "frigate", [drawn(boat(2)), drawn(pips(3))]),
    ("hulls", "tender", [drawn(boat(2)), drawn(pips(3)), game("plus", 0.3)]),
    ("hulls", "xebec", [drawn(boat(3, lateen=True)), drawn(pips(3))]),
    ("hulls", "merchantman", [drawn(boat(3)), drawn(pips(3)), board("dollar", 0.26)]),
    ("hulls", "galleon", [drawn(boat(3)), drawn(pips(4))]),
    ("hulls", "bulwark", [drawn(boat(3)), drawn(pips(4)), board("shield", 0.26)]),
    ("hulls", "bomb_ketch", [drawn(boat(2)), drawn(pips(4)), drawn(bomb(0.14, 0.2, 0.06))]),
    ("hulls", "man_of_war", [drawn(boat(3)), drawn(pips(5))]),
    ("hulls", "ship_of_the_line", [drawn(boat(3)), drawn(pips(5)), board("crown_a", 0.26)]),
]


def tile(folder):
    """종류의 색의 둥근 네모입니다. 위쪽이 밝고, 가장자리는 어둡습니다."""
    r, g, b = TILES[folder]
    canvas = Canvas(SIZE)
    canvas.fill(box(0.5, 0.5, 0.5, 0.5, 0.14), (r * 0.45, g * 0.45, b * 0.45))
    shade = lambda y: tuple(min(1.0, value * (1.25 - 0.45 * y)) for value in (r, g, b))
    canvas.fill(box(0.5, 0.5, 0.465, 0.465, 0.11), shade)
    return canvas


def main():
    if len(sys.argv) != 3:
        sys.exit(__doc__)
    sources = {
        "board": Path(sys.argv[1]) / "PNG/Double (128px)",
        "game": Path(sys.argv[2]) / "PNG/White/2x",
    }
    for folder, name, glyphs in ICONS:
        canvas = tile(folder)
        for glyph in glyphs:
            if glyph[0] == "drawn":
                canvas.fill(moved(glyph[1], 0.0, 0.025), SHADOW, 0.35)
                canvas.fill(glyph[1], INK)
            else:
                kind, picture, scale, lift = glyph
                size = round(SIZE * scale)
                # 작은 그림 문자는 오른쪽 위의 표시이고, 큰 그림 문자는 가운데에 놓습니다.
                left, top = (SIZE - size - 6, 6) if scale < 0.4 else ((SIZE - size) // 2, (SIZE - size) // 2 - round(SIZE * lift))
                mask = mask_of(sources[kind] / f"{picture}.png", size)
                canvas.stamp(mask, size, left, top + 2, SHADOW, 0.35)
                canvas.stamp(mask, size, left, top, INK)
        canvas.save(OUT / folder / f"{name}.png")
        print(f"icons/{folder}/{name}.png")


if __name__ == "__main__":
    main()
