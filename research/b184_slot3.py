#!/usr/bin/env python3
"""B184: the original's ceremony with its own Carol (P1, hand toggled to right) and Will (lefty) from slot 3.
Pokes team `team`'s games/points to one short of the match, presses Cross now and then (P1 serves, double-faults or
swings) until the result object is up, optionally pokes every controller word to the computer's (0x21) so the
ceremony shows the winners instead of P1's side, logs each figure's matrix and takes one screenshot `at` frames in.
With `flip`, the match's winner word is turned over to the other team as soon as it is set.
usage: b184_slot3.py <outdir> <team> <coms 0|1> [at] [flip]. Run under tools/pcsx2.sh."""
import sys, time, os, struct, subprocess
sys.path.insert(0, "tools")
from pine import Pine
p = Pine()
out, team, coms = sys.argv[1], int(sys.argv[2]), sys.argv[3] == "1"
at = int(sys.argv[4]) if len(sys.argv) > 4 else 0
flip = len(sys.argv) > 5
os.makedirs(out, exist_ok=True)
p.load_state(3); time.sleep(2)
g = p.read32(0x423044) - 1
p.write32(0x42306c + 4 * team, g); p.write32(0x42307c + 0x14 * team, g); p.write32(0x423064 + 4 * team, 3)
n_pl = p.read32(0x422fa4)
chars = [p.read32(0x422fa8 + 4 * i) for i in range(n_pl)]
if coms:
    for i in range(n_pl):
        p.write32(0x422fc8 + 4 * i, 0x21)
log = open(f"{out}/log.txt", "a")
log.write(f"team={team} coms={coms} at={at} chars={chars} hands={[p.read32(0x2f0884 + c * 0x118) for c in chars]}\n")
f = lambda a: struct.unpack("<f", struct.pack("<I", p.read32(a)))[0]
pad = subprocess.Popen(["bash", "-c", "while true; do tools/vpad.py send 'press cross 100'; sleep 1.5; done"])
last, n, k0, t0 = p.read32(0x1d5780), 0, None, time.time()
while time.time() - t0 < 300:
    v = p.read32(0x1d5780)
    if v == last: continue
    last, n = v, n + 1
    if flip and k0 is None and p.read32(0x4230a8) == team:
        p.write32(0x4230a8, 1 - team)
    o = p.read32(0x43b2f0)
    if not o or not (0x100000 <= o < 0x2000000) or p.read32(0x4230a8) > 1: continue
    st = p.read8(o + 0x217c)
    if k0 is None:
        pad.kill(); subprocess.run(["tools/vpad.py", "send", "release"])
        k0, k0v = n, v
        log.write(f"obj={o:#x} winner={p.read32(0x4230a8)} swapped={p.read8(0x4230b4)}\n")
        for i in range(n_pl):
            m = [round(f(o + 0x2230 + 0x40 * i + 4 * k), 3) for k in range(16)]
            log.write(f"p{i} hand={f(o + 0x1e0 + 4 * i)} flag={p.read8(p.read32(p.read32(p.read32(o + 0x178 + 4 * i))) + 0x135)} m={m}\n")
        log.flush()
    if st == 0 and n - k0 >= at:
        a = (v - k0v) & 0xffffffff
        subprocess.run(["tools/screenshot.sh", f"{out}/tmp.png"], capture_output=True)
        os.replace(f"{out}/tmp.png", f"{out}/t{team}c{int(coms)}_{at:04d}_{a}-{(p.read32(0x1d5780) - k0v) & 0xffffffff}.png")
        break
pad.kill()
print("done", n, k0)
