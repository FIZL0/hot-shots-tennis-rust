#!/usr/bin/env python3
"""Record the hit-effect manager every frame from a save-state load (bot games: slot 5).
Usage: record_effects.py <slot> <frames> <out.bin>. A PCSX2 copy runs at 1x (tools/pine.py; HST_LOCKSTEP=1: every frame, slowly); the user's own PCSX2: run it slowed (NominalScalar 0.25).

Header: u32 n models (6), then per impact model u32 MOR entry count, u32 MTA entry count.
Sample: u32 vsync, effects object 0x100, impact effect (+0xa0 object) 0x60, ball 0x290, court marker 8 (0x40),
then per impact model: model 0x80, mesh +0xe0..+0x140 (world matrix, scale), MOR player 0x30, MTA player 0x30,
the MOR entries (0x28 each: +0x20 key cursor, +0x24 weight), the MTA entries (0x20 each: +0x8 key cursor,
+0xc value) and the colour each MTA entry writes (0x10)."""
import struct, sys, time
from pine import Pine

VSYNC, GM_PTR, FX_PTR = 0x1d5780, 0x422f80, 0x423f80
p, want, out = Pine(step=True), int(sys.argv[2]), open(sys.argv[3], "wb")
p.load_state(int(sys.argv[1]))
time.sleep(2)  # the load lands asynchronously; pointers read before it are stale
gm, fx = p.read32(GM_PTR), p.read32(FX_PTR)
r = [(fx, 0x100), (p.read32(fx + 0xa0), 0x60), (p.read32(gm + 0x88), 0x290), (p.read32(gm + 0xb8) + 0xd130 + 8 * 0x40, 0x40)]
head = [6]
for k in range(6):
    m = p.read32(fx + 0x84 + 4 * k)
    mor, mta = p.read32(p.read32(m) + 0xc), p.read32(p.read32(m + 12) + 0xc)
    r += [(m, 0x80), (p.read32(p.read32(m)) + 0xe0, 0x60), (mor, 0x30), (mta, 0x30)]
    nm, nt = p.read32(mor + 8), p.read32(mta + 8)
    me, te = [p.read32(p.read32(mor + 0xc) + 4 * i) for i in range(nm)], [p.read32(p.read32(mta + 0xc) + 4 * i) for i in range(nt)]
    r += [(e, 0x28) for e in me] + [(e, 0x20) for e in te] + [(p.read32(e + 0x18), 0x10) for e in te]
    head += [nm, nt]
out.write(struct.pack(f"<{len(head)}I", *head))
last, n = p.read32(VSYNC), 0
while n < want:
    v = p.next_frame(last)
    if v != last + 1:
        print(f"missed frames {last + 1}..{v - 1}", flush=True)
    last = v
    a = p.settle(r, v)
    if a is None:
        print(f"missed frame {v} (it ticked mid-read)", flush=True)
        continue
    out.write(struct.pack("<I", v) + a)
    n += 1
    if n % 1200 == 0: print(n, flush=True)
print("done", n)
