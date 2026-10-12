#!/usr/bin/env python3
"""B42b: start a match from save slot 2 (as b42_intro_shots.py) and log, every poll, the intro's banner state: match
frame (gm+0x58), phase (gm+0x55), the court presentation's banner flags (+0xd94 shown, +0xd95 model done), its
overlay's alpha/stage/count (+0x54 → +0x70/+0x7c/+0x78), and the scoreboard's own start banner (0x42d6c0 → +0x568 on,
+0x570 stage, +0x56c count). Screenshots while either banner is up and for ~3 s after phase 0.
Usage (under tools/pcsx2.sh): b42b_banner.py <out dir> [confirm button, default circle]"""
import os, subprocess, sys, time
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "tools"))
from pine import Pine
from pick import vpad, T
out = sys.argv[1]; ok = sys.argv[2] if len(sys.argv) > 2 else "circle"
os.makedirs(out, exist_ok=True)
p = Pine()
p.load_state(2)
time.sleep(1.5)
vpad("sleep 2500", f"press {ok} 150", "sleep 1200", *[f"press {ok} 150", "sleep 1200"] * 3, "press start 150", "sleep 3500")
t = time.time()
while not p.read32(0x422f80):
    if time.time() - t > 25: sys.exit("no match")
    vpad(f"press {ok} 150", "sleep 2500")
log = open(f"{out}/log.txt", "w")
n = 0; t = time.time(); end = None; last = None
while time.time() - t < 60:
    gm = p.read32(0x422f80)
    if not gm: time.sleep(0.05); continue
    ph, fr = p.read8(gm + 0x55), p.read32(gm + 0x58)
    pr = p.read32(0x43b1d8); ov = p.read32(pr + 0x54) if pr else 0; sb = p.read32(0x42d6c0)
    row = (ph, fr, p.read8(pr + 0xd94) if pr else -1, p.read8(pr + 0xd95) if pr else -1,
           *( (p.read32(ov + 0x70), p.read32(ov + 0x7c), p.read32(ov + 0x78)) if ov else (-1, -1, -1)),
           *( (p.read8(sb + 0x568), p.read8(sb + 0x570), p.read32(sb + 0x56c)) if sb else (-1, -1, -1)))
    if ph != 0 and end is None: end = time.time()
    if end and time.time() - end > 4: break
    shoot = fr > 280 and (end is None or time.time() - end < 4)
    if row != last or shoot:
        f = ""
        if shoot:
            f = f"{out}/{n:03d}.png"
            try: subprocess.run([f"{T}/screenshot.sh", f], check=True, capture_output=True)
            except subprocess.CalledProcessError: f = "fail"
            n += 1
        print(f"{time.time()-t:.2f}", *row, f, file=log, flush=True)
        last = row
print("court", p.read32(0x422f90), "shots", n)
