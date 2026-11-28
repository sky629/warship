#!/usr/bin/env python3
"""배경 음악을 `crates/client/assets/music`에 만듭니다. ffmpeg가 필요합니다.

사용법: python3 scripts/assets/build_music.py <시작 화면의 곡> <방 안의 곡>

곡의 출처는 `crates/client/assets/CREDITS.md`에 있습니다. 두 곡의 평균 크기를 `MEAN_DB`에 맞춥니다.
게임은 음악을 효과음의 45% 크기로 틀므로, 이 크기에서 음악이 효과음보다 작게 들립니다.
"""

import re
import subprocess
import sys
from pathlib import Path

OUT = Path(__file__).resolve().parents[2] / "crates/client/assets/music"
MEAN_DB = -23.0


def run(args):
    return subprocess.run(["ffmpeg", "-hide_banner", "-y", *args], check=True, capture_output=True, text=True).stderr


def main():
    if len(sys.argv) != 3:
        sys.exit(__doc__)
    OUT.mkdir(parents=True, exist_ok=True)
    for name, source in zip(("start", "battle"), sys.argv[1:]):
        report = run(["-i", source, "-af", "volumedetect", "-f", "null", "-"])
        mean = float(re.search(r"mean_volume: (-?[\d.]+) dB", report).group(1))
        run(["-i", source, "-af", f"volume={MEAN_DB - mean}dB", "-ac", "2", "-ar", "44100",
             "-c:a", "vorbis", "-strict", "-2", "-q:a", "5", str(OUT / f"{name}.ogg")])
        print(f"music/{name}.ogg {MEAN_DB - mean:+.1f} dB")


if __name__ == "__main__":
    main()
