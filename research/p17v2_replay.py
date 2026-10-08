#!/usr/bin/env python3
"""Replay the static shadow-texture pass of a match-load GS dump the way PCSX2's software renderer draws it (P17v2).

Usage: p17v2_replay.py load.gs pool.npy [frame]   (load.gs from p17v2_load.py; pool.npy: 16 × 128 × 128 palette
indices of the textures at 0x3308 + 0x20·i, e.g. read from cap0.gs; frame: the vsync whose pass is replayed)

Keeps VRAM from the dump's start, applies every image transfer, and from the given frame on draws what goes to the
256² alpha target at FBP 0x8c: the clear, the casters' strips (DATE, ATE GEQUAL 0x60; solid ones write 0x80, the
textured ones their bilinear At, HIGHLIGHT2), the four bilinear sprites that halve it to 128² in place, and the
PSMT4HH copy into the texture. Prints how many texels of each texture differ from pool.npy."""
import struct, sys
import numpy as np
sys.path.insert(0, "research")
sys.path.insert(0, "research/tools")
from gsdump import packets
from p17v_gssim import addr32, addr4, vram, F, trunc

NVERT = [1, 2, 2, 3, 3, 3, 2, 1]
# PCSX2 GSTables.cpp: PSMT8 block and column swizzles
BT8 = [[0, 1, 4, 5, 16, 17, 20, 21], [2, 3, 6, 7, 18, 19, 22, 23], [8, 9, 12, 13, 24, 25, 28, 29], [10, 11, 14, 15, 26, 27, 30, 31]]
CT8 = [[0, 4, 16, 20, 32, 36, 48, 52, 2, 6, 18, 22, 34, 38, 50, 54],
       [8, 12, 24, 28, 40, 44, 56, 60, 10, 14, 26, 30, 42, 46, 58, 62],
       [33, 37, 49, 53, 1, 5, 17, 21, 35, 39, 51, 55, 3, 7, 19, 23],
       [41, 45, 57, 61, 9, 13, 25, 29, 43, 47, 59, 63, 11, 15, 27, 31],
       [96, 100, 112, 116, 64, 68, 80, 84, 98, 102, 114, 118, 66, 70, 82, 86],
       [104, 108, 120, 124, 72, 76, 88, 92, 106, 110, 122, 126, 74, 78, 90, 94],
       [65, 69, 81, 85, 97, 101, 113, 117, 67, 71, 83, 87, 99, 103, 115, 119],
       [73, 77, 89, 93, 105, 109, 121, 125, 75, 79, 91, 95, 107, 111, 123, 127],
       [128, 132, 144, 148, 160, 164, 176, 180, 130, 134, 146, 150, 162, 166, 178, 182],
       [136, 140, 152, 156, 168, 172, 184, 188, 138, 142, 154, 158, 170, 174, 186, 190],
       [161, 165, 177, 181, 129, 133, 145, 149, 163, 167, 179, 183, 131, 135, 147, 151],
       [169, 173, 185, 189, 137, 141, 153, 157, 171, 175, 187, 191, 139, 143, 155, 159],
       [224, 228, 240, 244, 192, 196, 208, 212, 226, 230, 242, 246, 194, 198, 210, 214],
       [232, 236, 248, 252, 200, 204, 216, 220, 234, 238, 250, 254, 202, 206, 218, 222],
       [193, 197, 209, 213, 225, 229, 241, 245, 195, 199, 211, 215, 227, 231, 243, 247],
       [201, 205, 217, 221, 233, 237, 249, 253, 203, 207, 219, 223, 235, 239, 251, 255]]


def addr8(x, y, bp, bw):  # byte address
    page = (y >> 6) * (bw >> 1) + (x >> 7)
    return bp * 256 + page * 8192 + BT8[(y >> 4) & 3][(x >> 4) & 7] * 256 + CT8[y & 15][x & 15]


def texel_round(st, q):
    """PCSX2 GSState's texel coordinate rounding (STQ, all Z of the draw equal): S or T loses its low 9 mantissa bits,
    more by the exponent it is below Q's."""
    b, e = struct.unpack("<I", struct.pack("<f", st))[0], struct.unpack("<I", struct.pack("<f", q))[0] >> 23 & 0xff
    es = b >> 23 & 0xff
    return f32(b & ~((1 << min(9 + max(es, e) - es, 23)) - 1))


f32 = lambda v: struct.unpack("<f", struct.pack("<I", v & 0xffffffff))[0]


class Gs:
    def __init__(self, vm, frame):
        self.w = vm.copy().view(np.uint32)  # VRAM as words
        self.r, self.verts, self.q, self.frame, self.from_frame = {}, [], 1.0, 0, frame
        self.batch, self.blits, self.xfer, self.skipped = None, [], None, set()

    # --- memory -------------------------------------------------------------------------------------------------
    def nib(self, x, y, bp, bw):
        a = addr4(x, y, bp, bw)
        return (int(self.w[a >> 3]) >> (4 * (a & 7))) & 15

    def set_nib(self, x, y, bp, bw, n):
        a = addr4(x, y, bp, bw)
        i, s = a >> 3, 4 * (a & 7)
        self.w[i] = (int(self.w[i]) & ~(15 << s) & 0xffffffff) | (n << s)

    def image(self, data):
        """Host → local: CT32 only (the game uploads every texture and CLUT that way)."""
        bb, (x0, y0, w, h, i) = self.r[0x50], self.xfer
        dbp, dbw, dpsm = (bb >> 32) & 0x3fff, (bb >> 48) & 0x3f, (bb >> 56) & 0x3f
        if dpsm != 0:  # ponytail: the shadow pass reads CT32 uploads only; others are noted, not written
            self.skipped.add((dbp, dpsm))
            return
        words = np.frombuffer(data, np.uint32)
        for wv in words:
            if i >= w * h:
                break
            self.w[addr32(x0 + i % w, y0 + i // w, dbp, dbw)] = wv
            i += 1
        self.xfer = (x0, y0, w, h, i)

    def local(self):
        bb, pos, reg = self.r[0x50], self.r[0x51], self.r[0x52]
        sbp, sbw, spsm = bb & 0x3fff, (bb >> 16) & 0x3f, (bb >> 24) & 0x3f
        dbp, dbw, dpsm = (bb >> 32) & 0x3fff, (bb >> 48) & 0x3f, (bb >> 56) & 0x3f
        sx, sy, dx, dy = pos & 0x7ff, (pos >> 16) & 0x7ff, (pos >> 32) & 0x7ff, (pos >> 48) & 0x7ff
        w, h = reg & 0xfff, (reg >> 32) & 0xfff
        if (spsm, dpsm) == (0, 0):
            for y in range(h):
                for x in range(w):
                    self.w[addr32(dx + x, dy + y, dbp, dbw)] = self.w[addr32(sx + x, sy + y, sbp, sbw)]
        elif (spsm, dpsm) == (0x2c, 0x14):  # PSMT4HH (alpha bits 28..31) → PSMT4
            for y in range(h):
                for x in range(w):
                    self.set_nib(dx + x, dy + y, dbp, dbw, int(self.w[addr32(sx + x, sy + y, sbp, sbw)]) >> 28)
            if sbp == 0x1180 and self.frame >= self.from_frame:
                self.blits.append(dbp)
        else:
            raise ValueError(f"local copy {spsm:#x} → {dpsm:#x}")

    def texture(self, tex0):
        """TEX0's texture as (h, w) uint32 colours (PSMT4 through a CT32 CSM1 CLUT, or CT32)."""
        tbp, tbw, psm = tex0 & 0x3fff, (tex0 >> 14) & 0x3f, (tex0 >> 20) & 0x3f
        tw, th, cbp = 1 << ((tex0 >> 26) & 15), 1 << ((tex0 >> 30) & 15), (tex0 >> 37) & 0x3fff
        if psm == 0:
            return np.array([[self.w[addr32(x, y, tbp, tbw)] for x in range(tw)] for y in range(th)], np.int64)
        assert psm in (0x13, 0x14) and (tex0 >> 51) & 15 == 0 and (tex0 >> 55) & 1 == 0, hex(tex0)
        if psm == 0x13:  # CSM1: entries 8..15 and 16..23 of every 32 swap places
            b = self.w.view(np.uint8)
            clut = np.array([self.w[addr32((i & 7) | (i & 0x10) >> 1, (i >> 4 & 0xe) | (i >> 3 & 1), cbp, 1)] for i in range(256)], np.int64)
            return clut[[[b[addr8(x, y, tbp, tbw)] for x in range(tw)] for y in range(th)]]
        clut = np.array([self.w[addr32(i & 7, i >> 3, cbp, 1)] for i in range(16)], np.int64)
        return clut[[[self.nib(x, y, tbp, tbw) for x in range(tw)] for y in range(th)]]

    # --- drawing ------------------------------------------------------------------------------------------------
    def kick(self, x, y, z, draw):
        st = self.r.get(2, 0)
        self.verts.append((x, y, f32(st), f32(st >> 32), self.q, self.r.get(1, 0), self.r.get(3, 0), z))
        prim = self.r.get(0, 0)
        n = NVERT[prim & 7]
        if len(self.verts) >= n:
            if draw and self.frame >= self.from_frame and (self.r.get(0x4c, 0) & 0x1ff) == 0x8c:
                assert not prim & 0x200
                self.prim(prim, self.verts[-n:])
            if prim & 7 == 6:
                self.verts = []
            elif len(self.verts) > 3:
                self.verts = self.verts[-2:]

    def flush(self):
        self.batch = None

    def prim(self, prim, vs):
        g = self.r.get
        key = (prim & ~8, g(0x06), g(0x14), g(0x08), g(0x47), g(0x18), g(0x40), g(0x4c))
        if self.batch is None or self.batch[0] != key:  # PCSX2 draws a batch with one texture snapshot
            self.batch = (key, self.texture(g(0x06)) if prim & 0x10 else None)
        tex = self.batch[1]
        frame, test, xyo, sc = g(0x4c), g(0x47), g(0x18), g(0x40)
        fbw = (frame >> 16) & 0x3f
        assert frame >> 32 == 0x00ffffff, hex(frame)  # alpha only (blending is RGB only)
        scis = (sc & 0x7ff, (sc >> 32) & 0x7ff, ((sc >> 16) & 0x7ff) + 1, ((sc >> 48) & 0x7ff) + 1)
        ox, oy = xyo & 0xffff, (xyo >> 32) & 0xffff
        ate, atst, aref, date = test & 1, (test >> 1) & 7, (test >> 4) & 0xff, (test >> 14) & 1
        assert (test >> 12) & 3 == 0 and (test >> 15) & 1 == 0 and atst in (5,) if ate else True
        tex0, clamp = g(0x06), g(0x08)
        tw, th = (tex0 >> 26) & 15, (tex0 >> 30) & 15
        uvclamp = []
        for wm, mn, mx, n in ((clamp & 3, (clamp >> 4) & 0x3ff, (clamp >> 14) & 0x3ff, 1 << tw),
                              ((clamp >> 2) & 3, (clamp >> 24) & 0x3ff, (clamp >> 34) & 0x3ff, 1 << th)):
            uvclamp.append({0: (True, n - 1, 0), 1: (False, 0, n - 1), 2: (False, min(mn, n - 1), min(mx, n - 1)),
                            3: (True, mn & (n - 1), mx)}[wm])
        assert g(0x14) & 0x21 == 0x21 or not prim & 0x10  # bilinear (LCM 1, K 0: magnification)

        def put(y, x, at):  # x: column array; at: source alpha per pixel
            idx = np.array([addr32(int(c), y, 0x8c * 32, fbw) for c in x])
            d = self.w[idx].astype(np.int64)
            ok = np.ones(len(x), bool)
            if date:
                ok &= (d >> 31) == 0
            if ate:
                ok &= at >= aref
            self.w[idx[ok]] = ((d[ok] & 0xffffff) | (at[ok] << 24)).astype(np.uint32)

        def sample(u, v):  # 16.16 texel coordinates (half texel already off) → bilinear alpha
            uf, vf = (u & 0xffff) >> 12, (v & 0xffff) >> 12
            res = []
            for (rep, mn, mx), a in zip(uvclamp, (u >> 16, v >> 16)):
                res.append([((b & mn) | mx) if rep else np.clip(b, mn, mx) for b in (a, a + 1)])
            (x0, x1), (y0, y1) = res
            al = (tex >> 24) & 0xff
            c00, c01, c10, c11 = al[y0, x0], al[y0, x1], al[y1, x0], al[y1, x1]
            r0 = c00 + (((c01 - c00) * uf) >> 4)
            r1 = c10 + (((c11 - c10) * uf) >> 4)
            return r0 + (((r1 - r0) * vf) >> 4)

        a_vertex = (vs[-1][5] >> 24) & 0xff
        tsz = (F(0x10000 << tw), F(0x10000 << th))
        if prim & 7 == 6:  # sprite (FST)
            assert prim & 0x110 == 0x110 or not prim & 0x10
            p = [(F(F(x - ox) * F(1 / 16)), F(F(y - oy) * F(1 / 16))) for x, y, *_ in vs]
            t = [(F(F(v[6] & 0x3fff) * F(4096)) - F(0x8000), F(F((v[6] >> 16) & 0x3fff) * F(4096)) - F(0x8000)) for v in vs]
            if p[1] < p[0]:
                p, t = p[::-1], t[::-1]
            l, tp = max(int(np.ceil(p[0][0])), scis[0]), max(int(np.ceil(p[0][1])), scis[1])
            r, b = min(int(np.ceil(p[1][0])), scis[2]), min(int(np.ceil(p[1][1])), scis[3])
            if r <= l or b <= tp:
                return
            if not prim & 0x10:
                for y in range(tp, b):
                    put(y, np.arange(l, r), np.full(r - l, a_vertex, np.int64))
                return
            dt = [F(F(t[1][k] - t[0][k]) / F(p[1][k] - p[0][k])) for k in (0, 1)]
            s0 = F(t[0][0] + F(dt[0] * F(F(l) - p[0][0])))
            t0 = F(t[0][1] + F(dt[1] * F(F(tp) - p[0][1])))
            for y in range(tp, b):
                xs = np.arange(l, r)
                u, v = self.lanes(s0, dt[0], l, r), np.full(r - l, int(trunc(t0)), np.int64)
                put(y, xs, sample(u, v))
                t0 = F(t0 + dt[1])
            return
        assert prim & 7 == 4
        out = []
        if prim & 0x10 and len({v[7] for v in vs}) == 1:  # ponytail: PCSX2 asks for equal Z over the whole batch; the shadow strips are flat
            vs = [(*v[:2], texel_round(v[2], v[4]), texel_round(v[3], v[4]), *v[4:]) for v in vs]
        for x, y, s, t, q, rgba, uv, _ in vs:
            px, py = F(F(x - ox) * F(1 / 16)), F(F(y - oy) * F(1 / 16))
            out.append((px, py, F(F(F(s) * tsz[0]) - F(0x8000)), F(F(F(t) * tsz[1]) - F(0x8000)), F(q), F(((rgba >> 24) & 0xff) << 7)))
        if prim & 0x10:
            assert all(v[4] == 1.0 for v in vs)  # constant Q: the fst path
            tfx, iip = (tex0 >> 35) & 3, prim & 8
            assert tfx in (0, 3) and (tex0 >> 34) & 1 and (iip or tfx == 3)

            def span(y, l, r, tc, ds):
                at = sample(self.lanes(tc[0], ds[0], l, r), self.lanes(tc[1], ds[1], l, r))
                if tfx == 0:  # MODULATE: (At << 2) · gaf >> 16, gaf the 8.7 Gouraud alpha
                    at = np.minimum((at << 2) * self.colour(tc[3], ds[3], l, r) >> 16, 255)
                put(y, np.arange(l, r), at)
            self.tri(out, scis, span)
        else:
            self.tri(out, scis, lambda y, l, r, tc, ds: put(y, np.arange(l, r), np.full(r - l, a_vertex, np.int64)))

    @staticmethod
    def lanes(t0, d, left, right):
        """PCSX2 SW's fst texture coordinate over a span (AVX2: 8 lanes from the aligned 8 at or before left)."""
        skip = left & 7
        nv = (right - left + skip + 7) // 8
        off = (np.arange(8) - skip).astype(F)
        k = np.repeat(np.arange(nv), 8)
        u = int(trunc(t0)) + np.tile(trunc(F(d) * off), nv) + k * int(trunc(F(d) * F(8)))
        return u[skip:skip + right - left]

    @staticmethod
    def colour(c0, d, left, right):
        """PCSX2 SW's Gouraud channel (8.7) over a span: truncated at its start, 16-bit lane steps, ≥ 0 per step."""
        skip = left & 7
        nv = (right - left + skip + 7) // 8
        off = trunc(F(d) * (np.arange(8) - skip).astype(F)) & 0xffff
        step = int(trunc(F(d) * F(8))) & 0xffff
        c, out = (int(trunc(c0)) + off) & 0xffff, []
        for k in range(nv):
            if k:
                c = (c + step) & 0xffff
                c = np.where(c >= 0x8000, 0, c)  # max_i16 with 0
            out.append(c)
        return np.concatenate(out)[skip:skip + right - left]

    @staticmethod
    def tri(v, scis, span):
        """p17v_gssim.draw_tri's PCSX2 SW triangle setup, calling span(y, left, right, (s, t, q) at left, d/dx)."""
        ys = [v[0][1], v[1][1], v[2][1]]
        m1 = (ys[0] > ys[1]) | (ys[0] > ys[2]) << 1 | (ys[1] > ys[2]) << 2
        order = [[0, 1, 2], [1, 0, 2], None, [1, 2, 0], [0, 2, 1], None, [2, 0, 1], [2, 1, 0]][m1]
        v0, v1, v2 = (np.array(v[i], F) for i in order)
        if v0[1] == v1[1] == v2[1]:
            return
        flat_top = v0[1] == v1[1]
        tbf = np.ceil(np.array([v0[1], v1[1], v1[1], v2[1]], F))
        t0, t1 = max(tbf[0], scis[1]), max(tbf[1], scis[1])
        b1, b2 = min(tbf[2], scis[3]), min(tbf[3], scis[3])
        dv0, dv1, dv2 = v1 - v0, v2 - v0, v2 - v1
        cross = F(dv0[1] * dv1[0]) - F(dv0[0] * dv1[1])
        if cross == 0:
            return
        m2 = int(cross < 0)
        d = [F(dv0[0] / dv0[1]) if dv0[1] else F(np.inf), F(dv1[0] / dv1[1]), F(dv2[0] / dv2[1]) if dv2[1] else F(np.inf)]
        c = np.array([dv0[0], dv0[1], dv1[0], dv1[1]], F) / cross
        dscan = (dv1 * c[1] - dv0 * c[3]).astype(F)
        dedge = (dv0 * c[2] - dv1 * c[0]).astype(F)

        def section(top, bottom, ex, dex, etc, p0):
            for y in range(int(top), int(bottom)):
                dy = F(F(y) - p0[1])
                lx, rx = F(ex[0] + F(dex[0] * dy)), F(ex[1] + F(dex[1] * dy))
                left, right = int(max(np.ceil(lx), scis[0])), int(min(np.ceil(rx), scis[2]))
                if right <= left:
                    continue
                pre = F(F(left) - p0[0])
                tc = ((etc + (dedge[2:] * dy).astype(F)).astype(F) + (dscan[2:] * pre).astype(F)).astype(F)
                span(y, left, right, tc, dscan[2:])

        if flat_top:
            a, b = (v0, v1) if m2 else (v1, v0)
            dd = (d[1], d[2]) if m2 else (d[2], d[1])
            section(t0, b2, (a[0], b[0]), dd, a[2:], a)
        else:
            dd = (d[1], d[0]) if m2 else (d[0], d[1])
            section(t0, b1, (v0[0], v0[0]), dd, v0[2:], v0)
            e = (F(v0[0] + F(dd[0] * dv0[1])), F(v0[0] + F(dd[1] * dv0[1])))
            dd2 = (d[1], d[2]) if m2 else (d[2], d[1])
            section(t1, b2, e, dd2, v1[2:], v1)

    # --- GIF ------------------------------------------------------------------------------------------------------
    def set(self, a, v):
        if a == 0x00:
            self.verts = []
        if a == 0x01:
            self.q = f32(v >> 32)
        if a in (0x04, 0x05, 0x0c, 0x0d):
            self.kick(v & 0xffff, (v >> 16) & 0xffff, (v >> 32) & (0xffffff if a in (0x04, 0x0c) else 0xffffffff), a in (0x04, 0x05))
            return
        if a in (0x00, 0x06, 0x08, 0x14, 0x18, 0x40, 0x47, 0x4c):
            self.flush()
        self.r[a] = v
        if a == 0x53:
            self.flush()
            if v & 3 == 0:
                reg = self.r[0x52]
                pos = self.r[0x51]
                self.xfer = ((pos >> 32) & 0x7ff, (pos >> 48) & 0x7ff, reg & 0xfff, (reg >> 32) & 0xfff, 0)
            elif v & 3 == 2:
                self.local()

    def gif(self, data):
        o = 0
        while o + 16 <= len(data):
            lo, hi = struct.unpack_from("<QQ", data, o); o += 16
            nloop, pre, prim, flg, nreg = lo & 0x7FFF, (lo >> 46) & 1, (lo >> 47) & 0x7FF, (lo >> 58) & 3, (lo >> 60) & 0xF
            nreg = nreg or 16
            regs = [(hi >> (4 * i)) & 0xF for i in range(nreg)]
            if pre:
                self.set(0, prim)
            if flg == 0:
                for _ in range(nloop):
                    for r in regs:
                        a, b = struct.unpack_from("<QQ", data, o); o += 16
                        if r == 0xE:
                            self.set(b & 0xFF, a)
                        elif r == 1:
                            self.r[1] = (a & 0xFF) | ((a >> 32) & 0xFF) << 8 | (b & 0xFF) << 16 | ((b >> 32) & 0xFF) << 24  # Q stays
                        elif r == 2:
                            self.r[2] = a
                            self.q = f32(b)
                        elif r in (4, 5):
                            adc = (b >> 47) & 1
                            self.kick(a & 0xFFFF, (a >> 32) & 0xFFFF, (b >> 4) & 0xffffff if r == 4 else b & 0xffffffff, not adc)
                        elif r == 0xF:
                            pass
                        else:
                            self.set(r, a)
            elif flg == 1:
                words = nloop * nreg
                for i in range(words):
                    (v,) = struct.unpack_from("<Q", data, o + 8 * i)
                    r = regs[i % nreg]
                    if r not in (0xE, 0xF):
                        self.set(r, v)
                o += ((words + 1) // 2) * 16
            else:
                self.image(data[o:o + nloop * 16])
                o += nloop * 16


def main():
    dump, pool = sys.argv[1], np.load(sys.argv[2])
    frame = int(sys.argv[3]) if len(sys.argv) > 3 else 396
    gs = Gs(vram(dump), frame)
    for p in packets(open(dump, "rb").read()):
        if p[0] == "vsync":
            gs.frame += 1
            if gs.frame > frame:
                break
        else:
            gs.gif(p[2])
            if len([b for b in gs.blits if b >= 0x3308]) >= 16:
                break
    out = []
    for i in range(16):
        t = np.array([[gs.nib(x, y, 0x3308 + 0x20 * i, 2) for x in range(128)] for y in range(128)])
        out.append(int((t != pool[i]).sum()))
        if "-v" in sys.argv and out[-1]:
            ys, xs = np.nonzero(t != pool[i])
            print(i, [(int(x), int(y), int(t[y, x]), int(pool[i][y, x])) for y, x in zip(ys[:12], xs[:12])])
    print("texels that differ per texture:", out)
    print("uploads not applied (dbp, psm):", sorted((hex(a), hex(b)) for a, b in gs.skipped))


if __name__ == "__main__":
    main()
