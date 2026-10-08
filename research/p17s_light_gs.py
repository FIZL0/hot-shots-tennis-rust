"""P17s: a character's VU1 lighting in a GS dump. Finds the model's packets in the dump's last frame (by length and
kick pattern, as research/p17m_noise_gs.py) and prints, per drawn vertex, the GS colour divided by vertex colour ×
material colour (VU1: vc·(light·mat·scale·diffuse + light2·mat·diffuse2 + ambient·mat) + highlight).

  p17s_light_gs.py dump.gs model.txt   (model.txt: `noidump model.mdl model.mtl`)"""
import struct, sys, os
import numpy as np
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "tools"))
from gsdump import packets


def stream(buf):
    """The last frame's runs: (PRIM, [(x, y, kick, (r, g, b, a))])."""
    frames = [[]]
    for p in packets(buf):
        (frames.append([]) if p[0] == "vsync" else frames[-1].append(p))
    runs, rgba = [], (0, 0, 0, 0)
    for _, path, data in [f for f in frames if f][-1]:
        o = 0
        while o + 16 <= len(data):
            lo, hi = struct.unpack_from("<QQ", data, o); o += 16
            nloop, pre, flg, nreg = lo & 0x7FFF, (lo >> 46) & 1, (lo >> 58) & 3, ((lo >> 60) & 0xF) or 16
            regs = [(hi >> (4 * i)) & 0xF for i in range(nreg)]
            if pre:
                runs.append([(lo >> 47) & 0x7FF])
            if flg == 0:
                for _ in range(nloop):
                    for r in regs:
                        a, b = struct.unpack_from("<QQ", data, o); o += 16
                        if r == 0 or (r == 0xE and b & 0xFF == 0):
                            runs.append([a & 0x7FF])
                        elif r == 1:
                            rgba = (a & 0xFF, (a >> 32) & 0xFF, b & 0xFF, (b >> 32) & 0xFF)
                        elif r in (4, 5):
                            runs[-1].append((a & 0xFFFF, (a >> 32) & 0xFFFF, 1 - ((b >> 47) & 1), rgba))
            elif flg == 1:
                o += ((nloop * nreg + 1) // 2) * 16
            else:
                o += nloop * 16
    return [(r[0], r[1:]) for r in runs if len(r) > 1]


def main():
    runs = stream(open(sys.argv[1], "rb").read())
    mats, pks = {}, []
    for line in open(sys.argv[2]):
        t = line.split()
        if t[0] == "M":
            mats[int(t[1])] = ([float(x) for x in t[2:6]], float(t[6]), float(t[7]), t[8])
        elif t[0] == "P":
            pks.append(dict(material=int(t[1]), verts=[]))
        elif t[0] == "V":
            pks[-1]["verts"].append((int(t[1]), [int(x) for x in t[2:6]]))
    rows = []
    for pk in pks:
        sig = [k for k, _ in pk["verts"]]
        hits = {tuple(r) for prim, r in runs if prim & 0x10 and len(r) == len(sig) and [v[2] for v in r] == sig}
        if len(hits) != 1 or len(sig) < 6:
            continue
        m, _, h14, name = mats[pk["material"]]
        for (_, vc), g in zip(pk["verts"], hits.pop()):
            rows.append((pk["material"], name, h14, vc, m, g[3]))
    print(f"{len(rows)} vertices matched", file=sys.stderr)
    # per channel: GS colour / (vc · mat): ambient + light · diffuse
    for mi in sorted({r[0] for r in rows}):
        rs = [r for r in rows if r[0] == mi]
        k = np.array([[r[5][c] / (r[3][c] * r[4][c]) for c in range(3)] for r in rs])
        print(f"mat {mi:2} {rs[0][1]:12} h14 {rs[0][2]:.2f} n {len(rs):4}  min {np.round(k.min(0), 4)}  max {np.round(k.max(0), 4)}")
    if os.environ.get("ROWS"):
        for r in rows:
            print(*r)


if __name__ == "__main__":
    main()
