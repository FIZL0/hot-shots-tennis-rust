#!/usr/bin/env python3
"""P11k7: which of the human shot record's (0x3646a0, tag 4 of record_ai_rally.py) branches a capture reaches.
Usage: p11k7_branches.py <capture.bin>... — per file, counts of: calls, claimed (path object +0x1fc = the player),
volley found (kind 2), kind 2 lost → tiers, the min 1 → min 0 retry after a find, smash found, tier finds."""
import struct, sys
from collections import Counter

MT, REC, A = 0x9c8, 0x780, 0x80
u32 = lambda d, o: struct.unpack_from("<I", d, o)[0]
i32 = lambda d, o: struct.unpack_from("<i", d, o)[0]
for name in sys.argv[1:]:
    d = open(name, "rb").read()
    o, c = 8 + MT, Counter()
    while o < len(d):
        e = d[o:o + u32(d, o + 4)]; o += len(e)
        x = d[o:o + u32(d, o + 4)]; o += len(x)
        if u32(e, 0) != 4:
            continue
        c["calls"] += 1
        team = i32(e, 0x348)
        hitter = i32(e, 0x40 + 0x18)
        if hitter < 0 or hitter & 1 == team & 1:
            continue
        c["opp shot"] += 1
        if i32(e, 0x74c) == team:
            c["claimed"] += 1
            continue
        f0, k0, m0 = e[A + 0x40], e[A + 0x48], i32(e, A + 0x4c)
        f1, k1, m1 = x[A + 0x40], x[A + 0x48], i32(x, A + 0x4c)
        if e[A + 0x50]:
            c["volley flag"] += 1
        if not f0 and f1:
            c[f"found kind {k1} min {m1}"] += 1
        if f0 and k0 == 2 and k1 == 0:
            c["kind 2 lost -> tiers"] += 1
        if f0 and k0 == 3 and k1 == 0:
            c["kind 3 lost -> tiers"] += 1
        if f0 and m0 == 1 and m1 == 0:
            c["min 1 -> 0 after a find"] += 1
        if f0 and k0 not in (0, 2, 3):
            c["other kind"] += 1
    print(name, dict(sorted(c.items())))
