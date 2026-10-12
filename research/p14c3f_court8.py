#!/usr/bin/env python3
"""P14c3f: start a court-8 doubles match (the court with type 32, trg_BA-02) from save slot 2 and save P1's first serve
to slot 8 (`start`), then list slot 8's trigger creatures: type, +0xbc/+0xbd, +0x12c (active), +0x280, +0x281 and
the model weight *(*(*(+0x5c))+0xc)+0x5c (`read`). On PCSX2 copy 4 ✕ confirms.
Usage (under tools/pcsx2.sh): p14c3f_court8.py start|read"""
import os, struct, subprocess, sys, time, zipfile
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "tools"))
from pine import Pine
from pick import vpad

STATES = "/home/ryha/Emulation/saves/ps2/states/SCUS-97610 (72326E67).%02d.p2s"
if os.environ.get("HST_PCSX2"):
    common = subprocess.check_output(["git", "rev-parse", "--path-format=absolute", "--git-common-dir"], text=True).strip()
    STATES = f"{os.path.dirname(common)}-slots/pcsx2/s{os.environ['HST_PCSX2']}/PCSX2/sstates/" + os.path.basename(STATES)

if sys.argv[1] == "start":
    p = Pine()
    p.load_state(2)
    time.sleep(1.5)
    # character select: Ashley + the COMs' first picks → confirm screen → Select Court, right ×3 (court 8) → start
    vpad("sleep 2500", *["press cross 150", "sleep 1200"] * 4, "press start 150", "sleep 3000",
         "press down 150", "sleep 600", "press cross 150", "sleep 2000", *["press right 150", "sleep 700"] * 3,
         "press cross 150", "sleep 2000", "press up 150", "sleep 600", "press cross 150", "sleep 2500")
    t = time.time()
    while True:
        gm = p.read32(0x422f80)
        pl = gm and p.read32(gm + 0xa8)
        if pl and p.read8(gm + 0x55) == 2 and p.read8(pl + 0x3fa4) == 1:
            break
        if time.time() - t > 120: sys.exit("match didn't reach the serve")
        time.sleep(0.1)
    time.sleep(1)
    p.save_state(8)
    time.sleep(2)
    print("court", p.read32(0x422f90))

ram = zipfile.ZipFile(STATES % 8).read("eeMemory.bin")
u32 = lambda a: struct.unpack_from("<I", ram, a)[0]
f = lambda a: struct.unpack_from("<f", ram, a)[0]
print("court", u32(0x422f90), "players", u32(0x422fa4))
for a in range(0x100000, len(ram), 4):
    if u32(a) == 0x1d2180:
        c = u32(a + 0x5c)
        w = f(u32(u32(u32(c)) + 0xc) + 0x5c) if c else None
        print(hex(a), "type", ram[a + 0x50], "bc/bd", ram[a + 0xbc], ram[a + 0xbd], "active", ram[a + 0x12c],
              "280/281", ram[a + 0x280], ram[a + 0x281], "weight", w, "pos", [round(f(a + 0x1e0 + 4 * i), 2) for i in range(3)])
