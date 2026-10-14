#!/usr/bin/env python3
"""B287: court 8 (jungle) in the original — a doubles match from save slot 2 (B42's route lands on court 8), saved at
the first serve to a scratch slot, a screenshot and the view as `HST_EYE` (eye, forward, down rows, tan).
Usage (under tools/pcsx2.sh): b287_shots.py <out dir> [save slot, default 8] [--load]  (--load: start from that slot)"""
import os, struct, subprocess, sys, time
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "tools"))
from pine import Pine
from pick import vpad, T
out = sys.argv[1]; slot = int(sys.argv[2]) if len(sys.argv) > 2 else 8
p = Pine()
f = lambda a: struct.unpack("<f", struct.pack("<I", p.read32(a)))[0]
if "--load" in sys.argv:
    p.load_state(slot); time.sleep(2)
else:
    p.load_state(2); time.sleep(1.5)
    vpad("sleep 2500", "press circle 150", "sleep 1200", *["press circle 150", "sleep 1200"] * 3, "press start 150", "sleep 3500")
    t = time.time()
    while not p.read32(0x422f80):
        if time.time() - t > 25: sys.exit("no match")
        vpad("press circle 150", "sleep 2500")
    t = time.time()
    while True:
        gm = p.read32(0x422f80); pl = gm and p.read32(gm + 0xa8)
        if gm and p.read8(gm + 0x55) == 2 and pl and p.read8(pl + 0x3fa4) == 1: break
        if time.time() - t > 90: sys.exit("no serve")
        time.sleep(0.2)
    time.sleep(1)
    p.save_state(slot); time.sleep(2)
print("court", p.read32(0x422f90))
subprocess.run([f"{T}/screenshot.sh", f"{out}/serve.png"], check=True)
v = [f(0x1e7d20 + 4 * i) for i in range(3)] + [f(0x1e7d30 + 4 * i) for i in range(3)] + [f(0x1e7d40 + 4 * i) for i in range(3)] + [f(0x1e7d64)]
print("HST_EYE=" + ",".join(f"{x:.6g}" for x in v))
