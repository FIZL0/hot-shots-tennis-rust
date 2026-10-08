#!/usr/bin/env python3
"""P3e7: start a doubles match from save slot 2 (saves P1's first serve to slot 8), then read the
gallery manager's marks and each walker's state from slot 8's RAM: who registered which slot at the match start
(walker +0x304 = the manager's count when it registered) and what count is left.
Usage (under tools/pcsx2.sh): p3e7_start.py [vpad commands on the confirm screen | read]"""
import os, struct, subprocess, sys, time, zipfile
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "tools"))
from pine import Pine
from pick import vpad

STATES = "/home/ryha/Emulation/saves/ps2/states/SCUS-97610 (72326E67).%02d.p2s"
if os.environ.get("HST_PCSX2"):
    common = subprocess.check_output(["git", "rev-parse", "--path-format=absolute", "--git-common-dir"], text=True).strip()
    STATES = f"{os.path.dirname(common)}-slots/pcsx2/s{os.environ['HST_PCSX2']}/PCSX2/sstates/" + os.path.basename(STATES)
p = Pine()
def start(confirm):
    """Slot 2's character select → Ashley + the COMs' first picks → the confirm screen (`confirm`: extra presses there,
    e.g. a court) → the match, waiting for P1's first serve; saved to slot 8. In copy 2 the pad's "cross" is ○."""
    p.load_state(2)
    time.sleep(1.5)
    vpad("sleep 2500", *["press cross 150", "sleep 1200"] * 4, "press start 150", "sleep 3000", *confirm)
    t = time.time()
    while not p.read32(0x422f80):
        if time.time() - t > 20: sys.exit("no match")
        time.sleep(0.1)
    while True:
        gm = p.read32(0x422f80)
        pl = gm and p.read32(gm + 0xa8)
        if pl and p.read8(gm + 0x55) == 2 and p.read8(pl + 0x3fa4) == 1:
            break
        if time.time() - t > 90: sys.exit("match didn't reach the serve")
        time.sleep(0.1)
    time.sleep(1)
    p.save_state(8)
    time.sleep(2)

if sys.argv[1:2] != ["read"]:
    start(sys.argv[1:] or ["press cross 150", "sleep 2500"])
ram = zipfile.ZipFile(STATES % 8).read("eeMemory.bin")
u32 = lambda a: struct.unpack_from("<I", ram, a)[0]
i32 = lambda a: struct.unpack_from("<i", ram, a)[0]
f = lambda a: struct.unpack_from("<f", ram, a)[0]
mgr = u32(0x43b1c0)
print("stage", u32(0x422f90), "players", u32(0x422fa4), "kind", i32(mgr + 0x8c4), "count", i32(mgr + 0x8c8),
      "run", ram[mgr + 0x1b61], "tick", i32(mgr + 0x1b74), "gm+0x54", ram[u32(0x422f80) + 0x54])
for k in range(8):
    s = mgr + 0x8d0 + k * 0x30
    print(" slot", k, [round(f(s + 4 * i), 3) for i in range(3)], "base", [round(f(s + 0x10 + 4 * i), 3) for i in range(3)],
          "delay", i32(s + 0x20), "n", i32(s + 0x24))
for a in range(0x100000, len(ram), 4):
    if u32(a) == 0x1d1de0:
        c = u32(a + 0x60)
        print(" walker", hex(a), "slot", i32(a + 0xbc), "counter", i32(a + 0xd0), "mode", i32(a + 0x210),
              "reg", i32(a + 0x304), "home", [round(f(u32(a + 0xd4) + 0x30 + 4 * i), 3) for i in range(3)])
