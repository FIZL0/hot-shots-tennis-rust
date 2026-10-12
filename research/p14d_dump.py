#!/usr/bin/env python3
"""Print the changes in a p14d_record.py recording: phase, replay flag, result kind, call, each creature's state."""
import struct, sys
d = open(sys.argv[1], "rb").read()
n = struct.unpack_from("<I", d)[0]
objs = struct.unpack_from(f"<{n}I", d, 4)
o, S = 4 + 4 * n, 4 + 0x10 + 0x10 + 8 + 8 + 0x9d0 + 0xc0 * n
prev = None
for k in range((len(d) - o) // S):
    s = d[o + k * S:o + (k + 1) * S]
    v = struct.unpack_from("<I", s)[0]
    ph, rep, kind, call = s[4 + 5], s[4 + 0x10 + 6], struct.unpack_from("<i", s, 0x24)[0], s[0x2c + 6]
    c = 0x34 + 0x9d0
    cr = [struct.unpack_from("<I", s, c + 0xc0 * i + 0x58)[0:1] + struct.unpack_from("<iBiiB", s, c + 0xc0 * i + 0xa4)
          for i in range(n)]
    cr = [(b, a4, a8, ac, b0, b4) for (b, a4, a8, ac, b0, b4) in [(x[0], x[1], x[2], struct.unpack_from("<i", s, c + 0xc0 * i + 0xac)[0], struct.unpack_from("<i", s, c + 0xc0 * i + 0xb0)[0], s[c + 0xc0 * i + 0xb4]) for i, x in enumerate(cr)]]
    key = (ph, rep, kind, call, [(a8, ac, b0, b4) for (_, _, a8, ac, b0, b4) in cr])
    if key != prev:
        print(k, v, "ph", ph, "rep", rep, "kind", kind, "call", call, " ".join(f"{b}:{a8}/{a4}/{ac}/{b0}{'+' if b4 else '-'}" for (b, a4, a8, ac, b0, b4) in cr))
    prev = key
