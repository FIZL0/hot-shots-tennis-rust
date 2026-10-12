#!/usr/bin/env python3
"""B42: start a doubles match from save slot 2 (pick.py's route, confirm button as argv) and screenshot the court intro
every ~0.4 s until the first serve, logging the match frame (gm+0x58) and phase (gm+0x55) per shot.
Usage (under tools/pcsx2.sh): b42_intro_shots.py <out dir> [confirm button, default circle] [court steps right]"""
import os, subprocess, sys, time
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "tools"))
from pine import Pine
from pick import vpad, T
out = sys.argv[1]; ok = sys.argv[2] if len(sys.argv) > 2 else "circle"
steps = int(sys.argv[3]) if len(sys.argv) > 3 else -1
p = Pine()
p.load_state(2)
time.sleep(1.5)
vpad("sleep 2500", f"press {ok} 150", "sleep 1200", *[f"press {ok} 150", "sleep 1200"] * 3, "press start 150", "sleep 3500")
if steps >= 0:  # confirm screen: down, Select Court, right ×n, confirm, up
    vpad("press down 150", "sleep 700", f"press {ok} 150", "sleep 2000",
         *[c for _ in range(steps) for c in ("press right 150", "sleep 900")], f"press {ok} 150", "sleep 2000",
         "press up 150", "sleep 700")
t = time.time()
while not p.read32(0x422f80):
    if time.time() - t > 25: sys.exit("no match")
    vpad(f"press {ok} 150", "sleep 2500")
log = open(f"{out}/log.txt", "w")
n = 0; t = time.time()
while time.time() - t < 60:
    gm = p.read32(0x422f80)
    if not gm: time.sleep(0.1); continue
    ph, fr = p.read8(gm + 0x55), p.read32(gm + 0x58)
    pl = p.read32(gm + 0xa8)
    if ph == 2 and pl and p.read8(pl + 0x3fa4) == 1: break
    f = f"{out}/{n:03d}.png"
    try: subprocess.run([f"{T}/screenshot.sh", f], check=True, capture_output=True)
    except subprocess.CalledProcessError: pass
    print(n, f"{time.time()-t:.2f}", ph, fr, file=log, flush=True)
    n += 1
print("court", p.read32(0x422f90), "shots", n)
