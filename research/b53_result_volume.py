#!/usr/bin/env python3
"""B53: the BGM's volume around the match end and on the result page. Loads a save slot, pokes team 0's games to one
short of the set (as p26c_ceremony.py), logs every change of: result state +0x217c and +0x2180 (leaving), the BGM
handle (*0x312540 +0x18), its volume (handle +0x10) and the music director's (*0x3165c8) level +0xd0 (float), mode
+0xd4, hold +0xe0, mute +0xe1. Stops `after` vsyncs past the result page's first tick.
usage: b53_result_volume.py <slot> <out.txt> [after]. Run under tools/pcsx2.sh."""
import sys, time, struct
sys.path.insert(0, "tools")
from pine import Pine
p = Pine()
slot, out = int(sys.argv[1]), open(sys.argv[2], "w")
after = int(sys.argv[3]) if len(sys.argv) > 3 else 300
p.load_state(slot); time.sleep(2)
g = p.read32(0x423044) - 1
p.write32(0x42306c, g); p.write32(0x42307c, g); p.write32(0x423070, 0); p.write32(0x423090, 0); p.write32(0x423064, 3); p.write32(0x423068, 0)
bgm, d = p.read32(0x312540), p.read32(0x3165c8)
f32 = lambda a: struct.unpack("<f", struct.pack("<I", p.read32(a)))[0]
last, n, k0, s1, prev = p.read32(0x1d5780), 0, None, None, None
while True:
    v = p.read32(0x1d5780)
    if v == last: continue
    last, n = v, n + 1
    h = p.read32(bgm + 0x18)
    o = p.read32(0x43b2f0)
    res = (p.read8(o + 0x217c), p.read32(o + 0x2180)) if o and 0x100000 <= o < 0x2000000 else None
    if res and k0 is None: k0 = n
    if res and res[0] == 1 and s1 is None: s1 = n
    cur = (res, h, p.read32(h + 0x10) if h else None, round(f32(d + 0xd0), 2), p.read32(d + 0xd4), p.read8(d + 0xe0), p.read8(d + 0xe1))
    if cur != prev:
        out.write(f"{n} k={n - k0 if k0 else None} res={res} bgm={h:#x} vol={cur[2]} dir={cur[3:]}\n"); out.flush()
        prev = cur
    if s1 is not None and n - s1 >= after or n > 30000: break
out.write(f"end n={n} k0={k0} s1={s1}\n"); print("done", n, k0, s1)
