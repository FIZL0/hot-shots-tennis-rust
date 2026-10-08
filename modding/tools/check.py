"""Conformance check for an HST-standard character .glb (modding/HST-CHARACTER-STANDARD.md §7).

python3 modding/tools/check.py MODEL.glb [--face-sheet out.png]

Prints one line per check (OK / WARN / FAIL) and exits 1 on any FAIL. --face-sheet renders the head in the rest
pose: neutral, then every HST face channel at weight 1, to look at with an image viewer.
"""
import argparse
import os
import sys

import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
sys.path.insert(0, HERE)
sys.path.insert(0, HERE)
import glb  # noqa: E402
from rerig import CHILD, binds, load_standard, read, worlds  # noqa: E402

REQUIRED = ["joy_eye", "joy_mouth", "anger_eye", "anger_mouth", "sorrow_eye", "sorrow_mouth", "doki_eye", "doki_mouth"]


def check(path):
    J, _ = load_standard()
    g, b = glb.read(path)
    res = []
    ok = lambda c, m, level="FAIL": res.append(("OK" if c else level, m))
    roots = [g["nodes"][i] for i in g["scenes"][0]["nodes"]]
    ok(roots and roots[0].get("name") == "game_space" and np.allclose(roots[0].get("rotation", [0, 0, 0, 1]), [0, 0, 1, 0]),
       "scene root is `game_space`, rotation [0,0,1,0]")
    ok(len(g.get("skins", [])) == 1, f"one skin ({len(g.get('skins', []))})")
    B, par = binds(g, b)
    missing = [n for n in J if n not in B]
    ok(not missing, f"54 core joints present{' — missing ' + ', '.join(missing) if missing else ''}")
    wrong = [f"{n}<{par[n]} (want {J[n][0]})" for n in J if n in B and par[n] != (J[n][0] or "game_space")]
    ok(not wrong, f"core parents{' — ' + '; '.join(wrong) if wrong else ''}")
    # rest = bind
    W, _ = worlds(g)
    G = np.diag([-1.0, -1.0, 1.0, 1.0])
    idx = {g["nodes"][j]["name"]: j for j in g["skins"][0]["joints"]}
    err = max(np.abs(G @ W[idx[n]] - B[n]).max() for n in B)
    ok(err < 1e-3, f"rest pose = bind pose (max diff {err:.2e})")
    # bone axes: each core bone's direction in its own frame matches HST's
    worst = (0, "")
    for n, c in CHILD.items():
        if n in B and c in B:
            d = np.linalg.inv(B[n][:3, :3]) @ (B[c][:3, 3] - B[n][:3, 3])
            dh = np.linalg.inv(J[n][1][:3, :3]) @ (J[c][1][:3, 3] - J[n][1][:3, 3])
            a = np.degrees(np.arccos(np.clip(d @ dh / np.linalg.norm(d) / np.linalg.norm(dh), -1, 1)))
            worst = max(worst, (a, n))
    ok(worst[0] < 10, f"bone axes match HST (worst {worst[1]} {worst[0]:.1f}°)")
    h = -B["Bip01"][1, 3] if "Bip01" in B else 0
    ok(0.4 < h < 1.3, f"Bip01 height {h:.3f} m (HST 0.58–0.95)", "WARN" if 0.2 < h < 2 else "FAIL")
    # skinning data
    feet, over4, badsum, targets = [], 0, 0, set()
    for i, n in enumerate(g["nodes"]):
        if "mesh" not in n:
            continue
        m = g["meshes"][n["mesh"]]
        targets |= set(m.get("extras", {}).get("targetNames", []))
        for p in m["primitives"]:
            a = p["attributes"]
            if "skin" in n:
                feet.append(read(g, b, a["POSITION"])[:, 1].max())
                w = read(g, b, a["WEIGHTS_0"])
                badsum += int((np.abs(w.sum(1) - 1) > 0.01).sum())
                over4 += int("JOINTS_1" in a)
    ok(feet and abs(max(feet)) < 0.06, f"feet at y=0 (lowest point {max(feet) if feet else float('nan'):.3f})", "WARN")
    ok(not over4 and not badsum, f"≤4 weights per vertex, sums 1 ({badsum} bad sums)")
    face = [c for c in REQUIRED if c not in targets]
    ok(not face, f"face channels{' — missing ' + ', '.join(face) if face else ''}", "WARN")
    ok("blink_eye" in targets, "blink_eye", "WARN")
    for r in res:
        print(f"{r[0]:4} {r[1]}")
    return all(r[0] != "FAIL" for r in res)


def face_sheet(path, out):
    import render
    from PIL import Image
    g, b = glb.read(path)
    tex = render.load_textures(g, b)
    Gw = render.globals_(g, {})
    head = np.diag([-1, -1, 1, 1]) @ next(Gw[i] for i, n in enumerate(g["nodes"]) if n["name"] == "Bip01Head")
    names = sorted({t for m in g["meshes"] for t in m.get("extras", {}).get("targetNames", [])})
    show = ["neutral"] + [c for c in ["blink_eye"] + REQUIRED if c in names]
    panels = []
    for want in show:
        # bake the target into the morph meshes' positions, then draw as usual
        g2 = dict(g, accessors=[dict(a) for a in g["accessors"]])
        extra = bytearray(b)
        for m in g["meshes"]:
            tn = m.get("extras", {}).get("targetNames", [])
            if want not in tn:
                continue
            k = tn.index(want)
            for p in m["primitives"]:
                pos = read(g, b, p["attributes"]["POSITION"]).astype(np.float32) + read(g, b, p["targets"][k]["POSITION"]).astype(np.float32)
                off = len(extra)
                extra += pos.tobytes()
                g2.setdefault("bufferViews", g["bufferViews"])
                g2["bufferViews"] = g2["bufferViews"] + [{"buffer": 0, "byteOffset": off, "byteLength": pos.nbytes}]
                a = dict(g2["accessors"][p["attributes"]["POSITION"]], bufferView=len(g2["bufferViews"]) - 1, byteOffset=0)
                a.pop("sparse", None)
                g2["accessors"].append(a)
                p2 = dict(p, attributes=dict(p["attributes"], POSITION=len(g2["accessors"]) - 1))
                m["primitives"] = [p2 if q is p else q for q in m["primitives"]]
        meshes = render.gather(g2, bytes(extra), Gw, tex)
        g, b = glb.read(path)  # undo the in-place primitive swap
        hp = head[:3, 3] * [-1, -1, 1]
        r = 0.16
        box = (np.array([hp[0] - r, hp[1] - r * 0.6, -9]), np.array([hp[0] + r, hp[1] + r * 1.4, 9]))
        img, _ = render.raster(meshes, "front", 220, 220, box)
        panels.append(img)
    Image.fromarray(np.concatenate(panels, 1)).save(out)
    print(out, " | ".join(show))


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("glb")
    ap.add_argument("--face-sheet")
    a = ap.parse_args()
    good = check(a.glb)
    if a.face_sheet:
        face_sheet(a.glb, a.face_sheet)
    sys.exit(0 if good else 1)


if __name__ == "__main__":
    main()
