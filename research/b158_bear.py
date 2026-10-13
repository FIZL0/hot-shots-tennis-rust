#!/usr/bin/env python3
"""B158: court 6's bear in the original. From save slot 2 (main menu on the parallel copies) starts a singles match
(P1 Ashley vs COM Cody) with the court index poked to 6, screenshots the intro flyover, saves the first serve to
scratch slot 9. Then (`tb` argument) pokes a deciding-set tiebreak (tiebreak flag, a 1-set match) and screenshots
the next points. Usage (under tools/pcsx2.sh): b158_bear.py OUTDIR [tb]"""
import os, subprocess, sys, time
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "tools"))
from pine import Pine
from pick import vpad

COURT, GAME, TIEBREAK, SETS, MATCH = 0x422f90, 0x422f80, 0x31661a, 0x423048, 0x42d6c0
out = sys.argv[1]
os.makedirs(out, exist_ok=True)
T = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "tools")
shot = lambda n: subprocess.run([f"{T}/screenshot.sh", f"{out}/{n}.png"], stdout=subprocess.DEVNULL)
p = Pine()

def at_serve():
    gm = p.read32(GAME)
    pl = gm and p.read32(gm + 0xa8)
    return pl and p.read8(gm + 0x55) == 2 and p.read8(pl + 0x3fa4) == 1

def show(tag):
    print(tag, "tiebreak", p.read8(TIEBREAK), "sets", p.read32(SETS), "played", p.read32(p.read32(MATCH) + 0x460), flush=True)

if "tb" not in sys.argv:
    p.load_state(2)
    time.sleep(2)
    vpad("sleep 1500", "press triangle 150", "sleep 1500", "press cross 150", "sleep 2500", "press left 150", "sleep 800", "press cross 150", "sleep 2500",
         "press cross 150", "sleep 1500", "press cross 150", "sleep 2500", "press cross 150", "sleep 1500",
         "press cross 150", "sleep 1500")
    p.write32(COURT, 6)
    vpad("press start 150")
    for _ in range(80):  # the court is picked after the start: keep it 6 through the first seconds of loading
        p.write32(COURT, 6)
        time.sleep(0.1)
    for k in range(10):  # loading screen, then the flyover
        time.sleep(2.5)
        shot(f"intro{k:02}")
    print("court", p.read32(COURT))
    t = time.time()
    while not at_serve() and time.time() - t < 90:
        time.sleep(0.2)
    shot("serve")
    p.save_state(9)
    time.sleep(2)
    show("serve")
else:
    p.load_state(9)
    time.sleep(2)
    p.write32(SETS, 1)
    p.write32(TIEBREAK & ~3, p.read32(TIEBREAK & ~3) | 1 << 8 * (TIEBREAK & 3))
    show("poked")
    for k in range(10):  # serve, let the point play out, look at the next ones
        vpad("press cross 150", "sleep 700", "press cross 150", "sleep 700", "press cross 150", "sleep 2500")
        shot(f"tb{k:02}")
        show(k)
