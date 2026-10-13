#!/usr/bin/env python3
"""B78: screenshots of the original's umpire from a save state (default 5): two mid-rally (head following the ball)
and one 40 ticks into each turn after a point. usage: b78_shots.py OUTDIR [slot] [turns]"""
import os, subprocess, struct, sys, time
sys.path.insert(0, "tools")
from pine import Pine
out = sys.argv[1]; slot = int(sys.argv[2]) if len(sys.argv) > 2 else 5; turns = int(sys.argv[3]) if len(sys.argv) > 3 else 2
os.makedirs(out, exist_ok=True)
p = Pine()
shot = lambda name: subprocess.run(["tools/screenshot.sh", f"{out}/{name}.png"], check=True)
p.load_state(slot); time.sleep(1)
act = p.read32(0x43b1c8)
f = lambda a: struct.unpack("<f", struct.pack("<I", p.read32(a)))[0]
for k in range(2):
    time.sleep(3)
    print("rally", k, "target", [round(f(act + 0xc0 + 4 * i), 2) for i in range(3)]); shot(f"rally{k}")
got = 0
t0 = time.time()
while got < turns and time.time() - t0 < 300:
    if p.read32(act + 0xd0) != 0:
        pl = p.read32(act + 0x60)
        while f(pl + 0x38) < 40: time.sleep(0.01)
        print("turn", got, "motion", p.read32(act + 0xd0), "t", f(pl + 0x38)); shot(f"turn{got}")
        got += 1
        while p.read32(act + 0xd0) != 0: time.sleep(0.05)
    time.sleep(0.02)
