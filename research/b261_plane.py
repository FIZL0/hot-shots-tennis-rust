#!/usr/bin/env python3
"""B261: court 2's plane (trigger type 6) in the original. From save slot 2 (main menu on the parallel copies) starts
a singles match with the court index poked to 2 and saves the first serve to scratch slot 9 (b158_bear.py's menu
path). Then (`fly` argument) from slot 9 serves points while holding the plane's conditions true (result kind
0x4230b8 = 1 game won, point kind scoreboard +0x560 = 1 ace) and screenshots every 0.25 s after each point.
Usage (under tools/pcsx2.sh): b261_plane.py OUTDIR [fly]"""
import math, os, subprocess, sys, time, threading
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "tools"))
from pine import Pine
from pick import vpad

COURT, GAME, RESULT, MATCH = 0x422f90, 0x422f80, 0x4230b8, 0x42d6c0
out = sys.argv[1]
os.makedirs(out, exist_ok=True)
T = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "tools")
shot = lambda n: subprocess.run([f"{T}/screenshot.sh", f"{out}/{n}.png"], stdout=subprocess.DEVNULL)
p = Pine()

def at_serve():
    gm = p.read32(GAME)
    pl = gm and p.read32(gm + 0xa8)
    return pl and p.read8(gm + 0x55) == 2 and p.read8(pl + 0x3fa4) == 1

if "fly" not in sys.argv:
    p.load_state(2)
    time.sleep(2)
    vpad("sleep 1500", "press triangle 150", "sleep 1500", "press cross 150", "sleep 2500", "press left 150", "sleep 800", "press cross 150", "sleep 2500",
         "press cross 150", "sleep 1500", "press cross 150", "sleep 2500", "press cross 150", "sleep 1500",
         "press cross 150", "sleep 1500")
    p.write32(COURT, 2)
    vpad("press start 150")
    for _ in range(80):
        p.write32(COURT, 2)
        time.sleep(0.1)
    t = time.time()
    while not at_serve() and time.time() - t < 120:
        time.sleep(0.2)
    print("court", p.read32(COURT), "serve", at_serve(), flush=True)
    shot("serve")
    p.save_state(9)
    time.sleep(2)
else:
    import zipfile, struct
    sys.path.insert(0, T)
    os.environ.setdefault("HST_PCSX2", "3")
    common = subprocess.run(["git", "rev-parse", "--path-format=absolute", "--git-common-dir"], capture_output=True, text=True).stdout.strip()
    states = f"{os.path.dirname(common)}-slots/pcsx2/s{os.environ['HST_PCSX2']}/PCSX2/sstates/SCUS-97610 (72326E67).09.p2s"
    ram = zipfile.ZipFile(states).read("eeMemory.bin")
    objs = [a for a in range(0x100000, len(ram), 4) if struct.unpack_from("<I", ram, a)[0] == 0x1d2180 and ram[a + 0x50] == 6]
    print("plane objects", [hex(a) for a in objs], flush=True)
    plane = objs[0]
    p.load_state(9)
    time.sleep(2)
    m = p.read32(MATCH)
    log = open(f"{out}/plane.txt", "w")
    f32 = lambda a: struct.unpack("<f", struct.pack("<I", p.read32(a)))[0]
    for k in range(3):
        vpad("press cross 150", "sleep 700", "press cross 150", "sleep 700", "press cross 150")
        t, n, last = time.time(), 0, 0
        while time.time() - t < 14:
            if p.read8(m + 0x426) == 0:
                p.write32(RESULT, 1)
                p.write32(m + 0x560, 1)
            if p.read8(plane + 0x281):
                xyz = [round(f32(plane + 0x1e0 + 4 * i), 2) for i in range(3)]
                log.write(f"{k} {time.time() - t:.2f} {xyz}\n")
                if os.environ.get("SHOTS") and n == 0 and xyz[2] < float(os.environ["SHOTS"]):  # slow it down there
                    v = [f32(plane + 0x140 + 4 * i) for i in range(3)]
                    for i in range(3):
                        p.write32(plane + 0x140 + 4 * i, struct.unpack("<I", struct.pack("<f", v[i] * 0.01))[0])
                    for n in range(1, 5):
                        eye = lambda: ",".join(f"{f32(0x1e7f30 + 16 * r + 4 * j):.4f}" for r in (3, 2, 1) for j in range(3)) + f",{math.tan(math.radians(f32(0x1e7d50)) / 2) * 4 / 3:.6f}"
                        e0 = eye()
                        shot(f"p{k}_{n:03}")
                        log.write(f"shot p{k}_{n:03} eye {e0} / {eye()} plane {[round(f32(plane + 0x1b0 + 4 * i), 3) for i in range(16)]}\n")
                    for i in range(3):
                        p.write32(plane + 0x140 + 4 * i, struct.unpack("<I", struct.pack("<f", v[i]))[0])
            time.sleep(0.03)
        print("point", k, "shots", n, flush=True)
