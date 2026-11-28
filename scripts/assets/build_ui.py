#!/usr/bin/env python3
"""패널의 바탕, 시작 화면의 그림, 미니맵의 표식, 수면의 질감을 `crates/client/assets`에 만듭니다.

사용법: python3 scripts/assets/build_ui.py

그림은 모두 이 스크립트가 그립니다.
"""

import math
from pathlib import Path

from build_icons import boat
from raster import Canvas, box, circle, minus, moved, polygon, scaled, union

OUT = Path(__file__).resolve().parents[2] / "crates/client/assets"
WHITE = (1.0, 1.0, 1.0)
GOLD = (0.78, 0.62, 0.3)


def panel():
    """패널의 바탕입니다. 가장자리 12픽셀이 테두리의 자리이고, 보이는 테두리는 바깥 4픽셀 안에 있습니다."""
    size = 48
    canvas = Canvas(size)
    px = 1.0 / size
    canvas.fill(box(0.5, 0.5, 0.5, 0.5, 5 * px), (0.02, 0.03, 0.05), 0.95)
    canvas.fill(box(0.5, 0.5, 0.5 - px, 0.5 - px, 4 * px), GOLD)
    canvas.fill(box(0.5, 0.5, 0.5 - 2.5 * px, 0.5 - 2.5 * px, 3 * px), (0.3, 0.22, 0.1))
    canvas.fill(box(0.5, 0.5, 0.5 - 3.5 * px, 0.5 - 3.5 * px, 2.5 * px), lambda y: (0.06, 0.1 - 0.02 * y, 0.15 - 0.03 * y))
    canvas.save(OUT / "ui/panel.png")


def start():
    """시작 화면의 그림입니다. 밤바다와 달, 배의 그림자입니다."""
    canvas = Canvas(960, 540)
    ratio = 540 / 960
    sky = lambda y: (0.03 + 0.05 * y, 0.06 + 0.1 * y, 0.12 + 0.16 * y)
    canvas.fill(box(0.5, ratio / 2, 0.5, ratio / 2), sky)
    canvas.fill(circle(0.78, 0.14, 0.11), (0.5, 0.6, 0.7), 0.12)
    canvas.fill(circle(0.78, 0.14, 0.05), (0.95, 0.93, 0.8))
    sea = lambda y: (0.02, 0.07 - 0.03 * y, 0.12 - 0.05 * y)
    horizon = ratio * 0.66
    canvas.fill(box(0.5, (horizon + ratio) / 2, 0.5, (ratio - horizon) / 2), sea)
    # 배의 그림자는 왼쪽의 수평선 위에 놓습니다. 배의 그림에서 선체의 바닥은 세로 0.8입니다.
    size = 0.34
    ship = moved(scaled(boat(3), size), 0.2 - 0.5, horizon - 0.5 - (0.8 - 0.5) * size + 0.004)
    canvas.fill(ship, (0.01, 0.02, 0.04))
    # 달빛이 수면에 비친 자리입니다. 아래로 갈수록 넓고 옅습니다.
    for row in range(6):
        y = horizon + ratio * (0.03 + 0.05 * row)
        canvas.fill(box(0.78, y, 0.03 + 0.012 * row, 0.0022, 0.0022), (0.95, 0.93, 0.8), 0.5 - 0.07 * row)
    canvas.save(OUT / "ui/start.png")


def marks():
    """미니맵의 표식입니다. 게임이 색을 입히므로 흰색입니다."""
    diamond = polygon([(0.5, 0.04), (0.96, 0.5), (0.5, 0.96), (0.04, 0.5)])
    shapes = {
        "island": circle(0.5, 0.5, 0.48),
        "port": minus(diamond, polygon([(0.5, 0.3), (0.7, 0.5), (0.5, 0.7), (0.3, 0.5)])),
        "camp": minus(circle(0.5, 0.5, 0.46), circle(0.5, 0.5, 0.24)),
        "tower": diamond,
        "harbor": box(0.5, 0.5, 0.42, 0.42, 0.08),
        "base": polygon([(0.5 + 0.48 * math.sin(k * math.pi / 4) * (1 if k % 2 == 0 else 0.6),
                          0.5 - 0.48 * math.cos(k * math.pi / 4) * (1 if k % 2 == 0 else 0.6)) for k in range(8)]),
        "creep": circle(0.5, 0.5, 0.36),
        "ship": circle(0.5, 0.5, 0.46),
        "me": union(minus(circle(0.5, 0.5, 0.48), circle(0.5, 0.5, 0.34)), circle(0.5, 0.5, 0.2)),
    }
    for name, shape in shapes.items():
        canvas = Canvas(32)
        canvas.fill(shape, WHITE)
        canvas.save(OUT / f"ui/minimap/{name}.png")


def water():
    """수면의 질감입니다. 가로와 세로로 이어 붙여도 이음매가 없습니다."""
    size = 256
    canvas = Canvas(size)
    turn = 2.0 * math.pi
    for py in range(size):
        for px in range(size):
            x, y = px / size, py / size
            ripple = math.sin(turn * (3 * x + 2 * y) + 2.0 * math.sin(turn * 2 * y))
            ripple += 0.6 * math.sin(turn * (5 * y - 2 * x) + 1.5 * math.sin(turn * 3 * x))
            crest = max(0.0, ripple - 1.1)
            shade = 0.03 * ripple
            canvas.pixels[py * size + px] = [0.16 + shade + 0.25 * crest, 0.31 + shade + 0.25 * crest, 0.44 + shade + 0.2 * crest, 1.0]
    canvas.save(OUT / "textures/water.png")


def main():
    panel()
    marks()
    water()
    start()
    print("ui/panel.png ui/start.png ui/minimap/*.png textures/water.png")


if __name__ == "__main__":
    main()
