"""P17s3: VU1 lights a two-bone vertex with its pre-weighted normals summed as is (|n_e| = w_e, no normalise).
Fits every bone's light direction (bone space) at once from a character's lit vertices in a GS dump (linear:
d = Σ n_e·L_bone(e)), then predicts each vertex: GS = ⌊vc·A·(1 + max(d, 0))⌋ (light = ambient on court 10). The
hypothesis holds when every |L| comes out 1 and the vertices match; the same with Σ n_e normalised for comparison.

  p17s3_multibone.py dump.gs model.txt A_r A_g A_b   (model.txt: noidump's M/P/V/E lines; A = ambient = light)"""
import sys, os, struct
import numpy as np
sys.path.insert(0, os.path.dirname(__file__))
from p17s_light_gs import stream


def model(path):
    pks = []
    for line in open(path):
        t = line.split()
        if t[0] == "P":
            pks.append([])
        elif t[0] == "V":
            pks[-1].append((int(t[1]), [int(x) for x in t[2:6]], []))
        elif t[0] == "E":
            w = struct.unpack("<f", bytes.fromhex(t[4])[::-1])[0]
            pks[-1][-1][2].append((int(t[5]), w, np.array([float(x) for x in t[7:10]])))
    return pks


def main():
    runs = stream(open(sys.argv[1], "rb").read())
    amb = np.array([float(x) for x in sys.argv[3:6]])
    verts = []  # (entries, vc, gs rgb)
    for pk in model(sys.argv[2]):
        sig = [k for k, _, _ in pk]
        hits = {tuple(r) for prim, r in runs if prim & 0x10 and len(r) == len(sig) and [v[2] for v in r] == sig}
        if len(hits) != 1 or len(sig) < 6:
            continue
        for (_, vc, es), g in zip(pk, hits.pop()):
            verts.append((es, np.array(vc[:3], float), np.array(g[3][:3], float)))
    print(f"{len(verts)} vertices matched")
    nodes = sorted({e[0] for es, _, _ in verts for e in es})
    ix = {n: i for i, n in enumerate(nodes)}
    d_of = lambda vc, g: ((g + 0.5) / (vc * amb) - 1.0).mean()
    pred = lambda vc, d: np.floor(vc * amb * (1.0 + max(d, 0.0)) + 1e-4)
    for norm in (False, True):
        scale = lambda es: np.linalg.norm(sum(x for _, _, x in es)) if norm else 1.0
        a, b = [], []
        for es, vc, g in verts:
            if (d := d_of(vc, g)) > 0.03:
                row = np.zeros(3 * len(nodes))
                for n, _, x in es:
                    row[3 * ix[n]:3 * ix[n] + 3] += x / scale(es)
                a.append(row); b.append(d)
        L = np.linalg.lstsq(np.array(a), np.array(b), rcond=None)[0].reshape(-1, 3)
        print("normalised" if norm else "as is", "|L|:", {n: round(float(np.linalg.norm(L[ix[n]])), 3) for n in nodes})
        for k, name in ((1, "single"), (2, "two-bone")):
            e = np.array([np.abs(pred(vc, sum(x @ L[ix[n]] for n, _, x in es) / scale(es)) - g).max() for es, vc, g in verts if len(es) == k])
            print(f"  {name:8} n {len(e):4}: exact {(e == 0).sum():4}  ±1 {(e == 1).sum():4}  worse {(e > 1).sum():3} (max {e.max():.0f})")


if __name__ == "__main__":
    main()
