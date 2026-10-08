#!/usr/bin/env python3
"""Every primitive of a GS dump with its full vertices (P17v): x, y (12.4, XYOFFSET kept), z, S, T, Q (floats),
RGBA, and the drawing state (PRIM, FRAME, ZBUF, TEX0, TEX1, CLAMP, ALPHA, TEST, XYOFFSET, SCISSOR) of its context.
Usage: p17v_gsprims.py dump.gs out.pkl  → a list of (vsync index, state dict, [vertex tuples])."""
import pickle, struct, sys
sys.path.insert(0, "research/tools")
from gsdump import packets, NVERT

f32 = lambda v: struct.unpack("<f", struct.pack("<I", v & 0xffffffff))[0]


class Gs:
    def __init__(self):
        self.r, self.verts, self.out, self.frame, self.q = {}, [], [], 0, 1.0

    def kick(self, x, y, z, draw):
        st = self.r.get(2, 0)
        self.verts.append((x, y, z, f32(st), f32(st >> 32), self.q, self.r.get(1, 0) & 0xffffffff, self.r.get(3, 0)))
        prim = self.r.get(0, 0)
        n = NVERT[prim & 7]
        if len(self.verts) >= n:
            if draw:
                c = (prim >> 9) & 1
                g = lambda a: self.r.get(a + c, 0)
                state = dict(prim=prim, frame=g(0x4c), zbuf=g(0x4e), tex0=g(0x06), tex1=g(0x14), clamp=g(0x08),
                             alpha=g(0x42), test=g(0x47), xyoffset=g(0x18), scissor=g(0x40), texa=self.r.get(0x3b, 0),
                             dthe=self.r.get(0x45, 0), colclamp=self.r.get(0x46, 0), fba=g(0x4a), pabe=self.r.get(0x49, 0))
                self.out.append((self.frame, state, list(self.verts[-n:])))
            if prim & 7 in (0, 1, 3, 6):
                self.verts = []
            elif prim & 7 == 5 and len(self.verts) > 3:  # fan keeps its first vertex
                self.verts = [self.verts[0]] + self.verts[-2:]
            elif len(self.verts) > 3:
                self.verts = self.verts[-2:]

    def set(self, a, v):
        if a == 0x00:
            self.verts = []
        if a == 0x01:
            self.q = f32(v >> 32)
        if a in (0x04, 0x05, 0x0c, 0x0d):
            z = (v >> 32) & (0xffffff if a in (0x04, 0x0c) else 0xffffffff)
            self.kick(v & 0xffff, (v >> 16) & 0xffff, z, a in (0x04, 0x05))
            return
        self.r[a] = v


def gif(gs, data):
    o = 0
    while o + 16 <= len(data):
        lo, hi = struct.unpack_from("<QQ", data, o); o += 16
        nloop, pre, prim, flg, nreg = lo & 0x7FFF, (lo >> 46) & 1, (lo >> 47) & 0x7FF, (lo >> 58) & 3, (lo >> 60) & 0xF
        nreg = nreg or 16
        regs = [(hi >> (4 * i)) & 0xF for i in range(nreg)]
        if pre:
            gs.set(0, prim)
        if flg == 0:
            for _ in range(nloop):
                for r in regs:
                    a, b = struct.unpack_from("<QQ", data, o); o += 16
                    if r == 0xE:
                        gs.set(b & 0xFF, a)
                    elif r == 1:
                        gs.r[1] = (a & 0xFF) | ((a >> 32) & 0xFF) << 8 | (b & 0xFF) << 16 | ((b >> 32) & 0xFF) << 24 | (gs.r.get(1, 0) & ~0xffffffff)
                    elif r == 2:  # ST packed: Q rides in the upper qword
                        gs.r[2] = a
                        gs.q = f32(b)
                    elif r in (4, 5):
                        x, y = a & 0xFFFF, (a >> 32) & 0xFFFF
                        adc = (b >> 47) & 1
                        z = (b >> 4) & 0xffffff if r == 4 else b & 0xffffffff
                        gs.kick(x, y, z, not adc)
                    elif r == 0:
                        gs.set(0, a)
                    elif r == 0xF:
                        pass
                    else:
                        gs.set(r, a)
        elif flg == 1:
            words = nloop * nreg
            for i in range(words):
                (v,) = struct.unpack_from("<Q", data, o + 8 * i)
                r = regs[i % nreg]
                if r not in (0xE, 0xF):
                    gs.set(r, v)
            o += ((words + 1) // 2) * 16
        else:
            o += nloop * 16


gs = Gs()
for p in packets(open(sys.argv[1], "rb").read()):
    if p[0] == "vsync":
        gs.frame += 1
    else:
        gif(gs, p[2])
pickle.dump(gs.out, open(sys.argv[2], "wb"))
print(len(gs.out), "prims over", gs.frame + 1, "vsync frames")
