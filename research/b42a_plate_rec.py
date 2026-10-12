#!/usr/bin/env python3
"""B42a: start the court 8 doubles match from save slot 2 (as b42_intro_shots.py) and log the court intro's title
plate object per match frame: whether it exists, its fade/blink counter and flags, the name's alpha, the match
phase. Optionally press ✕ once the match frame passes <skip frame> to see what the skip ends.
Usage (under tools/pcsx2.sh): b42a_plate_rec.py <out.txt> [skip frame]"""
import os, struct, sys, time
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "tools"))
from pine import Pine
from pick import vpad
out = sys.argv[1]; skip = int(sys.argv[2]) if len(sys.argv) > 2 else -1
p = Pine()
f32 = lambda a: struct.unpack("<f", struct.pack("<I", p.read32(a)))[0]
p.load_state(2)
time.sleep(1.5)
vpad("sleep 2500", "press cross 150", "sleep 1200", *["press cross 150", "sleep 1200"] * 3, "press start 150", "sleep 3500")
t = time.time()
while not p.read32(0x422f80):
    if time.time() - t > 90: sys.exit("no match")
    if time.time() - t < 20: vpad("press cross 150", "sleep 2500")
    else: time.sleep(0.5)
log = open(out, "w"); last = None; t = time.time(); pressed = False
while time.time() - t < 40:
    gm = p.read32(0x422f80)
    if not gm: continue
    ph, fr = p.read8(gm + 0x55), p.read32(gm + 0x58)
    mgr = p.read32(0x43b1d8)
    pl = mgr and p.read32(mgr + 0x54)
    row = (ph, pl != 0)
    if pl:
        row += (p.read32(pl + 0x64), p.read8(pl + 0x6c), p.read8(pl + 0x6d), p.read8(pl + 0x80), f32(p.read32(pl + 0x50) + 0x5c),
                p.read8(mgr + 0xd94), p.read8(0x2eefa8), p.read32(0x2ef870))
    if row != last:
        print(fr, *row, file=log, flush=True); last = row
    if skip >= 0 and fr >= skip and not pressed:
        vpad("press cross 100"); pressed = True; print(fr, "pressed cross", file=log, flush=True)
    if ph == 2: break
print("court", p.read32(0x422f90), "players", p.read32(0x422fa4))
