#!/usr/bin/env python3
"""B53: what music plays on the original's result (stat) page. Loads a save slot, pokes team 0's games to one short
of the set (as p26c_ceremony.py), and from the result object's appearance logs every change of: result state
+0x217c, the BGM object's handle/id (*0x312540 +0x18/+0x1c), and each of the 8 SMF slots' track pointer (+0x60)
and tick (+0x48, whether it advances). Runs `after` vsyncs past the result page's first tick, pressing nothing.
usage: b53_result_music.py <slot> <out.txt> [after]. Run under tools/pcsx2.sh."""
import sys, time, struct
sys.path.insert(0, "tools")
from pine import Pine
p = Pine()
slot, out = int(sys.argv[1]), open(sys.argv[2], "w")
after = int(sys.argv[3]) if len(sys.argv) > 3 else 900
p.load_state(slot); time.sleep(2)
g = p.read32(0x423044) - 1
p.write32(0x42306c, g); p.write32(0x42307c, g); p.write32(0x423070, 0); p.write32(0x423090, 0); p.write32(0x423064, 3); p.write32(0x423068, 0)
SMF = 0x310e40
bgm = p.read32(0x312540)
last, n, k0, s1, prev, ticks = p.read32(0x1d5780), 0, None, None, None, [0] * 8
while True:
    v = p.read32(0x1d5780)
    if v == last: continue
    last, n = v, n + 1
    a = p.read_regions([(SMF, 0x400)])
    slots = [struct.unpack_from("<I", a, 0x80 * i + 0x60)[0] for i in range(8)]
    t = [struct.unpack_from("<d", a, 0x80 * i + 0x48)[0] for i in range(8)]
    moving = tuple(i for i in range(8) if t[i] != ticks[i]); ticks = t
    o = p.read32(0x43b2f0)
    st = p.read8(o + 0x217c) if o and 0x100000 <= o < 0x2000000 else None
    if st is not None and k0 is None: k0 = n
    if st == 1 and s1 is None: s1 = n
    cur = (st, p.read32(bgm + 0x18), p.read32(bgm + 0x1c), tuple(slots), moving)
    if cur != prev:
        out.write(f"{n} v={v} k={n - k0 if k0 else None} st={st} bgm={cur[1]:#x},{cur[2]:#x} "
                  f"tracks={[hex(x) for x in slots]} moving={moving}\n"); out.flush()
        prev = cur
    if s1 is not None and n - s1 >= after or n > 30000: break
out.write(f"end n={n} k0={k0} s1={s1}\n"); print("done", n, k0, s1)
