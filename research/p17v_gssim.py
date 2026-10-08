#!/usr/bin/env python3
"""Replay the sun-shade map's receiver pass from a GS dump the way PCSX2's software renderer draws it (P17v).

Usage: p17v_gssim.py dump.gs prims.pkl ref.red  (prims.pkl from p17v_gsprims.py, ref.red from p17v_rebuild.py)

Takes the game's own vertices (12.4 XY, S/T/Q) and the shadow textures from the dump's VRAM, rasterizes every
receiver triangle with PCSX2 SW's float scanline setup (AVX2 path: 8 lanes, truncating conversions), samples the
PSMT4 texture bilinearly (16.16 uv, 4-bit fraction, clamp), adds the reds (ALPHA Cs + Cd, FIX 0x80, clamped) and
compares each tile with the red bytes the game read back."""
import pickle, struct, sys
import numpy as np

F = np.float32
VRAM = 1 << 22

# PCSX2 GSTables.cpp: block and column swizzles
BT32 = [[0, 1, 4, 5, 16, 17, 20, 21], [2, 3, 6, 7, 18, 19, 22, 23], [8, 9, 12, 13, 24, 25, 28, 29],
        [10, 11, 14, 15, 26, 27, 30, 31]]
CT32 = [[0, 1, 4, 5, 8, 9, 12, 13], [2, 3, 6, 7, 10, 11, 14, 15], [16, 17, 20, 21, 24, 25, 28, 29],
        [18, 19, 22, 23, 26, 27, 30, 31], [32, 33, 36, 37, 40, 41, 44, 45], [34, 35, 38, 39, 42, 43, 46, 47],
        [48, 49, 52, 53, 56, 57, 60, 61], [50, 51, 54, 55, 58, 59, 62, 63]]
BT4 = [[0, 2, 8, 10], [1, 3, 9, 11], [4, 6, 12, 14], [5, 7, 13, 15], [16, 18, 24, 26], [17, 19, 25, 27],
       [20, 22, 28, 30], [21, 23, 29, 31]]
_ct4 = [[0, 8, 32, 40, 64, 72, 96, 104, 2, 10, 34, 42, 66, 74, 98, 106, 4, 12, 36, 44, 68, 76, 100, 108, 6, 14, 38, 46, 70, 78, 102, 110],
        [16, 24, 48, 56, 80, 88, 112, 120, 18, 26, 50, 58, 82, 90, 114, 122, 20, 28, 52, 60, 84, 92, 116, 124, 22, 30, 54, 62, 86, 94, 118, 126],
        [65, 73, 97, 105, 1, 9, 33, 41, 67, 75, 99, 107, 3, 11, 35, 43, 69, 77, 101, 109, 5, 13, 37, 45, 71, 79, 103, 111, 7, 15, 39, 47],
        [81, 89, 113, 121, 17, 25, 49, 57, 83, 91, 115, 123, 19, 27, 51, 59, 85, 93, 117, 125, 21, 29, 53, 61, 87, 95, 119, 127, 23, 31, 55, 63],
        [192, 200, 224, 232, 128, 136, 160, 168, 194, 202, 226, 234, 130, 138, 162, 170, 196, 204, 228, 236, 132, 140, 164, 172, 198, 206, 230, 238, 134, 142, 166, 174],
        [208, 216, 240, 248, 144, 152, 176, 184, 210, 218, 242, 250, 146, 154, 178, 186, 212, 220, 244, 252, 148, 156, 180, 188, 214, 222, 246, 254, 150, 158, 182, 190],
        [129, 137, 161, 169, 193, 201, 225, 233, 131, 139, 163, 171, 195, 203, 227, 235, 133, 141, 165, 173, 197, 205, 229, 237, 135, 143, 167, 175, 199, 207, 231, 239],
        [145, 153, 177, 185, 209, 217, 241, 249, 147, 155, 179, 187, 211, 219, 243, 251, 149, 157, 181, 189, 213, 221, 245, 253, 151, 159, 183, 191, 215, 223, 247, 255]]
CT4 = _ct4 + [[c + 256 for c in r] for r in _ct4]


def addr32(x, y, bp, bw):  # word address
    page = (y >> 5) * bw + (x >> 6)
    return bp * 64 + page * 2048 + BT32[(y >> 3) & 3][(x >> 3) & 7] * 64 + CT32[y & 7][x & 7]


def addr4(x, y, bp, bw):  # nibble address
    page = (y >> 7) * (bw >> 1) + (x >> 7)
    return bp * 512 + page * 16384 + BT4[(y >> 4) & 7][(x >> 5) & 3] * 512 + CT4[y & 15][x & 31]


def vram(dump):
    b = open(dump, "rb").read(1 << 24)
    hsize, = struct.unpack_from("<I", b, 4)
    ssize, = struct.unpack_from("<I", b, 12)
    st = 8 + hsize
    vo = st + ssize - VRAM - 84  # the state ends with VRAM, 4 GIF paths (tag + reg) and q
    return np.frombuffer(b, np.uint8, VRAM, vo)


def texture(vm, tex0):
    """128×128 PSMT4 texture looked up through its CT32 CLUT (CSM1) → red, (128, 128) uint8."""
    tbp, tbw, cbp = tex0 & 0x3fff, (tex0 >> 14) & 0x3f, (tex0 >> 37) & 0x3fff
    words = vm.view(np.uint32)
    clut = [words[addr32(i & 7, i >> 3, cbp, 1)] & 0xff for i in range(16)]
    t = np.zeros((128, 128), np.uint8)
    for y in range(128):
        for x in range(128):
            a = addr4(x, y, tbp, tbw)
            t[y, x] = clut[(vm[a >> 1] >> (4 * (a & 1))) & 15]
    return t


def trunc(a):  # cvttps2dq
    return np.trunc(np.asarray(a, F)).astype(np.int64)


def draw_tri(fb, tex, v, fst, scis):
    """v: 3 × (x, y, s, t, q) as f32 (x, y in pixels); fb: (224, 640) int red accumulator."""
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
            span(fb, tex, y, left, right, tc, dscan[2:], fst)

    if flat_top:
        a, b = (v0, v1) if m2 else (v1, v0)  # a = edge vertex (left), b supplies the right x
        dd = (d[1], d[2]) if m2 else (d[2], d[1])
        section(t0, b2, (a[0], b[0]), dd, a[2:], a)
    else:
        dd = (d[1], d[0]) if m2 else (d[0], d[1])
        section(t0, b1, (v0[0], v0[0]), dd, v0[2:], v0)
        e = (F(v0[0] + F(dd[0] * dv0[1])), F(v0[0] + F(dd[1] * dv0[1])))
        dd2 = (d[1], d[2]) if m2 else (d[2], d[1])
        section(t1, b2, e, dd2, v1[2:], v1)


def span(fb, tex, y, left, right, tc, ds, fst):
    skip = left & 7
    base = left - skip
    n = right - base
    nv = (n + 7) // 8
    lane = np.arange(8)
    k = np.repeat(np.arange(nv), 8)
    off = np.tile(lane - skip, nv).astype(F)
    if fst:
        u = trunc(tc[0]) + np.tile(trunc(ds[0] * off[:8]), nv) + k * trunc(ds[0] * F(8))
        v = trunc(tc[1]) + np.tile(trunc(ds[1] * off[:8]), nv) + k * trunc(ds[1] * F(8))
    else:
        s = np.empty(nv * 8, F); t = np.empty(nv * 8, F); q = np.empty(nv * 8, F)
        s0 = (tc[0] + (ds[0] * off[:8]).astype(F)).astype(F)
        t0 = (tc[1] + (ds[1] * off[:8]).astype(F)).astype(F)
        q0 = (tc[2] + (ds[2] * off[:8]).astype(F)).astype(F)
        st8 = (ds * F(8)).astype(F)
        for j in range(nv):
            s[j * 8:j * 8 + 8], t[j * 8:j * 8 + 8], q[j * 8:j * 8 + 8] = s0, t0, q0
            s0, t0, q0 = (s0 + st8[0]).astype(F), (t0 + st8[1]).astype(F), (q0 + st8[2]).astype(F)
        u = trunc(s / q) - 0x8000
        v = trunc(t / q) - 0x8000
    sl = slice(skip, skip + right - left)
    u, v = u[sl], v[sl]
    uf, vf = (u & 0xffff) >> 12, (v & 0xffff) >> 12
    x0, y0 = u >> 16, v >> 16
    cl = lambda a: np.clip(a, 0, 127)
    c00, c01 = tex[cl(y0), cl(x0)].astype(np.int64), tex[cl(y0), cl(x0 + 1)].astype(np.int64)
    c10, c11 = tex[cl(y0 + 1), cl(x0)].astype(np.int64), tex[cl(y0 + 1), cl(x0 + 1)].astype(np.int64)
    r0 = c00 + (((c01 - c00) * uf) >> 4)
    r1 = c10 + (((c11 - c10) * uf) >> 4)
    r = r0 + (((r1 - r0) * vf) >> 4)
    fb[y, left:right] = np.minimum(fb[y, left:right] + r, 255)


def main():
    dump, pk, ref = sys.argv[1:4]
    vm = vram(dump)
    prims = pickle.load(open(pk, "rb"))
    red = np.fromfile(ref, np.uint8).reshape(896, 1280)
    texs = {}
    # PCSX2 batches until the state changes; eq.q is decided over the batch
    runs = []
    for f, st, vs in prims:
        if st["frame"] & 0x1ff != 0xd2 or st["prim"] != 0x5c:
            continue
        key = (f, st["tex0"], st["xyoffset"])
        if not runs or runs[-1][0] != key:
            runs.append((key, st, []))
        runs[-1][2].append(vs)
    tiles = {}
    for (f, tex0, xyo), st, tris in runs:
        if tex0 not in texs:
            texs[tex0] = texture(vm, tex0)
        fb = tiles.setdefault(f, np.zeros((224, 640), np.int64))
        qs = {vt[5] for vs in tris for vt in vs}
        eqq = len(qs) == 1
        ox, oy = xyo & 0xffff, (xyo >> 32) & 0xffff
        sc = st["scissor"]
        scis = (sc & 0x7ff, (sc >> 32) & 0x7ff, ((sc >> 16) & 0x7ff) + 1, ((sc >> 48) & 0x7ff) + 1)
        for vs in tris:
            out = []
            for x, y, z, s, t, q, rgba, uv in vs:
                px, py = F(x - ox) * F(1 / 16), F(y - oy) * F(1 / 16)
                if eqq and q != 1.0:
                    ss, tt = F(F(s) / F(q)) * F(1 << 23) - F(0x8000), F(F(t) / F(q)) * F(1 << 23) - F(0x8000)
                    out.append((px, py, ss, tt, F(q)))
                else:
                    out.append((px, py, F(s) * F(1 << 23), F(t) * F(1 << 23), F(q)))
            draw_tri(fb, texs[tex0], out, eqq and vs[0][5] != 1.0, scis)
    for f, fb in sorted(tiles.items()):
        # tile f: the 640×224 window at XYOFFSET − (1728, 1936) of the 1280×896 frame
        xyo = next(k[2] for k, _, _ in runs if k[0] == f)
        gx, gy = (xyo & 0xffff) // 16 - 1728, ((xyo >> 32) & 0xffff) // 16 - 1936
        g = red[gy:gy + 224, gx:gx + 640].astype(np.int64)
        diff = fb != g
        print(f"tile {f} at ({gx},{gy}): {diff.sum()} of {diff.size} reds differ, max |d| {np.abs(fb - g).max()},"
              f" threshold bits differ {((fb > 0x6f) != (g > 0x6f)).sum()}")
        np.save(f"context/p17v/sim_tile{f}.npy", fb)


if __name__ == "__main__":
    main()
