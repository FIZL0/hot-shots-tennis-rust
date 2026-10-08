#!/usr/bin/env python3
"""P3e7: court 5. `save`: slot 9's confirm screen → Select Court → two left of the list's first cursor (menu entry 11, internal court 5) → back on the confirm screen, saved to
slot 8. Default: load slot 8, start the match (P1 pressing ✕ twice, 0.45 s apart, every 2.5 s: toss and hit), print every change of the match phase (gm+0x54/0x55), the
manager's run/kind/count/tick-reset, the scoreboard's call (+0x426) and point kind (+0x560) and 0x4230b8, with the
vsync, for `secs` seconds (real time: a change inside one tick shows only its result)."""
import os, struct, subprocess, sys, time
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "tools"))
from pine import Pine
from pick import vpad

p = Pine()
if sys.argv[1:2] == ["save"]:
    p.load_state(9); time.sleep(1.5)
    vpad("press down 150", "sleep 600", "press cross 150", "sleep 2000", *["press left 150", "sleep 700"] * 2,
         "press cross 150", "sleep 2000")
    subprocess.run(["tools/screenshot.sh", "context/p3e7/court5_confirm.png"])
    p.save_state(8); time.sleep(2); sys.exit()
secs = float(sys.argv[1]) if sys.argv[1:] else 60
p.load_state(8); time.sleep(1.5)
vpad("press up 150", "sleep 600", "press cross 150")
t, last, pressed = time.time(), None, 0
while time.time() - t < secs:
    if time.time() - pressed > 2.5:  # P1 serves and swings
        vpad("press cross 120", "sleep 450", "press cross 120"); pressed = time.time()
    v, gm, m, sb = p.read32(0x1d5780), p.read32(0x422f80), p.read32(0x43b1c0), p.read32(0x42d6c0)
    if not (gm and m and sb and 0x100000 <= m < 0x2000000): continue
    kind, count = struct.unpack("<ii", p.read_block(m + 0x8c4, 8))
    st = (p.read8(gm + 0x54), p.read8(gm + 0x55), p.read8(m + 0x1b61), kind, count, p.read8(sb + 0x426),
          p.read32(sb + 0x560), struct.unpack("<i", struct.pack("<I", p.read32(0x4230b8)))[0], p.read32(0x422f90))
    if st != last:
        print(v, "phase %d→%d run %d kind %d count %d call %d kind560 %d b8 %d stage %d" % st, flush=True)
        last = st
