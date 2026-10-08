"""P17s6: bg-dir models (skies, `_bg`, `_clo`) draw on VU1's unlit path. For each packet of a `noidump` model, finds
a GS run in the dump's last frame of the same length with GS = min(⌊vc · mat · K⌋, 255) for every vertex, K = A + L
(the light block's ambient + light; 1.2 on court 4).

  p17s6_unlit_gs.py dump.gs K model.txt...   (model.txt: `noidump model.mdl model.MTL`)"""
import sys, os
import numpy as np
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from p17s_light_gs import stream
runs = [np.array([v[3][:3] for v in r], float) for _, r in stream(open(sys.argv[1], 'rb').read())]
K = float(sys.argv[2])
for path in sys.argv[3:]:
    mats, pks = {}, []
    for line in open(path):
        t = line.split()
        if t[0] == 'M': mats[int(t[1])] = [float(x) for x in t[2:5]]
        elif t[0] == 'P': pks.append((int(t[1]), []))
        elif t[0] == 'V': pks[-1][1].append([int(x) for x in t[2:5]])
    exact = []
    for i, (m, vc) in enumerate(pks):
        want = np.minimum(np.floor(np.array(vc, float) * mats[m] * K), 255)
        best = max(((want == g).all(1).sum() for g in runs if len(g) == len(want)), default=0)
        print(os.path.basename(path), 'packet', i, 'verts', len(want), 'unsaturated', int((want < 255).sum()), 'exact', best)
        exact.append(best == len(want))
    print(os.path.basename(path), 'packets exact', sum(exact), '/', len(exact))
