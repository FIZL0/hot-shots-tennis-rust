#!/usr/bin/env python3
"""P26c: the original's winners' ceremony (the result object's state 0). Loads a save slot, pokes team 0's games to
one short of the set, waits for the match to end, then logs every vsync: result state +0x217c, +0x2384 (camera
done), shown team +0x2388, +0x2380, banner rows +0x2188/+0x218c, the players' modes +0x1f4.., and the sound
manager's MT index (draws); screenshots (F8) every `every` frames of state 0 and at the result page's first tick.
Saves scratch slot 8 when the result object appears.
usage: p26c_ceremony.py <slot> <outdir> [max_frames] [every]. Run under tools/pcsx2.sh."""
import sys, time, os, subprocess
sys.path.insert(0, "tools")
from pine import Pine
p = Pine()
out = sys.argv[2]; os.makedirs(out, exist_ok=True)
want = int(sys.argv[3]) if len(sys.argv) > 3 else 20000
every = int(sys.argv[4]) if len(sys.argv) > 4 else 20
slot = int(sys.argv[1]); p.load_state(slot); time.sleep(2)
saved = slot == 8  # 8: the ceremony's start, saved by an earlier run
if not saved:
    g = p.read32(0x423044) - 1
    p.write32(0x42306c, g); p.write32(0x42307c, g); p.write32(0x423070, 0); p.write32(0x423090, 0); p.write32(0x423064, 3); p.write32(0x423068, 0)
mt = p.read32(p.read32(0x43b1d0) + 0x740)
i32 = lambda a: p.read32(a) - (1 << 32) * (p.read32(a) >> 31)
def shot(name):
    subprocess.run(["tools/screenshot.sh", f"{out}/{name}.png"], capture_output=True)
log = open(f"{out}/log.txt", "w")
last, n, k0, prev = p.read32(0x1d5780), 0, None, None
while n < want:
    v = p.read32(0x1d5780)
    if v == last: continue
    last, n = v, n + 1
    o = p.read32(0x43b2f0)
    if not o or not (0x100000 <= o < 0x2000000): continue
    st = p.read8(o + 0x217c)
    if k0 is None:
        k0 = n
        log.write(f"{n} v={v} obj={o:#x} rows={i32(o + 0x2188)},{i32(o + 0x218c)} team={i32(o + 0x2388)} "
                  f"lost={i32(o + 0x2380)} winner={i32(0x4230a8)}\n")
        if not saved: p.save_state(8); saved = True
    cur = (st, p.read8(o + 0x2384), [i32(o + 0x1f4 + 4 * i) for i in range(p.read32(0x422fa4))], p.read32(mt + 0x9c4))
    if cur != prev:
        log.write(f"{n - k0} v={v} state={cur[0]} done={cur[1]} modes={cur[2]} mtidx={cur[3]}\n"); log.flush()
        prev = cur
    if st == 0 and (n - k0) % every == 0:
        shot(f"s0_{n - k0:03d}"); log.write(f"{n - k0} v={p.read32(0x1d5780)} shot\n")
    if st == 1:
        shot("s1"); break
log.write(f"end {n}\n"); print("done", n)
