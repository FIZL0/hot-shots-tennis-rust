#!/usr/bin/env python3
"""B38: the screen transition that brings up the result page (result object state 0 → 1). Loads a save slot (5: pokes
team 0's games to one short of the set; 9: the state-1 start saved by an earlier run), saves scratch slot 8 at the first transition (the match's end), logs the global transition
object's words (+0x8..+0x14) and the result state every vsync; saves scratch slot 9 at state 1's first tick and
screenshots (F8) every `every` vsyncs from there for `frames`.
usage: b38_transition.py <slot> <outdir> [frames] [every]. Run under tools/pcsx2.sh."""
import sys, time, os, subprocess
sys.path.insert(0, "tools")
from pine import Pine
p = Pine()
slot, out = int(sys.argv[1]), sys.argv[2]; os.makedirs(out, exist_ok=True)
frames = int(sys.argv[3]) if len(sys.argv) > 3 else 120
every = int(sys.argv[4]) if len(sys.argv) > 4 else 0
p.load_state(slot); time.sleep(2)
if slot != 9:
    g = p.read32(0x423044) - 1
    p.write32(0x42306c, g); p.write32(0x42307c, g); p.write32(0x423070, 0); p.write32(0x423090, 0); p.write32(0x423064, 3); p.write32(0x423068, 0)
i32 = lambda a: p.read32(a) - (1 << 32) * (p.read32(a) >> 31)
log = open(f"{out}/log.txt", "w")
k8 = None
last, n, k1, prev, nxt = p.read32(0x1d5780), 0, None, None, 0
while n < 40000:
    v = p.read32(0x1d5780)
    if v == last: continue
    last, n = v, n + 1
    o = p.read32(0x43b2f0)
    st = p.read8(o + 0x217c) if 0x100000 <= o < 0x2000000 else -1
    t = p.read32(0x1ea5d0)
    cur = (st, i32(t + 8), i32(t + 0xc), p.read32(t + 0x10), p.read8(t + 0x14), p.read32(t), p.read32(0x1e9f10))
    if cur[4] and k8 is None and slot != 9:
        k8 = n; p.save_state(8)  # the first transition after the load: the match → the ceremony
    if (st >= 0 or cur[4]) and cur != prev:
        log.write(f"{n} v={v} k={'' if k1 is None else n - k1} state={st} t8={cur[1]} tc={cur[2]} t10={cur[3]:#x} "
                  f"t14={cur[4]} vt={cur[5]:#x} cur={cur[6]:#x}\n"); log.flush()
        prev = cur
    if st == 1 and k1 is None:
        k1 = n
        if slot != 9: p.save_state(9)
    if k1 is not None:
        k = n - k1
        if every and k >= nxt:
            subprocess.run(["tools/screenshot.sh", f"{out}/k{k:03d}.png"], capture_output=True)
            log.write(f"shot k{k:03d} at v={p.read32(0x1d5780)}\n"); nxt = k + every
        if k >= frames: break
log.write(f"end {n}\n"); print("done", n)
