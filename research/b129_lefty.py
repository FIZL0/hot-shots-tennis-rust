#!/usr/bin/env python3
"""B129: the ceremony's figures' facing in the original, with chosen players made left-handed. Loads slot 5 (bot-only
doubles), pokes team 0's games to one short of the set, sets the hand word (character record +4, 2 = left) of the
listed players' characters, waits for the result object, logs each figure's matrix (+0x2230 + 0x40·i) and hand
(+0x1e0 + 4·i) and screenshots state 0 every `every` frames.
usage: b129_lefty.py <outdir> <players,comma-separated|-> [every]. Run under tools/pcsx2.sh."""
import sys, time, os, struct, subprocess
sys.path.insert(0, "tools")
from pine import Pine
p = Pine()
out = sys.argv[1]; os.makedirs(out, exist_ok=True)
lefty = [] if sys.argv[2] == "-" else [int(x) for x in sys.argv[2].split(",")]
every = int(sys.argv[3]) if len(sys.argv) > 3 else 60
p.load_state(5); time.sleep(2)
g = p.read32(0x423044) - 1
p.write32(0x42306c, g); p.write32(0x42307c, g); p.write32(0x423070, 0); p.write32(0x423090, 0); p.write32(0x423064, 3); p.write32(0x423068, 0)
n_pl = p.read32(0x422fa4)
chars = [p.read32(0x422fa8 + 4 * i) for i in range(n_pl)]
log = open(f"{out}/log.txt", "w")
log.write(f"chars={chars} hands={[p.read32(0x2f0884 + c * 0x118) for c in chars]} swapped={p.read8(0x4230b4)}\n")
for i in lefty:
    p.write32(0x2f0884 + chars[i] * 0x118, 2)
f = lambda a: struct.unpack("<f", struct.pack("<I", p.read32(a)))[0]
last, n, k0 = p.read32(0x1d5780), 0, None
while n < 20000:
    v = p.read32(0x1d5780)
    if v == last: continue
    last, n = v, n + 1
    o = p.read32(0x43b2f0)
    if not o or not (0x100000 <= o < 0x2000000): continue
    st = p.read8(o + 0x217c)
    if k0 is None:
        k0 = n
        log.write(f"obj={o:#x} team={p.read32(o + 0x2388)} winner={p.read32(0x4230a8)} swapped={p.read8(0x4230b4)}\n")
        for i in range(n_pl):
            m = [round(f(o + 0x2230 + 0x40 * i + 4 * k), 3) for k in range(16)]
            log.write(f"p{i} char={chars[i]} hand={f(o + 0x1e0 + 4 * i)} cull={p.read8(o + 0x2170 + i)} m={m}\n")
        log.flush()
    if st == 0 and (n - k0) % every == 0:
        subprocess.run(["tools/screenshot.sh", f"{out}/s0_{n - k0:03d}.png"], capture_output=True)
    if st == 1 or n - k0 > 330:
        break
print("done", n)
