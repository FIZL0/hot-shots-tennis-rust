"""P17s2: check VU1's two lights against a GS dump. For each node of a model, fits the node's rotation R to its
single-bone vertices drawn in the dump and prints the residual of
  GS = ⌊vc · mat · (A + L·max(−Rn·l1, 0) + T·max(−Rn·l2, 0))⌋    (h14 = 0 materials only)
with the given third light T and with T = 0 (the port before P17s2). Light rows from p17s2_capture.py's RAM.

  p17s2_check.py dump.gs model.txt A L T l1x l1y l1z l2x l2y l2z   (model.txt: `noidump model.mdl model.mtl`)"""
import sys, os
import numpy as np
from scipy.optimize import least_squares
from scipy.spatial.transform import Rotation
sys.path.insert(0, os.path.dirname(__file__))
from p17s_light_gs import stream


def load(dump, model):
    runs = stream(open(dump, "rb").read())
    mats, pks = {}, []
    for line in open(model):
        t = line.split()
        if t[0] == "M":
            mats[int(t[1])] = ([float(x) for x in t[2:5]], float(t[7]))
        elif t[0] == "P":
            pks.append(dict(material=int(t[1]), verts=[]))
        elif t[0] == "V":
            pks[-1]["verts"].append([int(t[1]), [int(x) for x in t[2:5]], []])
        elif t[0] == "E":
            pks[-1]["verts"][-1][2].append((int(t[5]), [float(x) for x in t[7:10]]))
    rows = []
    for pk in pks:
        sig = [v[0] for v in pk["verts"]]
        # VU1's culling loop clears the kick of back faces: a drawn kick needs the data's
        hits = {tuple(r) for prim, r in runs if prim & 0x10 and len(r) == len(sig) and all(v[2] <= k for v, k in zip(r, sig)) and sum(v[2] for v in r) * 2 > sum(sig)}
        m, h14 = mats[pk["material"]]
        if len(hits) != 1 or len(sig) < 6 or h14 != 0:
            continue
        for (_, vc, es), g in zip(pk["verts"], hits.pop()):
            if len(es) == 1:
                rows.append((es[0][0], es[0][1], np.array(vc) * m, g[3][:3]))
    return rows


def main():
    a = sys.argv
    rows = load(a[1], a[2])
    A, L, T = map(float, a[3:6])
    l1, l2 = np.array(a[6:9], float), np.array(a[9:12], float)
    print(f"{len(rows)} single-bone vertices", file=sys.stderr)
    tot = {T: [0, 0], 0.0: [0, 0]}
    for node in sorted({r[0] for r in rows}):
        rs = [r for r in rows if r[0] == node]
        if len(rs) < 8:
            continue
        n = np.array([r[1] for r in rs]); vm = np.array([r[2] for r in rs]); g = np.array([r[3] for r in rs], float)
        out = []
        for t in (T, 0.0):
            def pred(rv):
                rn = Rotation.from_rotvec(rv).apply(n)
                k = A + L * np.maximum(-rn @ l1, 0) + t * np.maximum(-rn @ l2, 0)
                return vm * k[:, None]
            best = min((least_squares(lambda rv: (pred(rv) - g - 0.5).ravel(), x0) for x0 in Rotation.random(12, random_state=1).as_rotvec()), key=lambda r: r.cost)
            err = np.floor(pred(best.x)) - g
            exact = int((np.abs(err).max(1) == 0).sum())
            tot[t][0] += exact; tot[t][1] += len(rs)
            out.append(f"T={t:<5} exact {exact:4}/{len(rs):<4} max|err| {np.abs(err).max():3.0f}")
        print(f"node {node:3}  " + "   ".join(out))
    for t, (e, c) in tot.items():
        print(f"T={t}: {e}/{c} exact")


if __name__ == "__main__":
    main()
