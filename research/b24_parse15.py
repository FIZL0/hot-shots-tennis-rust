#!/usr/bin/env python3
"""B24: print research/b24_rec15.py samples where the type-15 creature's state changes (or every 30 while on)."""
import struct, sys
d = open(sys.argv[1], "rb").read()
S = 5 + 0x290 + 8 + 0x40 + 0x30 + 0x9d0
prev = None
for s in range((len(d) - 4) // S):
    o = 4 + s * S
    v, ph = struct.unpack_from("<IB", d, o)
    ob, b8, c, h = o + 5, o + 5 + 0x290, o + 5 + 0x298, o + 5 + 0x2d8
    f = lambda a: struct.unpack_from("<f", d, a)[0]
    st = (ph, d[ob + 0x281], d[ob + 0xbc], d[ob + 0x135], d[b8], struct.unpack_from("<i", d, b8 + 4)[0])
    if st != prev or (d[b8] and s % 20 == 0) or "-a" in sys.argv:
        print(s, v, st, "frame", f(c + 0x38), "len", f(h + 0x2c), "a274", round(f(ob + 0x274), 3),
              "pos", [round(f(ob + 0x1e0 + 4 * k), 3) for k in range(3)], "fwd", [round(f(ob + 0x1d0 + 4 * k), 3) for k in range(3)])
        prev = st
