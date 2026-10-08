#!/usr/bin/env python3
"""List the draws in a PCSX2 GS dump (.gs, zstd -d first): one line per run of primitives with the same GS state.

  gsdump.py dump.gs [--all]   (default: only the last frame, i.e. after the last vsync but one)

Each line: index, prim type and count, screen bbox (pixels, XYOFFSET removed), FRAME FBP/PSM/FBMSK, TEX0 TBP/PSM
(if textured), ALPHA (A,B,C,D,FIX), TEST, flags (ABE/TME/FGE), first vertex RGBAQ. Enough to find full-screen
post passes (sprites textured from the frame buffer, constant-colour overlays)."""
import struct, sys

PRIMS = ["point", "line", "lstrip", "tri", "tstrip", "tfan", "sprite", "?"]
NVERT = [1, 2, 2, 3, 3, 3, 2, 1]


def packets(buf):
    magic, hsize = struct.unpack_from("<II", buf, 0)
    assert magic == 0xFFFFFFFF, "old dump format"
    (state_version, state_size, *_rest) = struct.unpack_from("<II", buf, 8)
    o = 8 + hsize + state_size + 8192
    while o < len(buf):
        t = buf[o]; o += 1
        if t == 0:
            path, size = struct.unpack_from("<BI", buf, o); o += 5
            yield ("xfer", path, buf[o:o + size]); o += size
        elif t == 1:
            o += 1; yield ("vsync",)
        elif t == 2:
            o += 4
        elif t == 3:
            o += 8192
        else:
            raise ValueError(f"packet type {t} at {o}")


class Gs:
    def __init__(self):
        self.r = {}
        self.verts = []
        self.out = []

    def set(self, a, v):
        self.r[a] = v
        if a == 0x00:  # PRIM
            self.verts = []
        elif a in (0x04, 0x05, 0x0c, 0x0d):  # XYZF2, XYZ2, XYZF3, XYZ3
            self.verts.append((v & 0xFFFF, (v >> 16) & 0xFFFF, self.r.get(1, 0)))
            prim = self.r.get(0, 0)
            n = NVERT[prim & 7]
            if len(self.verts) >= n:
                if a in (0x04, 0x05):
                    self.emit(prim, self.verts[-n:])
                if prim & 7 in (0, 1, 3, 6):
                    self.verts = []

    def emit(self, prim, vs):
        ctx = (prim >> 9) & 1
        g = lambda a1, a2: self.r.get(a2 if ctx else a1, 0)
        off = g(0x18, 0x19)
        ox, oy = off & 0xFFFF, (off >> 32) & 0xFFFF
        xs = [(v[0] - ox) / 16 for v in vs]; ys = [(v[1] - oy) / 16 for v in vs]
        frame, tex0, alpha, test = g(0x4c, 0x4d), g(0x06, 0x07), g(0x42, 0x43), g(0x47, 0x48)
        key = (prim & 0x7ff & ~0x8, frame, tex0 if prim & 0x10 else 0, alpha if prim & 0x40 else 0, test,
               self.r.get(0x46), self.r.get(0x45))  # ignore IIP (shading) when grouping
        if self.out and self.out[-1]["key"] == key:
            d = self.out[-1]; d["n"] += 1
            d["bb"] = [min(d["bb"][0], *xs), min(d["bb"][1], *ys), max(d["bb"][2], *xs), max(d["bb"][3], *ys)]
        else:
            self.out.append({"key": key, "n": 1, "bb": [min(xs), min(ys), max(xs), max(ys)], "rgba": vs[0][2]})


def gif(gs, data):
    o = 0
    while o + 16 <= len(data):
        lo, hi = struct.unpack_from("<QQ", data, o); o += 16
        nloop, eop, pre, prim, flg, nreg = lo & 0x7FFF, (lo >> 15) & 1, (lo >> 46) & 1, (lo >> 47) & 0x7FF, (lo >> 58) & 3, (lo >> 60) & 0xF
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
                    elif r == 1:  # RGBAQ packed
                        gs.set(1, (a & 0xFF) | ((a >> 32) & 0xFF) << 8 | (b & 0xFF) << 16 | ((b >> 32) & 0xFF) << 24)
                    elif r in (4, 5):  # XYZF2/XYZ2 packed; ADC bit (b bit 47) → no kick
                        x, y = a & 0xFFFF, (a >> 32) & 0xFFFF
                        adc = (b >> 47) & 1
                        reg = (0x0c if r == 4 else 0x0d) if adc else r
                        gs.set(reg, x | y << 16)
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


def main():
    buf = open(sys.argv[1], "rb").read()
    frames = [[]]
    for p in packets(buf):
        if p[0] == "vsync":
            frames.append([])
        else:
            frames[-1].append(p)
    gs = Gs()
    pick = [f for f in frames if f]
    for f in pick if "--all" in sys.argv else pick[-1:]:
        for _, path, data in f:
            gif(gs, data)
    for i, d in enumerate(gs.out):
        prim, frame, tex0, alpha, test, colclamp, dthe = d["key"]
        fbp, psm, msk = frame & 0x1FF, (frame >> 24) & 0x3F, frame >> 32
        s = f"{i:4} {PRIMS[prim & 7]:6}x{d['n']:<5} bb {d['bb'][0]:6.0f},{d['bb'][1]:5.0f}-{d['bb'][2]:5.0f},{d['bb'][3]:5.0f} FBP {fbp:#x}/{psm:#x}"
        if msk:
            s += f" MSK {msk:#x}"
        if prim & 0x10:
            s += f" TBP {tex0 & 0x3FFF:#x}/{(tex0 >> 20) & 0x3F:#x} TFX {(tex0 >> 35) & 3}"
        if prim & 0x40:
            s += f" ALPHA {alpha & 3}{(alpha >> 2) & 3}{(alpha >> 4) & 3}{(alpha >> 6) & 3} FIX {(alpha >> 32) & 0xFF:#x}"
        s += f" TEST {test:#x}" + (" FGE" if prim & 0x20 else "") + f" rgba {d['rgba']:08x}"
        print(s)


if __name__ == "__main__":
    main()
