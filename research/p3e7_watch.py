#!/usr/bin/env python3
"""P3e7: watch the gallery manager's marks (count, kind, the first 8 slots) and gm+0x54/0x55 through a match start,
printing each change with its vsync (real time, so a change inside one tick shows only its result).
Usage (under tools/pcsx2.sh): p3e7_watch.py save  → slot 2 to the confirm screen, saved to slot 9;
p3e7_watch.py [secs] → load slot 9, start the match (✕ is the pad's "cross" in copy 2), watch."""
import os, struct, sys, time
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "tools"))
from pine import Pine
from pick import vpad

p = Pine()
if sys.argv[1:] == ["save"]:
    p.load_state(2); time.sleep(1.5)
    vpad("sleep 2500", *["press cross 150", "sleep 1200"] * 4, "press start 150", "sleep 3000")
    p.save_state(9); time.sleep(2); sys.exit()
secs = float(sys.argv[1]) if sys.argv[1:] else 60
p.load_state(9); time.sleep(1.5)
vpad("press cross 150")
t, last = time.time(), None
while time.time() - t < secs:
    v, gm, m = p.read32(0x1d5780), p.read32(0x422f80), p.read32(0x43b1c0)
    if not (gm and m and 0x100000 <= m < 0x2000000): continue
    b = p.read_block(m + 0x8c4, 8 + 0x30 * 8)
    kind, count = struct.unpack_from("<ii", b)
    slots = [struct.unpack_from("<3f3fii", b, 8 + 0x30 * k + 0)[:3] + struct.unpack_from("<3f", b, 8 + 0x30 * k + 0x10)
             + struct.unpack_from("<ii", b, 8 + 0x30 * k + 0x20) for k in range(8)]
    st = (p.read8(gm + 0x54), p.read8(gm + 0x55), p.read8(m + 0x1b61), kind, count,
          tuple(tuple(round(x, 3) if isinstance(x, float) else x for x in s) for s in slots))
    if st != last:
        print(v, "gm54/55", st[0], st[1], "run", st[2], "kind", kind, "count", count)
        for k, s in enumerate(st[5]):
            if any(s): print("   ", k, s)
        last = st
