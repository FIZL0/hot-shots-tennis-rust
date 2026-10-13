#!/usr/bin/env python3
"""B73: screenshot the original mid change of ends. Loads <slot>, waits for change-ends tick <tick> (gm+0x58) and
presses the copy's pad-bound Screenshot hotkey (L3 + R3) right then (tools/screenshot.sh first tries F8 and waits
~3 s, longer than the 81-tick phase); the PNG lands in the copy's snaps folder, copied to <out.png>. Prints the vsync,
tick, marker flag (balloon manager +0x725) and HUD mode for each frame of the press.
usage: b73_hold_shot.py <slot> <tick> <out.png>"""
import os, shutil, struct, subprocess, sys, time
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "tools"))
from pine import Pine
from b73_marker_ends import find  # noqa: E402

snaps = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", "pcsx2", f"s{os.environ['HST_PCSX2']}",
                     "PCSX2", "snaps")
before = set(os.listdir(snaps))
p = Pine(); p.load_state(int(sys.argv[1])); time.sleep(1)
tick, out = int(sys.argv[2]), sys.argv[3]
gm = p.read32(0x422f80); bm = find(p); hud = p.read32(0x42d6c0)
last = p.read32(0x1d5780)
while True:
    last = p.next_frame(last)
    if p.read8(gm + 0x55) == 1 and p.read32(gm + 0x58) >= tick: break
press = subprocess.Popen(["tools/vpad.py", "send", "down l3", "down r3", "sleep 120", "up r3", "up l3"])
while press.poll() is None:
    print(last, "t", p.read32(gm + 0x58), "phase", p.read8(gm + 0x55), "marker", p.read_block(bm + 0x724, 4)[1],
          "hud", p.read8(hud + 0x148), flush=True)
    last = p.next_frame(last)
for _ in range(60):
    new = sorted(set(os.listdir(snaps)) - before)
    if new:
        f = os.path.join(snaps, new[-1]); s = -1
        while s != os.path.getsize(f) or s == 0: s = os.path.getsize(f); time.sleep(0.5)
        shutil.copy(f, out); print("shot", out); break
    time.sleep(0.5)
