#!/usr/bin/env python3
"""B193: screenshot the original <before> frames before the change of ends starts (the post-point phase, with the
last shot's red landing marker) — the same save state replays the same vsyncs, so a first pass finds the phase's
first vsync and a second presses the copy's pad-bound Screenshot hotkey (L3 + R3) that many frames earlier.
usage: b193_post_shot.py <slot> <before> <out.png>"""
import os, shutil, subprocess, sys, time
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "tools"))
from pine import Pine

snaps = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", "pcsx2", f"s{os.environ['HST_PCSX2']}",
                     "PCSX2", "snaps")
p = Pine(); slot, before, out = int(sys.argv[1]), int(sys.argv[2]), sys.argv[3]
gm = 0x422f80

def run():
    p.load_state(slot); time.sleep(1)
    g = p.read32(gm); last = p.read32(0x1d5780)
    return g, last

g, last = run()
while p.read8(g + 0x55) != 1:
    last = p.next_frame(last)
start = last
print("change of ends at vsync", start, flush=True)
g, last = run()
while last < start - before:
    last = p.next_frame(last)
seen = set(os.listdir(snaps))
press = subprocess.Popen(["tools/vpad.py", "send", "down l3", "down r3", "sleep 120", "up r3", "up l3"])
while press.poll() is None:
    print(last, "phase", p.read8(g + 0x55), flush=True)
    last = p.next_frame(last)
for _ in range(60):
    new = sorted(set(os.listdir(snaps)) - seen)
    if new:
        f = os.path.join(snaps, new[-1]); s = -1
        while s != os.path.getsize(f) or s == 0: s = os.path.getsize(f); time.sleep(0.5)
        shutil.copy(f, out); print("shot", out); break
    time.sleep(0.5)
