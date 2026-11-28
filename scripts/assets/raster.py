"""그림을 코드로 그리는 도구입니다. 도형은 거리 함수(도형의 가장자리까지의 거리, 안쪽이 음수)이고, 좌표는 그림의 한 변이 1입니다.

파이썬의 표준 라이브러리만 씁니다. 다른 그림 파일을 읽을 때에만 ffmpeg를 씁니다.
"""

import math
import struct
import subprocess
import zlib


def circle(cx, cy, r):
    return lambda x, y: math.hypot(x - cx, y - cy) - r


def box(cx, cy, half_w, half_h, round_=0.0, turn=0.0):
    """가운데가 (cx, cy)인 네모입니다. `round_`는 모서리의 반지름이고 `turn`은 도는 각도(도)입니다."""
    c, s = math.cos(math.radians(turn)), math.sin(math.radians(turn))

    def distance(x, y):
        dx, dy = x - cx, y - cy
        px, py = abs(dx * c + dy * s) - (half_w - round_), abs(-dx * s + dy * c) - (half_h - round_)
        return math.hypot(max(px, 0.0), max(py, 0.0)) + min(max(px, py), 0.0) - round_

    return distance


def capsule(ax, ay, bx, by, ra, rb=None):
    """(ax, ay)에서 (bx, by)까지의 굵은 선입니다. 양 끝의 굵기가 다를 수 있습니다."""
    rb = ra if rb is None else rb
    vx, vy = bx - ax, by - ay
    length2 = vx * vx + vy * vy

    def distance(x, y):
        t = max(0.0, min(1.0, ((x - ax) * vx + (y - ay) * vy) / length2))
        return math.hypot(x - ax - vx * t, y - ay - vy * t) - (ra + (rb - ra) * t)

    return distance


def polygon(points):
    def distance(x, y):
        best, inside = 1e9, False
        for i, (ax, ay) in enumerate(points):
            bx, by = points[i - 1]
            ex, ey = bx - ax, by - ay
            t = max(0.0, min(1.0, ((x - ax) * ex + (y - ay) * ey) / (ex * ex + ey * ey)))
            best = min(best, math.hypot(x - ax - ex * t, y - ay - ey * t))
            if (ay > y) != (by > y) and x < ax + (y - ay) / (by - ay) * ex:
                inside = not inside
        return -best if inside else best

    return distance


def union(*shapes):
    return lambda x, y: min(shape(x, y) for shape in shapes)


def minus(shape, hole):
    return lambda x, y: max(shape(x, y), -hole(x, y))


def both(a, b):
    return lambda x, y: max(a(x, y), b(x, y))


def moved(shape, dx, dy):
    return lambda x, y: shape(x - dx, y - dy)


def scaled(shape, factor):
    """그림의 가운데를 기준으로 도형의 크기를 바꿉니다."""
    return lambda x, y: shape((x - 0.5) / factor + 0.5, (y - 0.5) / factor + 0.5) * factor


class Canvas:
    """한 변이 `size` 픽셀인 투명한 그림입니다."""

    def __init__(self, size, height=None):
        self.width, self.height = size, height or size
        self.pixels = [[0.0, 0.0, 0.0, 0.0] for _ in range(self.width * self.height)]

    def blend(self, index, color, alpha):
        if alpha <= 0.0:
            return
        pixel = self.pixels[index]
        out = alpha + pixel[3] * (1.0 - alpha)
        for channel in range(3):
            pixel[channel] = (color[channel] * alpha + pixel[channel] * pixel[3] * (1.0 - alpha)) / out
        pixel[3] = out

    def fill(self, shape, color, alpha=1.0):
        """도형의 안쪽을 칠합니다. `color`는 색 하나이거나, 세로 자리(0에서 1)를 받아 색을 주는 함수입니다."""
        for py in range(self.height):
            y = (py + 0.5) / self.width
            tint = color(py / self.height) if callable(color) else color
            for px in range(self.width):
                cover = 0.5 - shape((px + 0.5) / self.width, y) * self.width
                if cover > 0.0:
                    self.blend(py * self.width + px, tint, min(cover, 1.0) * alpha)

    def stamp(self, mask, size, left, top, color, alpha=1.0):
        """`size` x `size`의 불투명도 표를 (left, top) 픽셀에 `color`로 찍습니다."""
        for my in range(size):
            for mx in range(size):
                x, y = left + mx, top + my
                if 0 <= x < self.width and 0 <= y < self.height:
                    self.blend(y * self.width + x, color, mask[my * size + mx] * alpha)

    def save(self, path):
        rows = bytearray()
        for py in range(self.height):
            rows.append(0)
            for pixel in self.pixels[py * self.width : (py + 1) * self.width]:
                rows += bytes(round(max(0.0, min(1.0, value)) * 255) for value in pixel)
        def chunk(kind, data):
            return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data))
        header = struct.pack(">IIBBBBB", self.width, self.height, 8, 6, 0, 0, 0)
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", header) + chunk(b"IDAT", zlib.compress(bytes(rows), 9)) + chunk(b"IEND", b""))


def mask_of(path, size):
    """그림 파일을 `size` x `size`로 줄인 불투명도 표입니다. 흰 그림 문자를 색을 바꿔 찍을 때 씁니다."""
    command = ["ffmpeg", "-v", "error", "-i", str(path), "-vf", f"scale={size}:{size}:flags=lanczos", "-f", "rawvideo", "-pix_fmt", "rgba", "-"]
    raw = subprocess.run(command, check=True, capture_output=True).stdout
    return [raw[index * 4 + 3] / 255.0 for index in range(size * size)]
