#!/usr/bin/env python3
"""B184: the original's ceremony with chosen players' characters poked (e.g. to Carol, 6) before the match ends:
loads slot 5 (bot-only doubles), pokes team 0's games/points to one short of the match, sets the listed players'
character ids (0x422fa8 + 4·i), waits for the result object and takes one screenshot `at` frames into it.
usage: b184_carol.py <outdir> <player=char,...|-> [at]. Run under tools/pcsx2.sh."""
import sys, time, os, struct, subprocess
sys.path.insert(0, "tools")
from pine import Pine
p = Pine()
out = sys.argv[1]; os.makedirs(out, exist_ok=True)
pokes = [] if sys.argv[2] == "-" else [tuple(map(int, x.split("="))) for x in sys.argv[2].split(",")]
at = int(sys.argv[3]) if len(sys.argv) > 3 else 0
p.load_state(5); time.sleep(2)
g = p.read32(0x423044) - 1
p.write32(0x42306c, g); p.write32(0x42307c, g); p.write32(0x423070, 0); p.write32(0x423090, 0); p.write32(0x423064, 3); p.write32(0x423068, 0)
n_pl = p.read32(0x422fa4)
log = open(f"{out}/log.txt", "w")
log.write(f"before chars={[p.read32(0x422fa8 + 4 * i) for i in range(n_pl)]}\n")
for i, c in pokes:
    p.write32(0x422fa8 + 4 * i, c)
chars = [p.read32(0x422fa8 + 4 * i) for i in range(n_pl)]
log.write(f"chars={chars} hands={[p.read32(0x2f0884 + c * 0x118) for c in chars]}\n")
f = lambda a: struct.unpack("<f", struct.pack("<I", p.read32(a)))[0]
last, n, k0, nxt = p.read32(0x1d5780), 0, None, 0
while n < 20000:
    v = p.read32(0x1d5780)
    if v == last: continue
    last, n = v, n + 1
    o = p.read32(0x43b2f0)
    if not o or not (0x100000 <= o < 0x2000000): continue
    st = p.read8(o + 0x217c)
    if k0 is None:
        k0, k0v = n, v
        log.write(f"obj={o:#x} winner={p.read32(0x4230a8)} swapped={p.read8(0x4230b4)}\n")
        for i in range(n_pl):
            m = [round(f(o + 0x2230 + 0x40 * i + 4 * k), 3) for k in range(16)]
            log.write(f"p{i} hand={f(o + 0x1e0 + 4 * i)} m={m}\n")
        log.flush()
    if st == 0 and n - k0 >= at:
        # one shot `at` frames in; a screenshot takes a while, so the name has the counter before and after it
        a = (v - k0v) & 0xffffffff
        subprocess.run(["tools/screenshot.sh", f"{out}/tmp.png"], capture_output=True)
        os.replace(f"{out}/tmp.png", f"{out}/s{at:04d}_{a}-{(p.read32(0x1d5780) - k0v) & 0xffffffff}.png")
        break
    if st == 1 or n - k0 > 400:
        break
print("done", n)
