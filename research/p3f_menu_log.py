#!/usr/bin/env python3
"""P3f: log every rand() call from the doubles character select (save slot 2) through the match setup to P1's first
serve, picking character <char> for P1 as tools/pick.py does, then 2P's cursor moved by [dir ...] (it starts on character 2:
`1 8 out.bin left` gives P1 and P1's partner character 1), 3P and 4P where they start, and save that serve to <save slot>.
On pad copy 3 ✕ confirms (○ goes back).
Usage (under tools/pcsx2.sh): p3f_menu_log.py <char> <save slot> <out.bin> [dir ...]. Records as research/p3f_rand_log.py's."""
import os, sys, time
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "tools"))
from pine import Pine
from pick import PATH, vpad
from p3f_rand_log import BUF, CODE, DATA, RAND, VSYNC, build, j

ch, slot, out, com = int(sys.argv[1]), int(sys.argv[2]), sys.argv[3], sys.argv[4:]
p = Pine()
p.load_state(2)
# the load lands late (slot 2 is at vsync 4370): patch a few frames after it
while not 4370 <= p.read32(VSYNC) < 4420: time.sleep(0.01)
v0 = p.read32(VSYNC)
while p.read32(VSYNC) - v0 < 5: time.sleep(0.01)
A, B = p.read32(RAND), p.read32(RAND + 4)
stub = build(A, B)
p.pause()
p.write32(DATA, BUF)
for n, w in enumerate(stub): p.write32(CODE + 4 * n, w)
p.write32(RAND, j(CODE))
p.write32(RAND + 4, 0)
p.resume()
try:
    vpad("sleep 2500", *[c for d in PATH[ch] for c in (f"press {d} 150", "sleep 700")], "press cross 150",
         "sleep 1200", *[c for d in com for c in (f"press {d} 150", "sleep 700")], *["press cross 150", "sleep 1200"] * 3, "press start 150", "sleep 3000")
    t = time.time()
    while True:
        gm = p.read32(0x422f80)
        if not gm and time.time() - t < 20:
            vpad("press cross 150", "sleep 2500")
        pl = gm and p.read32(gm + 0xa8)
        if pl and p.read8(gm + 0x55) == 2 and p.read8(pl + 0x3fa4) == 1:
            break
        if time.time() - t > 90: sys.exit("match didn't reach the serve")
        time.sleep(0.1)
finally:
    p.pause()
    p.write32(RAND + 4, B)
    p.write32(RAND, A)
    end = p.read32(DATA)
    p.resume()
data = b"".join(p.read_block(x, min(0x10000, end - x)) for x in range(BUF, end, 0x10000)) if end > BUF else b""
open(out, "wb").write(data)
time.sleep(1)
p.save_state(slot)
time.sleep(2)
print(f"{len(data) // 24} calls; characters", [p.read32(0x422fa8 + 4 * k) for k in range(4)], f"saved to slot {slot}")
