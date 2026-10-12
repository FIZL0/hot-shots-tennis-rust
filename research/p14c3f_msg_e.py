#!/usr/bin/env python3
"""P14c3f: print a `record_npc.py … trig` recording made with HST_GM=1 where the match phase (+0x54), the replay
flags (+0x344, +0x346) or any trigger creature's last message (+0xc0), +0xbc, +0xbd or active (+0x12c) change.
Usage: p14c3f_msg_e.py <rec.bin>"""
import struct, sys
b = open(sys.argv[1], "rb").read()
n = struct.unpack_from("<I", b)[0]
objs = struct.unpack_from(f"<{n}I", b, 4)
head = 4 + 4 * n
size = 4 + 0x180 + 0x9d0 + 0x280 + 0x20 + 8 + n * (0x290 + 0x40 + 0x30) + 0x360
prev, i = None, 0
for at in range(head, len(b) - size + 1, size):
    s = b[at:at + size]
    gm = s[size - 0x360:]
    base = lambda k: 4 + 0x180 + 0x9d0 + 0x280 + 0x20 + 8 + k * (0x290 + 0x40 + 0x30)
    cr = tuple((s[base(k) + 0x50], struct.unpack_from("<I", s, base(k) + 0xc0)[0], s[base(k) + 0xbc], s[base(k) + 0xbd],
                s[base(k) + 0x12c]) for k in range(n))
    st = (gm[0x54], gm[0x344], gm[0x346], cr)
    if st != prev:
        print(i, struct.unpack_from("<I", s)[0], "phase", gm[0x54], "replay", gm[0x344], "over", gm[0x346],
              " ".join(f"[{t} m{m:x} bc{bc} bd{bd} a{a}]" for t, m, bc, bd, a in cr))
    prev, i = st, i + 1
