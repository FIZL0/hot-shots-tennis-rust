"""P17m: check the costume noise's VU half against a GS dump. The dump's frame was taken with the deformers frozen
(rate 0) and their amp boosted (context/p17m/dump.py); the model's packets come from `noidump`.

  p17m_noise_gs.py dump.gs model.txt node:freq:prev:amp ...   (hex floats; FIXTURE=out.txt writes the test fixture)

Finds the model's packets in the dump's vertex stream (by length and kick pattern), fits each bone's model→screen
projective matrix to the noise-free vertices, then reprojects the noise vertices with the entries moved as
`hst_sim::noise::deform` moves them (and, for contrast, unmoved or with the lanes read in another order)."""
import struct, sys, os
import numpy as np
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "tools"))
from gsdump import packets

F = lambda h: struct.unpack("<f", struct.pack("<I", int(h, 16)))[0]


def stream(buf):
    """The last frame's runs: per PRIM write, the (x, y, z, kick) of each XYZ write (12.4 pixels, XYOFFSET kept)."""
    frames = [[]]
    for p in packets(buf):
        (frames.append([]) if p[0] == "vsync" else frames[-1].append(p))
    runs = []
    for _, path, data in [f for f in frames if f][-1]:
        o = 0
        while o + 16 <= len(data):
            lo, hi = struct.unpack_from("<QQ", data, o); o += 16
            nloop, pre, flg, nreg = lo & 0x7FFF, (lo >> 46) & 1, (lo >> 58) & 3, ((lo >> 60) & 0xF) or 16
            regs = [(hi >> (4 * i)) & 0xF for i in range(nreg)]
            if pre:
                runs.append([("prim", (lo >> 47) & 0x7FF, len(runs))])
            if flg == 0:
                for _ in range(nloop):
                    for r in regs:
                        a, b = struct.unpack_from("<QQ", data, o); o += 16
                        if r == 0 or (r == 0xE and b & 0xFF == 0):
                            runs.append([("prim", a & 0x7FF, len(runs))])
                        elif r in (4, 5):
                            runs[-1].append((a & 0xFFFF, (a >> 32) & 0xFFFF, b & 0xFFFFFFFF, 1 - ((b >> 47) & 1)))
            elif flg == 1:
                o += ((nloop * nreg + 1) // 2) * 16
            else:
                o += nloop * 16
    # a run: its PRIM, index, and the vertices
    return [(r[0][1], r[0][2], r[1:]) for r in runs if len(r) > 1 and r[0][0] == "prim"]


def model(path):
    pks = []
    for line in open(path):
        t = line.split()
        if t[0] == "P":
            pks.append(dict(material=int(t[1]), group=int(t[2]), verts=[]))
        elif t[0] == "V":
            pks[-1]["verts"].append(dict(kick=int(t[1]), entries=[]))
        else:
            pks[-1]["verts"][-1]["entries"].append(([F(h) for h in t[1:4]], F(t[4]), int(t[5]), F(t[6])))
    return pks


f32 = np.float32


def table():
    r = 0x40490FD0 & 0x7FFFFF | 0x3F800000
    out = []
    for k in range(32):
        r = ((r << 1) ^ (r >> 4 & 1) ^ (r >> 22 & 1)) & 0x7FFFFF | 0x3F800000
        out.append(f32((F(f"{r:x}") + f32(2)) * f32(0.25 if k % 2 == 0 else -0.25)))
    return out


T = table()


def deform(p, weight, freq, prev, amp, lanes):
    """hst_sim::noise::deform (lanes: which lane's noise each lane reads; the port's is (2, 0, 1))."""
    p = [f32(c) for c in p]
    s = [p[0] + p[2], p[1] + p[0], p[2] + p[1]]
    fl, fr = [], []
    for k in range(3):
        x = f32(prev + s[k] * f32(freq))
        whole = f32(np.trunc(x)); fq = f32(x - whole); g = f32(np.trunc(fq - f32(1)))
        fl.append(int(whole + g)); fr.append(f32(fq - g))
    out = []
    for k in range(3):
        j = lanes[k]
        a, b = T[fl[j] & 31], T[(fl[j] + 1) & 31]
        n = f32(a + fr[j] * f32(b - a))
        out.append(f32(p[k] + f32(amp * n) * f32(weight)))
    return out


def main():
    buf = open(sys.argv[1], "rb").read()
    runs = stream(buf)
    pks = model(sys.argv[2])
    noise = {}
    for a in sys.argv[3:]:
        n, fr, pv, am = a.split(":")
        noise[int(n)] = (F(fr), F(pv), F(am))
    # each packet: the runs of its length with its kick pattern
    found = []
    for i, pk in enumerate(pks):
        sig = [v["kick"] for v in pk["verts"]]
        # the untextured pass is the shadow; the blended pass repeats the textured one's vertices
        hits = {tuple(r) for prim, _, r in runs if prim & 0x10 and len(r) == len(sig) and [v[3] for v in r] == sig}
        if len(hits) == 1 and len(sig) >= 6:
            found.append((i, list(hits.pop())))
    print(f"{len(runs)} runs, {len(pks)} packets, {len(found)} found once", file=sys.stderr)
    bones = sorted({e[2] for i, _ in found for v in pks[i]["verts"] for e in v["entries"]})
    col = {b: k for k, b in enumerate(bones)}
    rows = []
    def eqs(entries, x, y):
        out = []
        for c, s in ((0, x), (1, y)):
            # rows x, y and w of each bone's clip matrix (z isn't seen)
            row = np.zeros(12 * len(bones))
            for p, w, b, _ in entries:
                q = np.array([*p, w], dtype=np.float64)
                row[12 * col[b] + 4 * c:12 * col[b] + 4 * c + 4] += q
                row[12 * col[b] + 8:12 * col[b] + 12] -= s * q
            out.append(row)
        return out
    xs = np.array([v[0] for _, r in found for v in r], float); ys = np.array([v[1] for _, r in found for v in r], float)
    cx, cy, sc = float(xs.mean()), float(ys.mean()), float(max(xs.std(), ys.std()))
    for i, r in found:
        if any(e[3] != 0 and pks[i]["group"] in noise for v in pks[i]["verts"] for e in v["entries"]):
            continue
        for v, g in zip(pks[i]["verts"], r):
            rows.append((v["entries"], g, eqs(v["entries"], (g[0] - cx) / sc, (g[1] - cy) / sc)))
    # bones sharing a vertex share the homogeneous scale: one fit per connected group, then drop the vertices
    # off by more than 0.25 px (packets matched to the wrong run) and refit
    up = list(range(len(bones)))
    def root(k):
        while up[k] != k:
            k = up[k]
        return k
    for i, r in found:
        for v in pks[i]["verts"]:
            ks = [col[e[2]] for e in v["entries"]]
            for k in ks[1:]:
                up[root(k)] = root(ks[0])
    A = np.zeros((len(bones), 3, 4))
    def solve(rows):
        M = np.array(rows)
        for c in {root(k) for k in range(len(bones))}:
            cols = [12 * k + j for k in range(len(bones)) if root(k) == c for j in range(12)]
            sub = M[:, cols]
            sub = sub[np.abs(sub).sum(1) > 0]
            if len(sub) < len(cols):
                continue
            sol = np.linalg.svd(sub, full_matrices=False)[2][-1]
            for n, k in enumerate(k for k in range(len(bones)) if root(k) == c):
                A[k] = sol[12 * n:12 * n + 12].reshape(3, 4)
    def project(entries):
        X = sum(A[col[b]] @ np.array([*p, w]) for p, w, b, _ in entries)
        return X[0] / X[2] * sc + cx, X[1] / X[2] * sc + cy
    def err(entries, g):
        x, y = project(entries)
        return np.hypot(x - g[0], y - g[1]) / 16  # pixels
    solve([q for *_, e in rows for q in e])
    keep = [x for x in rows if err(x[0], x[1]) < 0.25]
    solve([q for *_, e in keep for q in e])
    fit = [err(x[0], x[1]) for x in keep]
    print(f"refit on {len(keep)} of {len(rows)} noise-free vertices")
    if os.environ.get("DIAG"):
        for b in map(int, os.environ["DIAG"].split(",")):
            e = [err(x[0], x[1]) for x in rows if any(q[2] == b for q in x[0])]
            k = [err(x[0], x[1]) for x in keep if any(q[2] == b for q in x[0])]
            print(f"bone {b}: {len(e)} vertices, kept {len(k)}, residual kept mean {np.mean(k) if k else 0:.3f}, all mean {np.mean(e) if e else 0:.3f}")
    print(f"fit: {len(fit)} noise-free vertices, {len(bones)} bones, residual px mean {np.mean(fit):.4f} max {np.max(fit):.4f}")
    hyps = {"port (z,x,y)": (2, 0, 1), "unmoved": None, "lanes (x,y,z)": (0, 1, 2), "lanes (y,z,x)": (1, 2, 0)}
    moving = [(i, r) for i, r in found if pks[i]["group"] in noise and any(e[3] for v in pks[i]["verts"] for e in v["entries"])]
    for refit in (False, True):
        print("noise vertices refit with the fit's own vertices:" if refit else "noise vertices reprojected:")
        for name, lanes in hyps.items():
            moved = []
            for i, r in moving:
                fr, pv, am = noise[pks[i]["group"]]
                for v, gv in zip(pks[i]["verts"], r):
                    if any(x[3] for x in v["entries"]):
                        ent = [((deform(p, n, fr, pv, am, lanes) if lanes and n else p), w, b, n) for p, w, b, n in v["entries"]]
                        moved.append((i, ent, gv))
            solve([q for *_, e in keep for q in e] + ([q for _, ent, g in moved for q in eqs(ent, (g[0] - cx) / sc, (g[1] - cy) / sc)] if refit else []))
            per = {}
            for i, ent, g in moved:
                per.setdefault(pks[i]["group"], []).append(err(ent, g))
            base = [err(x[0], x[1]) for x in keep]
            if refit and lanes == (2, 0, 1) and len(sys.argv) and os.environ.get("FIXTURE"):
                # the port's check (hst-sim/tests/noise_gs.rs): per noise vertex its screen position and its entries
                # with their bones' fitted matrices (pixels: x = cx + sc·X0/X2)
                with open(os.environ["FIXTURE"], "w") as out:
                    out.write(f"S {cx / 16!r} {cy / 16!r} {sc / 16!r}\n")
                    for g, (fr, pv, am) in noise.items():
                        out.write(f"N {g} {struct.unpack('<I', struct.pack('<f', fr))[0]:08x} {struct.unpack('<I', struct.pack('<f', pv))[0]:08x} {struct.unpack('<I', struct.pack('<f', am))[0]:08x}\n")
                    for i, r in moving:
                        for v, gv in zip(pks[i]["verts"], r):
                            if any(x[3] for x in v["entries"]):
                                out.write(f"V {pks[i]['group']} {gv[0] / 16!r} {gv[1] / 16!r}\n")
                                for p, w, b, n in v["entries"]:
                                    h = lambda x: f"{struct.unpack('<I', struct.pack('<f', x))[0]:08x}"
                                    out.write("E " + " ".join(h(x) for x in (*p, w, n)) + " " + " ".join(repr(float(x)) for x in A[col[b]].flatten()) + "\n")
            print(f"  {name:14}" + "".join(f"  node {g}: mean {np.mean(e):.3f} max {np.max(e):.3f} px" for g, e in per.items()) + f"  (fit vertices mean {np.mean(base):.3f})")


if __name__ == "__main__":
    main()
