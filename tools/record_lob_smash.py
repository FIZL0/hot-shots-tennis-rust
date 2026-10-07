#!/usr/bin/env python3
"""Record a bot game (slot 5) whose smashes are all △ smashes: whenever a player's contact search locks onto a
smash (+0x3ec1 == 4, countdown +0x3ec4 >= 0), its pressed button (+0x3ee4) is set to △ (4) before the hit, so
the game's own hit code launches smash kind 1. Usage: record_lob_smash.py <slot> <out.bin> <frames>.
Same sample layout as record_p2m2.py (`hst_sim::replay::frames_live`). Run PCSX2 slowed down (NominalScalar 0.5)."""
import struct, sys, time
from pine import Pine

VSYNC, GM_PTR = 0x1d5780, 0x422f80
PAD, GLOBALS, RALLY = (0x2efb00, 0x90), (0x422f80, 0x180), (0x3165f0, 0x50)
PLAYER = ((0x1380, 0x200), (0x3c00, 0x400))
TRIANGLE = 4

p, out, want = Pine(), open(sys.argv[2], "wb"), int(sys.argv[3])
p.load_state(int(sys.argv[1]))
time.sleep(2)  # the load lands asynchronously; pointers read before it are stale
gm = p.read32(GM_PTR)
players = [p.read32(gm + 0xa8 + 4 * i) for i in range(4)]
r = [PAD, GLOBALS, (gm, 0x100), (p.read32(gm + 0x98), 0x290)]
r += [(pl + o, n) for pl in players for o, n in PLAYER] + [(p.read32(gm + 0x88), 0x290), RALLY]
last, n, poked = p.read32(VSYNC), 0, 0
while n < want:
    v = p.read32(VSYNC)
    if v == last:
        continue
    if v != last + 1:
        print(f"missed frames {last + 1}..{v - 1}", flush=True)
    last = v
    time.sleep(0.004)
    a = p.read_regions(r)
    while True:
        b = p.read_regions(r)
        if a == b and p.read32(VSYNC) == v: break
        a = b
    out.write(struct.pack("<I", v) + a)
    n += 1
    for pl in players:
        if p.read8(pl + 0x3ec1) == 4 and struct.unpack("<i", struct.pack("<I", p.read32(pl + 0x3ec4)))[0] >= 0 and p.read32(pl + 0x3ee4) != TRIANGLE:
            p.write32(pl + 0x3ee4, TRIANGLE)
            poked += 1
            print(f"vsync {v}: player {players.index(pl)} smash set to △", flush=True)
    if n % 1200 == 0: print(n, flush=True)
print("done", n, "pokes", poked)
