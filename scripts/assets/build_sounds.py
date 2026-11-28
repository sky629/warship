#!/usr/bin/env python3
"""효과음을 `crates/client/assets/sounds`에 만듭니다. ffmpeg가 필요합니다.

사용법: python3 scripts/assets/build_sounds.py <Interface Sounds를 푼 폴더>

발사음, 폭발음, 침몰음, 건물이 무너지는 소리, 레벨과 승패의 소리는 ffmpeg의 소리 생성기로 합성합니다.
구매와 거절의 소리는 Kenney의 Interface Sounds(CC0)에서 가져옵니다.
모든 소리는 가장 큰 자리가 -3 dB가 되도록 크기를 맞춥니다.
"""

import re
import subprocess
import sys
import tempfile
from pathlib import Path

OUT = Path(__file__).resolve().parents[2] / "crates/client/assets/sounds"
PEAK_DB = -3.0


def noise(color, secs):
    return f"anoisesrc=color={color}:duration={secs}:amplitude=1:seed=7"


def wave(expression, secs):
    return f"aevalsrc='{expression}':d={secs}:s=44100"


def notes(pitches, each, ring):
    """음을 차례로 울립니다. 음마다 배음 두 개를 더하고 `ring`의 빠르기로 줄어듭니다."""
    parts = []
    for index, hz in enumerate(pitches):
        start = index * each
        local = f"(t-{start})"
        tone = f"(sin(2*PI*{hz}*{local})+0.4*sin(2*PI*{hz * 2}*{local})+0.2*sin(2*PI*{hz * 3}*{local}))"
        parts.append(f"gte(t,{start})*{tone}*exp(-{ring}*{local})")
    return wave("0.3*(" + "+".join(parts) + ")", len(pitches) * each + 4.0 / ring)


# 이름, 입력들, 입력을 섞는 필터입니다. 필터의 결과는 [out]입니다.
SYNTH = [
    # 짧고 낮은 포성입니다.
    ("fire_shell", [noise("brown", 0.6), wave("sin(2*PI*(110-90*t)*t)*exp(-10*t)", 0.6)],
     "[0]lowpass=f=700,volume='exp(-8*t)':eval=frame[n];[n][1]amix=inputs=2:normalize=0[out]"),
    # 여러 알이 흩어지는 거친 소리입니다.
    ("fire_scatter", [noise("white", 0.35), wave("sin(2*PI*(160-200*t)*t)*exp(-16*t)", 0.35)],
     "[0]highpass=f=900,lowpass=f=5000,volume='0.7*exp(-13*t)':eval=frame[n];[n][1]amix=inputs=2:normalize=0[out]"),
    # 박격포의 깊고 긴 소리입니다.
    ("fire_bomb", [noise("brown", 0.9), wave("sin(2*PI*(70-40*t)*t)*exp(-5*t)", 0.9)],
     "[0]lowpass=f=350,volume='exp(-5*t)':eval=frame[n];[n][1]amix=inputs=2:normalize=0[out]"),
    # 불이 스치는 소리입니다.
    ("fire_flame", [noise("pink", 0.6)],
     "[0]highpass=f=500,lowpass=f=3500,volume='sin(PI*t/0.6)*sin(PI*t/0.6)':eval=frame[out]"),
    ("blast", [noise("brown", 1.1), wave("sin(2*PI*(80-50*t)*t)*exp(-6*t)", 1.1), noise("white", 0.2)],
     "[0]lowpass=f=900,volume='exp(-4*t)':eval=frame[n];[2]volume='0.5*exp(-30*t)':eval=frame[c];"
     "[n][1][c]amix=inputs=3:normalize=0[out]"),
    # 낮아지는 소리와 물이 끓는 소리입니다.
    ("sink", [noise("brown", 1.6), wave("sin(2*PI*(140-60*t)*t)*exp(-2.5*t)*0.6", 1.6)],
     "[0]lowpass=f=500,tremolo=f=11:d=0.8,volume='sin(PI*t/1.6)':eval=frame[n];[n][1]amix=inputs=2:normalize=0[out]"),
    # 돌이 무너지는 길고 낮은 소리입니다.
    ("wreck", [noise("brown", 1.8), noise("white", 1.8), wave("sin(2*PI*(60-15*t)*t)*exp(-3*t)", 1.8)],
     "[0]lowpass=f=400,volume='exp(-2*t)':eval=frame[n];[1]bandpass=f=1800:w=1500,tremolo=f=17:d=0.9,"
     "volume='0.4*exp(-3*t)':eval=frame[c];[n][c][2]amix=inputs=3:normalize=0[out]"),
    # 올라가는 두 음입니다.
    ("level_up", [notes([659.3, 987.8], 0.12, 6.0)], "[0]anull[out]"),
    # 올라가는 장조의 화음입니다.
    ("victory", [notes([523.3, 659.3, 784.0, 1046.5], 0.16, 2.5)], "[0]anull[out]"),
    # 내려가는 단조의 화음입니다.
    ("defeat", [notes([392.0, 311.1, 261.6, 196.0], 0.24, 2.5)], "[0]anull[out]"),
]

# 이름과 Interface Sounds의 파일입니다.
COPIED = [("buy", "confirmation_002.ogg"), ("refuse", "error_004.ogg")]


def run(args):
    return subprocess.run(["ffmpeg", "-hide_banner", "-y", *args], check=True, capture_output=True, text=True).stderr


def finish(raw, target):
    """가장 큰 자리를 `PEAK_DB`에 맞추고 Ogg Vorbis로 적습니다."""
    report = run(["-i", str(raw), "-af", "volumedetect", "-f", "null", "-"])
    peak = float(re.search(r"max_volume: (-?[\d.]+) dB", report).group(1))
    run(["-i", str(raw), "-af", f"volume={PEAK_DB - peak}dB", "-ac", "2", "-ar", "44100",
         "-c:a", "vorbis", "-strict", "-2", "-q:a", "5", str(target)])


def main():
    if len(sys.argv) != 2:
        sys.exit(__doc__)
    interface = Path(sys.argv[1]) / "Audio"
    OUT.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory() as scratch:
        raw = Path(scratch) / "raw.wav"
        for name, inputs, mix in SYNTH:
            args = []
            for source in inputs:
                args += ["-f", "lavfi", "-i", source]
            run([*args, "-filter_complex", mix, "-map", "[out]", str(raw)])
            finish(raw, OUT / f"{name}.ogg")
            print(f"sounds/{name}.ogg")
        for name, source in COPIED:
            finish(interface / source, OUT / f"{name}.ogg")
            print(f"sounds/{name}.ogg")


if __name__ == "__main__":
    main()
