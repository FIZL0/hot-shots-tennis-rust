#!/usr/bin/env python3
"""B157: list the court model nodes in an EE RAM dump (e.g. eeMemory.bin from a save state) with their frustum state
(+0x110: 0x100 drawn, 0x200 culled, 0x300 drawn through VU1's clip pass), centre (+0xc0), bound radius (model bound
+0x30 × the node's scale) and distance to the court view's eye. usage: b157_nodes.py dump.ram [all]"""
import math, struct, sys
r = open(sys.argv[1], "rb").read()
u32 = lambda a: struct.unpack_from("<I", r, a)[0] if 0 <= a < len(r) - 4 else 0
f = lambda a: struct.unpack_from("<f", r, a)[0]
eye = [f(0x1e7d20 + 4 * i) for i in range(3)]
print("court", u32(0x422f90), "eye", [round(x, 3) for x in eye])
for n in range(0x100000, len(r) - 0x120, 0x10):
    st = u32(n + 0x110) & 0xffff
    if st not in (0x100, 0x200, 0x300) or f(n + 0xcc) != 1.0 or f(n + 0x3c) != 1.0: continue
    m = u32(n + 0x108); b = u32(m)
    if not (0x100000 <= m < len(r) and 0x100000 <= b < len(r)): continue
    rad = f(b + 0x30) * math.sqrt(f(n) ** 2 + f(n + 4) ** 2 + f(n + 8) ** 2)
    c = [f(n + 0xc0 + 4 * i) for i in range(3)]
    if not all(abs(x) < 1e4 for x in c) or not 0 < rad < 1e4: continue
    d = math.dist(c, eye)
    if st == 0x300 or len(sys.argv) > 2:
        print(hex(n), hex(st), "c", [round(x, 2) for x in c], "r %.2f" % rad, "eye d %.2f" % d, "inside" if d < rad else "")
