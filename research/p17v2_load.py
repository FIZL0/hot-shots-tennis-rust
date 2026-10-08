#!/usr/bin/env python3
"""GS-dump a match load on court 10, to catch the load-time draw of the shadow casters' textures (P17v2).
Usage: tools/pcsx2.sh python3 research/p17v2_load.py <out.gs.zst> [court, default 10]

Walks slot 2's character select as tools/pick.py does (P1 = character 0; on copy 5 Cross confirms), pokes the court index (0x422f90) once the
match is set up, and runs PCSX2's multi-frame GS dump (held F12, rebound in copy 5's ini) from then until P1 can serve. Moves the dump
to <out>."""
import os, shutil, subprocess, sys, time
sys.path.insert(0, "tools")
from pine import Pine
from pick import vpad

out, court = sys.argv[1], int(sys.argv[2]) if len(sys.argv) > 2 else 10
n = os.environ["HST_PCSX2"]
snaps = f"../pcsx2/s{n}/PCSX2/snaps"
pid = open(os.path.join(os.environ.get("XDG_RUNTIME_DIR", "/tmp"), f"hst-pcsx2{n}.pid")).read().strip()
# the multi-frame dump runs while its hotkey is held: F12 down starts it, up ends it
dump = lambda st: ["hyprctl", "dispatch", f'hl.dsp.send_key_state({{ mods = "", key = "F12", state = "{st}", window = "pid:{pid}" }})']
p = Pine()
p.load_state(2)
time.sleep(1.5)
vpad("sleep 2500", "press cross 150", "sleep 1200", *["press cross 150", "sleep 1200"] * 3, "press start 150", "sleep 3000")
before = set(os.listdir(snaps))
t, dumping = time.time(), False
while True:
    gm = p.read32(0x422f80)
    if not gm and not dumping and time.time() - t < 20:
        vpad("press cross 150", "sleep 2500")
    c = p.read32(0x422f90)
    if not dumping and gm:
        p.pause()
        print("court", c, "→", court, "gm", hex(gm), flush=True)
        p.write32(0x422f90, court)
        subprocess.run(dump("down"), check=True)
        p.resume()
        dumping = True
    pl = gm and p.read32(gm + 0xa8)
    if pl and p.read8(gm + 0x55) == 2 and p.read8(pl + 0x3fa4) == 1:
        break
    if time.time() - t > 120:
        print("no serve")
        break
    time.sleep(0.05)
subprocess.run(dump("up"), check=True)
print("court now", p.read32(0x422f90))
for _ in range(240):
    new = [f for f in set(os.listdir(snaps)) - before if f.endswith((".gs", ".gs.zst", ".gs.xz"))]
    if new:
        f = os.path.join(snaps, new[0])
        s = -1
        while s != os.path.getsize(f):
            s = os.path.getsize(f)
            time.sleep(1)
        shutil.move(f, out)
        print("dump", out, s)
        break
    time.sleep(0.5)
else:
    print("CAPTURE FAILED: no dump")
