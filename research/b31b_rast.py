"""The original's tornado add per pixel, rasterised from the GS dump's five draws (field 640×224): MODULATE
(texel × vertex >> 7), bilinear REPEAT on the MTI texels, add Cs·As >> 7 per channel. Writes rast.npy (224×640×3)."""
import struct, sys, numpy as np
sys.path.insert(0, "research/tools")
from gsdump import packets
S = "context/b31b"  # tex0/tex1.rgba: the tornado MTL textures as the port decodes them (128x128 RGBA)
tex = [np.fromfile(f"{S}/tex{k}.rgba", np.uint8).reshape(128, 128, 4).astype(float) for k in (0, 1)]
for t in tex:  # mtl.rs expands alpha 0x80 → 0xff; back to the GS's 0..0x80
    t[..., 3] = np.round(t[..., 3] * 128 / 255)
TBP = {0x2f80: 0, 0x2fc0: 1, 0x3020: 1, 0x2300: 1, 0x2360: 1}
buf = open("context/b31b/on.gs", "rb").read()
frames = [[]]
for p in packets(buf):
    (frames.append([]) if p[0] == "vsync" else frames[-1].append(p))
r, st, rgba, verts, tris = {}, (0, 0, 1), (0, 0, 0, 0), [], []
def kick(x, y, adc):
    global verts
    verts.append((x / 16, y / 16, st, rgba))
    prim = r.get(0, 0)
    if prim & 7 in (3, 4) and len(verts) >= 3:
        if not adc and (r.get(0x42, 0) & 0xFF) == 0x48 and (r.get(6, 0) & 0x3FFF) in TBP:
            tris.append((TBP[r[6] & 0x3FFF], verts[-3:]))
        if prim & 7 == 3: verts = []
for _, path, data in [f for f in frames if f][-1]:
    o = 0
    while o + 16 <= len(data):
        lo, hi = struct.unpack_from("<QQ", data, o); o += 16
        nloop, pre, flg, nreg = lo & 0x7FFF, (lo >> 46) & 1, (lo >> 58) & 3, ((lo >> 60) & 0xF) or 16
        regs = [(hi >> (4 * i)) & 0xF for i in range(nreg)]
        if pre: r[0] = (lo >> 47) & 0x7FF; verts = []
        if flg == 0:
            for _ in range(nloop):
                for g in regs:
                    a, b = struct.unpack_from("<QQ", data, o); o += 16
                    if g == 0xE:
                        r[b & 0xFF] = a
                        if b & 0xFF == 0: verts = []
                    elif g == 0: r[0] = a & 0x7FF; verts = []
                    elif g == 1: rgba = (a & 0xFF, (a >> 32) & 0xFF, b & 0xFF, (b >> 32) & 0xFF)
                    elif g == 2: st = (*struct.unpack("<ff", struct.pack("<Q", a)), struct.unpack("<f", struct.pack("<I", b & 0xFFFFFFFF))[0])
                    elif g in (4, 5): kick(a & 0xFFFF, (a >> 32) & 0xFFFF, (b >> 47) & 1)
        elif flg == 1: o += ((nloop * nreg + 1) // 2) * 16
        else: o += nloop * 16
off = r.get(0x18, 0); ox, oy = (off & 0xFFFF) / 16, ((off >> 32) & 0xFFFF) / 16
print(len(tris), "triangles, offset", ox, oy, file=sys.stderr)
out = np.zeros((224, 640, 3))
def sample(t, u, v):
    x, y = u * 128 - 0.5, v * 128 - 0.5
    x0, y0 = np.floor(x).astype(int), np.floor(y).astype(int); fx, fy = (x - x0)[:, None], (y - y0)[:, None]
    g = lambda i, j: t[j % 128, i % 128]
    return (g(x0, y0) * (1 - fx) + g(x0 + 1, y0) * fx) * (1 - fy) + (g(x0, y0 + 1) * (1 - fx) + g(x0 + 1, y0 + 1) * fx) * fy
for ti, vs in tris:
    P = np.array([[v[0] - ox, v[1] - oy] for v in vs])
    x0, y0 = np.floor(P.min(0)).astype(int); x1, y1 = np.ceil(P.max(0)).astype(int)
    xs, ys = np.meshgrid(np.arange(max(x0, 0), min(x1, 639) + 1), np.arange(max(y0, 0), min(y1, 223) + 1))
    px, py = xs.ravel() + 0.0, ys.ravel() + 0.0  # GS samples at pixel corners (top-left rule)
    (ax, ay), (bx, by), (cx, cy) = P
    den = (bx - ax) * (cy - ay) - (cx - ax) * (by - ay)
    if abs(den) < 1e-9: continue
    w1 = ((px - ax) * (cy - ay) - (cx - ax) * (py - ay)) / den
    w2 = ((bx - ax) * (py - ay) - (px - ax) * (by - ay)) / den
    w0 = 1 - w1 - w2
    m = (w0 >= 0) & (w1 >= 0) & (w2 >= 0)
    if not m.any(): continue
    w = np.stack([w0, w1, w2], 1)[m]
    S_ = np.array([v[2] for v in vs])  # (s, t, q): s/q, t/q interpolated linearly in screen as s, t, q
    s, t_, q = w @ S_[:, 0], w @ S_[:, 1], w @ S_[:, 2]
    col = w @ np.array([v[3] for v in vs], float)  # Gouraud, screen-linear
    T = sample(tex[ti], s / q, t_ / q)
    cs = np.floor(T[:, :3] * col[:, :3] / 128).clip(0, 255)
    As = np.floor(T[:, 3] * col[:, 3] / 128)
    add = np.floor(cs * As[:, None] / 128)
    np.add.at(out, (py[m].astype(int), px[m].astype(int)), add)
out = out.clip(0, 255)
np.save(f"{S}/rast.npy", out)
m = out.max(2) > 3
print("field px", m.sum(), "mean/ch", out[m].mean(0).round(1), "p50/90/99", np.percentile(out[m], [50, 90, 99]).round(0), "sum", round(out.sum() / 3000, 1), "k")
