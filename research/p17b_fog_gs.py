#!/usr/bin/env python3
"""P17b: every packed-XYZF2 vertex of a GS dump with PRIM FGE, its F and Q (= 1/w, VU1), checked against VU1's fog
F = ⌊max(min(intercept + w·slope, F near), F far)⌋ with the court view's fog block from a RAM dump (view +0x520:
F far, F near, slope, intercept). usage: p17b_fog_gs.py dump.gs dump.ram"""
import struct, sys
from collections import Counter
sys.path.insert(0, "research/tools")
from gsdump import packets
f32 = lambda v: struct.unpack("<f", struct.pack("<I", v & 0xffffffff))[0]
ram = open(sys.argv[2], "rb").read()
far, near, slope, icpt = struct.unpack_from("<4f", ram, 0x1e7d10 + 0x520)
print("view fog block: F far %g, F near %g, slope %g, intercept %g" % (far, near, slope, icpt))
f32r = lambda x: struct.unpack("<f", struct.pack("<f", x))[0]
player, zero, rgba = Counter(), 0, 0
seen, miss, prim, q = Counter(), Counter(), 0, 1.0
for p in packets(open(sys.argv[1], "rb").read()):
    if p[0] == "vsync": continue
    d, o = p[2], 0
    while o + 16 <= len(d):
        lo, hi = struct.unpack_from("<QQ", d, o); o += 16
        nloop, pre, pr, flg, nreg = lo & 0x7FFF, (lo >> 46) & 1, (lo >> 47) & 0x7FF, (lo >> 58) & 3, (lo >> 60) & 0xF
        nreg = nreg or 16
        regs = [(hi >> (4 * i)) & 0xF for i in range(nreg)]
        if pre: prim = pr
        if flg == 0:
            for _ in range(nloop):
                for r in regs:
                    a, b = struct.unpack_from("<QQ", d, o); o += 16
                    if r == 0: prim = a
                    elif r == 0xE and (b & 0xff) == 0: prim = a
                    elif r == 2: q = f32(b)
                    elif r == 1: rgba = (a & 0xff) | ((a >> 32) & 0xff) << 8 | (b & 0xff) << 16 | ((b >> 32) & 0xff) << 24
                    elif r == 4 and prim & 0x20 and q > 0:
                        F = (b >> 36) & 0xff
                        w = 1.0 / q
                        want = int(max(min(icpt + w * slope, near), far))
                        seen[(F == want, abs(F - want) <= 1)] += 1
                        # the players' view: the same far F and depths with their own near F (245 on court 10)
                        pl = int(max(min(245 + (w - 40) * (far - 245) / 150, 245), far))
                        if F != want: player[F == pl] += 1
                        if F == 0 and zero < 6: zero += 1; print("F 0: prim %#x Q %g xy %d,%d rgba %#x" % (prim, q, (a & 0xffff) / 16, ((a >> 32) & 0xffff) / 16, rgba))
                        if F != want and F != pl: miss[(F, want, round(w, -1) if w < 1000 else round(w, -2))] += 1
        elif flg == 1:
            o += ((nloop * nreg + 1) // 2) * 16
        else:
            o += nloop * 16
print("exact / within 1 / off:", sum(v for k, v in seen.items() if k[0]), sum(v for k, v in seen.items() if not k[0] and k[1]), sum(v for k, v in seen.items() if not k[1]))
by_f = Counter()
for (F, want, w), n in miss.items(): by_f[F] += n
print("misses by F:", by_f.most_common(12))
print("misses (F, want, w):", miss.most_common(15))
print("misses that fit the players' fog (near F 245):", player[True], "not:", player[False])
