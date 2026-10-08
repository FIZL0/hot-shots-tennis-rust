#!/usr/bin/env python3
"""P3f2: log every rand() call from a menu save state through the match intro to the first serve, then print the
intro's length: the match frame (gm+0x58) at the first point's shared seed, the flare ticks before it, the court, the
players and the weather. Nothing is pressed once the match is loaded (a press may skip the intro).
Usage (under tools/pcsx2.sh): p3f2_intro_log.py <load slot> <out.bin> <vpad command>... (the route from the menu to the
match, e.g. slot 2 doubles: "sleep 2500" "press cross 150" "sleep 1200" ...). Records as research/p3f_rand_log.py's.
--fixture out.bin log.bin... (crates/hst-sim/tests/rng.rs `intro_ticks_like_the_game`): per log 4 × u32: the court
(from the name's c<court>), the players, the rand() state's low word at the intro's first flare tick and at the first
point's shared seed."""
import os, struct, sys, time
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "tools"))
from pine import Pine
from pick import vpad
from p3f_rand_log import BUF, CODE, DATA, RAND, VSYNC, build, j

if sys.argv[1] == "--fixture":
    with open(sys.argv[2], "wb") as fix:
        for f in sys.argv[3:]:
            d = open(f, "rb").read()
            r = [struct.unpack_from("<6I", d, i) for i in range(0, len(d), 24)]
            e = next(i for i, x in enumerate(r) if x[1] == 0x324ca0)
            t = next(x for x in r if x[1] == 0x137f20 and x[3] >> 8 & 0xff == 0)
            court, players = map(int, os.path.basename(f).split("_")[:2])
            fix.write(struct.pack("<4I", court, players, t[4], r[e][4]))
    sys.exit()
slot, out, route = int(sys.argv[1]), sys.argv[2], sys.argv[3:]
p = Pine()
v0 = p.read32(VSYNC)
p.load_state(slot)
# the load lands late: patch a few frames after the vsync counter jumps
while abs(p.read32(VSYNC) - v0) < 30: time.sleep(0.01)
v0 = p.read32(VSYNC)
while p.read32(VSYNC) - v0 < 5: time.sleep(0.01)
A, B = p.read32(RAND), p.read32(RAND + 4)
p.pause()
p.write32(DATA, BUF)
for n, w in enumerate(build(A, B)): p.write32(CODE + 4 * n, w)
p.write32(RAND, j(CODE))
p.write32(RAND + 4, 0)
p.resume()
try:
    vpad(*route)
    t = time.time()
    while True:
        gm = p.read32(0x422f80)
        pl = gm and p.read32(gm + 0xa8)
        if pl and p.read8(gm + 0x55) == 2 and p.read8(pl + 0x3fa4) == 1: break
        if time.time() - t > 120: sys.exit("match didn't reach the serve")
        time.sleep(0.1)
finally:
    p.pause()
    p.write32(RAND + 4, B)
    p.write32(RAND, A)
    end = p.read32(DATA)
    p.resume()
data = b"".join(p.read_block(x, min(0x10000, end - x)) for x in range(BUF, end, 0x10000)) if end > BUF else b""
open(out, "wb").write(data)
r = [struct.unpack_from("<6I", data, i) for i in range(0, len(data), 24)]
s = next(i for i, x in enumerate(r) if x[1] == 0x322f1c)
e = next(i for i, x in enumerate(r) if x[1] == 0x324ca0)
ticks = sum(x[1] == 0x137f20 for x in r[s:e]) / 24
frame = max((x[5] for x in r[s:e] if x[3] >> 8 & 0xff == 0), default=None)  # the intro phase's last logged frame
i32 = lambda a: struct.unpack("<i", struct.pack("<I", p.read32(a)))[0]
print(f"{out}: court {i32(0x422f90)} players {i32(0x422fa4)} characters {[i32(0x422fa8 + 4 * k) for k in range(4)]}"
      f" intro frame {frame} flare ticks {ticks} (vsync {r[s][0]}..{r[e][0]})")
