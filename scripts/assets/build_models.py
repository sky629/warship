#!/usr/bin/env python3
"""Kenney의 Pirate Kit(CC0)에 있는 모형을 게임의 모형 규칙에 맞춰 `crates/client/assets/models`에 만듭니다.

사용법: python3 scripts/assets/build_models.py <Pirate Kit을 푼 폴더>

모형 하나는 원본 모형 여러 개를 놓아서 만듭니다. 만든 모형은 뱃머리가 +X이고, 바닥이 Y = 0이고,
가운데가 원점이며, 정해진 크기입니다. 돛과 깃발은 `team` 재질이어서 게임이 진영의 색을 입힙니다.
질감은 파일 안에 넣습니다.
"""

import json
import math
import struct
import sys
from pathlib import Path

OUT = Path(__file__).resolve().parents[2] / "crates/client/assets/models"

# 원본 모형에서 진영의 색을 받는 부분의 이름입니다.
TEAM_PARTS = ("sail", "flag")


def part(source, at=(0.0, 0.0, 0.0), scale=1.0, turn=0.0, team=False, sized=True):
    """원본 모형 하나를 놓는 자리입니다. `turn`은 Y축 둘레의 각도(도)입니다.
    `team`이면 전체가 진영의 색이고, `sized`가 아니면 모형의 크기를 잴 때 빼놓습니다."""
    return {"source": source, "at": at, "scale": scale, "turn": turn, "team": team, "sized": sized}


def ship(source):
    # 원본 배는 뱃머리가 +Z입니다.
    return [part(source, turn=90.0)]


def flag(source, at, turn=0.0):
    return part(source, at=at, turn=turn, team=True, sized=False)


TOWER = [part("tower-complete-small"), flag("flag-high", (0.0, 6.6, 0.0))]
HARBOR = [
    part("structure-platform-dock", scale=1.6),
    part("structure", at=(-0.7, 2.05, 0.0), scale=0.9),
    flag("flag", (1.3, 2.05, 1.2)),
]
BASE = [
    part("tower-complete-large", scale=1.2),
    part("tower-complete-small", at=(-3.4, 0.0, -3.4)),
    part("tower-complete-small", at=(-3.4, 0.0, 3.4)),
    part("tower-complete-small", at=(3.4, 0.0, -3.4)),
    part("tower-complete-small", at=(3.4, 0.0, 3.4)),
    part("castle-wall", at=(-3.4, 0.0, 0.0), scale=0.9),
    part("castle-wall", at=(3.4, 0.0, 0.0), scale=0.9),
    part("castle-wall", at=(0.0, 0.0, -3.4), scale=0.9, turn=90.0),
    part("castle-wall", at=(0.0, 0.0, 3.4), scale=0.9, turn=90.0),
    flag("flag-high", (0.0, 12.0, 0.0)),
]
PORT = [
    part("structure-platform-dock", scale=1.6),
    part("structure-roof", at=(0.0, 2.05, 0.0), scale=0.85),
    flag("flag-pennant", (1.5, 2.05, 1.5)),
]
ISLAND = [
    part("patch-sand-foliage"),
    part("patch-sand", turn=90.0),
    part("rocks-sand-b", at=(-0.6, 0.0, 0.4), scale=0.9),
    part("palm-detailed-bend", at=(1.8, 0.1, -1.2), scale=0.8, sized=False),
    part("palm-detailed-straight", at=(-1.0, 0.1, 2.2), scale=0.7, turn=120.0, sized=False),
]
BALL = [part("cannon-ball", team=True)]

# 폴더, 파일 이름, 놓을 것, X 방향의 크기, 바닥을 수면에 놓는지입니다.
MODELS = [
    ("hulls", "sloop", ship("ship-small"), 1.3, True),
    ("hulls", "lugger", ship("ship-small"), 1.3, True),
    ("hulls", "cutter", ship("ship-pirate-small"), 1.3, True),
    ("hulls", "brig", ship("ship-medium"), 1.3, True),
    ("hulls", "corvette", ship("ship-pirate-medium"), 1.3, True),
    ("hulls", "frigate", ship("ship-medium"), 1.3, True),
    ("hulls", "tender", ship("ship-medium"), 1.3, True),
    ("hulls", "xebec", ship("ship-pirate-medium"), 1.3, True),
    ("hulls", "merchantman", ship("ship-large"), 1.3, True),
    ("hulls", "galleon", ship("ship-large"), 1.3, True),
    ("hulls", "bulwark", ship("ship-large"), 1.3, True),
    ("hulls", "bomb_ketch", ship("ship-pirate-large"), 1.3, True),
    ("hulls", "man_of_war", ship("ship-large"), 1.3, True),
    ("hulls", "ship_of_the_line", ship("ship-large"), 1.3, True),
    ("creeps", "cutter", ship("ship-pirate-small"), 0.7, True),
    ("creeps", "brig", ship("ship-pirate-medium"), 0.7, True),
    ("creeps", "sea_raider", ship("ship-ghost"), 0.7, True),
    ("structures", "tower", TOWER, 1.0, True),
    ("structures", "harbor", HARBOR, 2.0, True),
    ("structures", "base", BASE, 2.4, True),
    ("structures", "port", PORT, 1.6, True),
    ("shots", "shell", BALL, 0.3, False),
    ("shots", "scatter", BALL, 0.3, False),
    ("shots", "flame", BALL, 0.3, False),
    ("shots", "bomb", [part("barrel")], 0.3, False),
    ("scenery", "island", ISLAND, 2.0, True),
]


def read_glb(path):
    data = path.read_bytes()
    length = struct.unpack_from("<I", data, 12)[0]
    doc = json.loads(data[20 : 20 + length])
    start = 20 + length
    size = struct.unpack_from("<I", data, start)[0]
    return doc, data[start + 8 : start + 8 + size]


def multiply(a, b):
    """4 x 4 행렬의 곱입니다. 행렬은 glTF처럼 열을 차례로 적은 16개의 수입니다."""
    return [sum(a[k * 4 + row] * b[col * 4 + k] for k in range(4)) for col in range(4) for row in range(4)]


def placement(at, scale, turn):
    c, s = math.cos(math.radians(turn)), math.sin(math.radians(turn))
    return [c * scale, 0, -s * scale, 0, 0, scale, 0, 0, s * scale, 0, c * scale, 0, at[0], at[1], at[2], 1]


def local(node):
    """원본의 노드가 가진 변환입니다. 원본에는 이동만 있습니다."""
    if "matrix" in node:
        return node["matrix"]
    if "rotation" in node or "scale" in node:
        raise ValueError(f"node {node.get('name')} has a rotation or a scale")
    x, y, z = node.get("translation", (0.0, 0.0, 0.0))
    return [1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, x, y, z, 1]


def corners(doc, node_index, matrix):
    """노드와 그 자식들의 메시의 테두리 상자의 꼭짓점을 `matrix`로 옮긴 자리입니다."""
    node = doc["nodes"][node_index]
    matrix = multiply(matrix, local(node))
    points = []
    if "mesh" in node:
        for primitive in doc["meshes"][node["mesh"]]["primitives"]:
            accessor = doc["accessors"][primitive["attributes"]["POSITION"]]
            low, high = accessor["min"], accessor["max"]
            for x in (low[0], high[0]):
                for y in (low[1], high[1]):
                    for z in (low[2], high[2]):
                        points.append([matrix[i] * x + matrix[4 + i] * y + matrix[8 + i] * z + matrix[12 + i] for i in range(3)])
    for child in node.get("children", []):
        points += corners(doc, child, matrix)
    return points


def build(kit, parts, size, grounded):
    texture = (kit / "Textures/colormap.png").read_bytes()
    out = {
        "asset": {"version": "2.0", "generator": "warship build_models.py"},
        "scene": 0,
        "scenes": [{"nodes": [0]}],
        "nodes": [{"name": "model", "children": []}],
        "meshes": [],
        "accessors": [],
        "bufferViews": [],
        "materials": [
            {"name": "colormap", "doubleSided": True, "pbrMetallicRoughness": {"baseColorTexture": {"index": 0}, "metallicFactor": 0.0}},
            {"name": "team", "doubleSided": True, "pbrMetallicRoughness": {"metallicFactor": 0.0, "roughnessFactor": 0.9}},
        ],
        "textures": [{"sampler": 0, "source": 0}],
        "samplers": [{"minFilter": 9987}],
        "images": [],
    }
    binary = bytearray()
    sized = []
    for placed in parts:
        doc, data = read_glb(kit / f"{placed['source']}.glb")
        base = {key: len(out[key]) for key in ("nodes", "meshes", "accessors", "bufferViews")}
        offset = len(binary)
        binary += data
        binary += b"\0" * (-len(binary) % 4)
        for view in doc["bufferViews"]:
            out["bufferViews"].append({**view, "buffer": 0, "byteOffset": view.get("byteOffset", 0) + offset})
        for accessor in doc["accessors"]:
            out["accessors"].append({**accessor, "bufferView": accessor["bufferView"] + base["bufferViews"]})
        team_meshes = {node["mesh"] for node in doc["nodes"] if "mesh" in node and node.get("name", "").startswith(TEAM_PARTS)}
        for index, mesh in enumerate(doc["meshes"]):
            primitives = []
            for primitive in mesh["primitives"]:
                attributes = {name: accessor + base["accessors"] for name, accessor in primitive["attributes"].items()}
                team = placed["team"] or index in team_meshes
                primitives.append({"attributes": attributes, "indices": primitive["indices"] + base["accessors"], "material": 1 if team else 0})
            out["meshes"].append({"name": mesh.get("name", ""), "primitives": primitives})
        for node in doc["nodes"]:
            copy = {key: value for key, value in node.items() if key in ("name", "translation", "matrix")}
            if "mesh" in node:
                copy["mesh"] = node["mesh"] + base["meshes"]
            if "children" in node:
                copy["children"] = [child + base["nodes"] for child in node["children"]]
            out["nodes"].append(copy)
        matrix = placement(placed["at"], placed["scale"], placed["turn"])
        holder = {"name": placed["source"], "matrix": matrix, "children": [root + base["nodes"] for root in doc["scenes"][doc.get("scene", 0)]["nodes"]]}
        out["nodes"][0]["children"].append(len(out["nodes"]))
        out["nodes"].append(holder)
        if placed["sized"]:
            for root in doc["scenes"][doc.get("scene", 0)]["nodes"]:
                sized += corners(doc, root, matrix)

    low = [min(point[axis] for point in sized) for axis in range(3)]
    high = [max(point[axis] for point in sized) for axis in range(3)]
    scale = size / (high[0] - low[0])
    middle = [(low[axis] + high[axis]) / 2 for axis in range(3)]
    lift = -low[1] if grounded else -middle[1]
    out["nodes"][0]["matrix"] = [scale, 0, 0, 0, 0, scale, 0, 0, 0, 0, scale, 0, -middle[0] * scale, lift * scale, -middle[2] * scale, 1]

    out["images"].append({"bufferView": len(out["bufferViews"]), "mimeType": "image/png"})
    out["bufferViews"].append({"buffer": 0, "byteOffset": len(binary), "byteLength": len(texture)})
    binary += texture
    binary += b"\0" * (-len(binary) % 4)
    out["buffers"] = [{"byteLength": len(binary)}]

    text = json.dumps(out, separators=(",", ":")).encode()
    text += b" " * (-len(text) % 4)
    total = 12 + 8 + len(text) + 8 + len(binary)
    return b"glTF" + struct.pack("<II", 2, total) + struct.pack("<I", len(text)) + b"JSON" + text + struct.pack("<I", len(binary)) + b"BIN\0" + bytes(binary)


def main():
    if len(sys.argv) != 2:
        sys.exit(__doc__)
    kit = Path(sys.argv[1]) / "Models/GLB format"
    for folder, name, parts, size, grounded in MODELS:
        target = OUT / folder / f"{name}.glb"
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(build(kit, parts, size, grounded))
        print(target.relative_to(OUT.parent))


if __name__ == "__main__":
    main()
