#!/usr/bin/env python3
"""무기마다 다른 포탄의 모형을 `crates/client/assets/models/shots`에 만듭니다.

사용법: python3 scripts/assets/build_shots.py

모형은 모두 이 스크립트가 공, 원뿔대, 판으로 만듭니다. 날아가는 쪽이 +X이고 가운데가 원점입니다.
`team` 재질은 게임이 쏜 진영의 색(화염 무기는 불의 색)을 입힙니다. 나머지 재질은 정해진 색입니다.
파일의 이름은 무기의 이름을 소문자로 바꾸고 글자와 숫자가 아닌 것을 `_`로 바꾼 것입니다.
"""

import json
import math
import struct
from pathlib import Path

OUT = Path(__file__).resolve().parents[2] / "crates/client/assets/models/shots"
SIDES = 12

# 재질의 이름과 색, 스스로 내는 빛입니다. `team`의 색은 게임이 바꿉니다.
MATERIALS = {
    "team": ((1.0, 1.0, 1.0), None),
    "iron": ((0.1, 0.1, 0.12), None),
    "brass": ((0.75, 0.55, 0.2), None),
    "wood": ((0.4, 0.26, 0.13), None),
    "clay": ((0.6, 0.33, 0.2), None),
    "ember": ((1.0, 0.55, 0.1), (1.0, 0.4, 0.05)),
}


def ball(material, r, at=(0.0, 0.0, 0.0), stretch=1.0):
    """공입니다. `stretch`는 X 방향으로 늘이는 배율입니다."""
    rows = SIDES // 2
    points, normals, faces = [], [], []
    for row in range(rows + 1):
        tilt = math.pi * row / rows
        for side in range(SIDES):
            turn = 2.0 * math.pi * side / SIDES
            n = (math.cos(tilt), math.sin(tilt) * math.cos(turn), math.sin(tilt) * math.sin(turn))
            points.append((at[0] + n[0] * r * stretch, at[1] + n[1] * r, at[2] + n[2] * r))
            length = math.hypot(n[0] / stretch, n[1], n[2])
            normals.append((n[0] / stretch / length, n[1] / length, n[2] / length))
    for row in range(rows):
        for side in range(SIDES):
            a, b = row * SIDES + side, row * SIDES + (side + 1) % SIDES
            faces += [(a, a + SIDES, b), (b, a + SIDES, b + SIDES)]
    return material, points, normals, faces


def tube(material, x0, r0, x1, r1, at=(0.0, 0.0)):
    """X축을 따라 놓인 원뿔대입니다. 반지름이 0인 끝은 뾰족합니다. `at`은 축의 (Y, Z) 자리입니다."""
    points, normals, faces = [], [], []
    slope = (r0 - r1) / (x1 - x0)
    for x, r in ((x0, r0), (x1, r1)):
        for side in range(SIDES):
            turn = 2.0 * math.pi * side / SIDES
            c, s = math.cos(turn), math.sin(turn)
            points.append((x, at[0] + c * r, at[1] + s * r))
            length = math.hypot(slope, 1.0)
            normals.append((slope / length, c / length, s / length))
    for side in range(SIDES):
        a, b = side, (side + 1) % SIDES
        faces += [(a, b, a + SIDES), (b, b + SIDES, a + SIDES)]
    # 양 끝의 뚜껑입니다.
    for x, r, facing in ((x0, r0, -1.0), (x1, r1, 1.0)):
        if r <= 0.0:
            continue
        middle = len(points)
        points.append((x, at[0], at[1]))
        normals.append((facing, 0.0, 0.0))
        for side in range(SIDES):
            turn = 2.0 * math.pi * side / SIDES
            points.append((x, at[0] + math.cos(turn) * r, at[1] + math.sin(turn) * r))
            normals.append((facing, 0.0, 0.0))
        for side in range(SIDES):
            a, b = middle + 1 + side, middle + 1 + (side + 1) % SIDES
            faces.append((middle, b, a) if facing > 0.0 else (middle, a, b))
    return material, points, normals, faces


def fins(material, x0, x1, reach, count=4):
    """축에서 바깥으로 뻗은 얇은 삼각 날개입니다. 앞쪽(x1)이 축에 붙고 뒤쪽(x0)이 `reach`까지 나옵니다."""
    points, normals, faces = [], [], []
    for index in range(count):
        turn = 2.0 * math.pi * index / count + math.pi / 4.0
        c, s = math.cos(turn), math.sin(turn)
        # 양쪽 면이 모두 보이게 면마다 꼭짓점을 따로 둡니다.
        for facing in (1.0, -1.0):
            base = len(points)
            points += [(x1, 0.0, 0.0), (x0, 0.0, 0.0), (x0, c * reach, s * reach)]
            normals += [(0.0, -s * facing, c * facing)] * 3
            faces.append((base, base + 1, base + 2) if facing > 0.0 else (base, base + 2, base + 1))
    return material, points, normals, faces


def band(x, r, width=0.03):
    """몸통을 두른 황동 띠입니다."""
    return tube("brass", x - width / 2, r, x + width / 2, r)


def flame(x, r, length):
    """뒤로 끌리는 불꽃입니다."""
    return [ball("ember", r, at=(x, 0.0, 0.0)), tube("ember", x - length, 0.0, x, r * 0.9)]


def shell(length, r):
    """뾰족한 긴 포탄입니다. 몸통은 진영의 색이고 머리는 쇠입니다."""
    half = length / 2
    return [
        tube("team", -half, r * 0.8, half - r * 2.0, r),
        tube("iron", half - r * 2.0, r, half, 0.0),
        band(-half + 0.05, r * 0.95),
    ]


def round_bomb(r, bands):
    """심지가 달린 둥근 폭탄입니다."""
    parts = [ball("iron", r), tube("brass", -0.03, r * 0.3, 0.03, r * 0.3, at=(r, 0.0)), ball("ember", r * 0.22, at=(0.0, r * 1.25, 0.0))]
    parts += [tube("team", x - 0.02, r * 1.02 * math.sqrt(1.0 - (x / r) ** 2), x + 0.02, r * 1.02 * math.sqrt(1.0 - (x / r) ** 2)) for x in bands]
    return parts


# 무기의 이름과 모형을 이루는 도형들입니다.
SHOTS = {
    # 기본 포의 둥근 포탄입니다.
    "Deck Gun": [ball("team", 0.15)],
    # 작은 납 알입니다.
    "Swivel Gun": [ball("team", 0.08), ball("iron", 0.05, at=(-0.1, 0.0, 0.0))],
    # 멀리 날아가는 긴 포탄입니다.
    "Long Nine": shell(0.5, 0.085),
    # 나란히 날아가는 포탄 두 발입니다.
    "Twin Deck Guns": [ball("team", 0.13, at=(0.0, 0.0, 0.15)), ball("team", 0.13, at=(0.0, 0.0, -0.15))],
    # 사슬로 이은 작은 알 두 개입니다.
    "Chain Gun": [
        ball("team", 0.07, at=(0.13, 0.0, 0.0)),
        ball("team", 0.07, at=(-0.13, 0.0, 0.0)),
        tube("iron", -0.13, 0.02, 0.13, 0.02),
    ],
    # 띠를 두른 큰 포탄입니다.
    "Culverin": [ball("team", 0.17), band(0.0, 0.172, 0.05)],
    # 날개가 달린 가장 긴 포탄입니다.
    "Long Twelve": shell(0.62, 0.095) + [fins("brass", -0.31, -0.12, 0.2)],
    # 한 줄로 날아가는 포탄 세 발입니다.
    "Grand Battery": [ball("team", 0.14, at=(x, 0.0, 0.0)) for x in (0.26, 0.0, -0.26)] + [tube("iron", -0.26, 0.03, 0.26, 0.03)],
    # 산탄의 알 하나입니다. 게임이 세 개를 함께 그립니다.
    "Carronade": [ball("team", 0.13), ball("iron", 0.07, at=(-0.14, 0.05, 0.0))],
    # 포도송이처럼 묶인 알입니다.
    "Heavy Carronade": [ball("team", 0.1, at=at) for at in ((0.08, 0.0, 0.0), (-0.07, 0.08, 0.05), (-0.07, -0.08, 0.05), (-0.07, 0.0, -0.1))],
    # 불꽃을 끌며 날아가는 로켓입니다.
    "Rocket Rack": [
        tube("team", -0.2, 0.06, 0.16, 0.06),
        tube("iron", 0.16, 0.06, 0.32, 0.0),
        fins("wood", -0.24, -0.06, 0.16),
    ]
    + flame(-0.22, 0.05, 0.22),
    # 심지가 달린 둥근 폭탄입니다.
    "Mortar": round_bomb(0.16, [0.0]),
    # 띠를 두 줄 두른 큰 폭탄입니다.
    "Siege Mortar": round_bomb(0.2, [-0.09, 0.09]),
    # 불이 붙은 질그릇입니다. 그릇의 목에서 불꽃이 나옵니다.
    "Fire Pot": [
        ball("clay", 0.14),
        tube("clay", -0.04, 0.07, 0.04, 0.07, at=(0.14, 0.0)),
        ball("team", 0.09, at=(0.0, 0.24, 0.0)),
        ball("team", 0.05, at=(-0.06, 0.34, 0.0)),
    ],
    # 길게 끌리는 불덩이입니다.
    "Greek Fire": [ball("team", 0.14, at=(0.1, 0.0, 0.0), stretch=1.3), tube("team", -0.42, 0.0, 0.1, 0.13)] + flame(-0.2, 0.06, 0.28),
}


def file_name(weapon):
    return "".join(letter if letter.isalnum() else "_" for letter in weapon.lower())


def build(parts):
    """도형들을 재질마다 한 묶음으로 모아서 glb 파일의 내용을 만듭니다."""
    names = [name for name in MATERIALS if any(part[0] == name for part in parts)]
    materials = []
    for name in names:
        color, glow = MATERIALS[name]
        material = {"name": name, "pbrMetallicRoughness": {"baseColorFactor": [*color, 1.0], "metallicFactor": 0.0, "roughnessFactor": 0.8}}
        if glow:
            material["emissiveFactor"] = list(glow)
        materials.append(material)
    binary = bytearray()
    views, accessors, primitives = [], [], []

    def store(data, target, kind, count, extra):
        views.append({"buffer": 0, "byteOffset": len(binary), "byteLength": len(data), "target": target})
        accessors.append({"bufferView": len(views) - 1, "componentType": kind, "count": count, **extra})
        binary.extend(data)
        binary.extend(b"\0" * (-len(binary) % 4))
        return len(accessors) - 1

    for index, name in enumerate(names):
        points, normals, faces = [], [], []
        for material, part_points, part_normals, part_faces in parts:
            if material != name:
                continue
            base = len(points)
            points += part_points
            normals += part_normals
            faces += [tuple(corner + base for corner in face) for face in part_faces]
        low = [min(point[axis] for point in points) for axis in range(3)]
        high = [max(point[axis] for point in points) for axis in range(3)]
        flat = lambda rows: struct.pack(f"<{len(rows) * 3}f", *(value for row in rows for value in row))
        position = store(flat(points), 34962, 5126, len(points), {"type": "VEC3", "min": low, "max": high})
        normal = store(flat(normals), 34962, 5126, len(points), {"type": "VEC3"})
        corners = [corner for face in faces for corner in face]
        indices = store(struct.pack(f"<{len(corners)}H", *corners), 34963, 5123, len(corners), {"type": "SCALAR"})
        primitives.append({"attributes": {"POSITION": position, "NORMAL": normal}, "indices": indices, "material": index})

    out = {
        "asset": {"version": "2.0", "generator": "warship build_shots.py"},
        "scene": 0,
        "scenes": [{"nodes": [0]}],
        "nodes": [{"name": "model", "mesh": 0}],
        "meshes": [{"name": "shot", "primitives": primitives}],
        "materials": materials,
        "accessors": accessors,
        "bufferViews": views,
        "buffers": [{"byteLength": len(binary)}],
    }
    text = json.dumps(out, separators=(",", ":")).encode()
    text += b" " * (-len(text) % 4)
    total = 12 + 8 + len(text) + 8 + len(binary)
    return b"glTF" + struct.pack("<II", 2, total) + struct.pack("<I", len(text)) + b"JSON" + text + struct.pack("<I", len(binary)) + b"BIN\0" + bytes(binary)


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    for weapon, parts in SHOTS.items():
        target = OUT / f"{file_name(weapon)}.glb"
        target.write_bytes(build(parts))
        print(target.relative_to(OUT.parents[1]))


if __name__ == "__main__":
    main()
