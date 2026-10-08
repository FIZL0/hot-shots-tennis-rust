"""Software render of a .glb (skinned + rigid meshes, textured, optional animation pose) to PNG.

python3 render.py in.glb out.png [--anim NAME_OR_INDEX] [--times 0,0.25,0.5] [--views front,side]
Each (time, view) is a panel, laid out in a row.
"""
import argparse
import io

import numpy as np
from PIL import Image

import glb


def quat_mat(q):
    x, y, z, w = q
    return np.array([[1 - 2 * (y * y + z * z), 2 * (x * y - z * w), 2 * (x * z + y * w)],
                     [2 * (x * y + z * w), 1 - 2 * (x * x + z * z), 2 * (y * z - x * w)],
                     [2 * (x * z - y * w), 2 * (y * z + x * w), 1 - 2 * (x * x + y * y)]])


def node_local(n, over):
    if "matrix" in n and not over:
        return np.array(n["matrix"]).reshape(4, 4).T
    t = over.get("translation", n.get("translation", [0, 0, 0]))
    r = over.get("rotation", n.get("rotation", [0, 0, 0, 1]))
    s = over.get("scale", n.get("scale", [1, 1, 1]))
    M = np.eye(4)
    M[:3, :3] = quat_mat(r) @ np.diag(s)
    M[:3, 3] = t
    return M


def sample(g, binary, anim, t):
    over = {}
    for ch in [c for c in anim["channels"] if c["target"]["path"] != "weights"]:  # ponytail: morph weights not rendered
        smp = anim["samplers"][ch["sampler"]]
        times = glb.get_accessor(g, binary, smp["input"])
        vals = glb.get_accessor(g, binary, smp["output"])
        tt = min(max(t, times[0]), times[-1])
        i = max(0, min(np.searchsorted(times, tt) - 1, len(times) - 2)) if len(times) > 1 else 0
        if len(times) == 1:
            v = vals[0]
        else:
            f = (tt - times[i]) / max(times[i + 1] - times[i], 1e-9)
            f = min(max(f, 0), 1)
            a, b = vals[i], vals[i + 1]
            if ch["target"]["path"] == "rotation":
                if np.dot(a, b) < 0:
                    b = -b
                v = a * (1 - f) + b * f
                v = v / np.linalg.norm(v)
            else:
                v = a * (1 - f) + b * f
        over.setdefault(ch["target"]["node"], {})[ch["target"]["path"]] = list(v)
    return over


def globals_(g, over):
    G = {}

    def rec(i, P):
        G[i] = P @ node_local(g["nodes"][i], over.get(i, {}))
        for c in g["nodes"][i].get("children", []):
            rec(c, G[i])

    for r in g["scenes"][0]["nodes"]:
        rec(r, np.eye(4))
    return G


def load_textures(g, binary):
    tex = {}
    for i, t in enumerate(g.get("textures", [])):
        img = g["images"][t["source"]]
        v = g["bufferViews"][img["bufferView"]]
        data = binary[v.get("byteOffset", 0): v.get("byteOffset", 0) + v["byteLength"]]
        tex[i] = np.asarray(Image.open(io.BytesIO(data)).convert("RGBA"))
    return tex


def gather(g, binary, G, tex):
    """-> list of (positions (n,3) world, normals, uvs, indices, texture or None, alpha mode)."""
    out = []
    for i, n in enumerate(g["nodes"]):
        if "mesh" not in n:
            continue
        if "skin" in n:
            sk = g["skins"][n["skin"]]
            ibm = glb.get_accessor(g, binary, sk["inverseBindMatrices"]).reshape(-1, 4, 4).transpose(0, 2, 1)
            J = np.array([G[j] @ ibm[k] for k, j in enumerate(sk["joints"])])
        for p in g["meshes"][n["mesh"]]["primitives"]:
            a = p["attributes"]
            P = glb.get_accessor(g, binary, a["POSITION"]).astype(np.float64)
            N = glb.get_accessor(g, binary, a["NORMAL"]).astype(np.float64)
            U = glb.get_accessor(g, binary, a["TEXCOORD_0"]).astype(np.float64)
            I = glb.get_accessor(g, binary, p["indices"]).reshape(-1, 3)
            Ph = np.c_[P, np.ones(len(P))]
            if "skin" in n:
                jj = glb.get_accessor(g, binary, a["JOINTS_0"]).astype(int)
                ww = glb.get_accessor(g, binary, a["WEIGHTS_0"]).astype(np.float64)
                M = np.einsum("vk,vkij->vij", ww, J[jj])
                Pw = np.einsum("vij,vj->vi", M, Ph)[:, :3]
                Nw = np.einsum("vij,vj->vi", M[:, :3, :3], N)
            else:
                Pw = (Ph @ G[i].T)[:, :3]
                Nw = N @ G[i][:3, :3].T
            mat = g["materials"][p["material"]]
            ti = mat["pbrMetallicRoughness"].get("baseColorTexture", {}).get("index")
            out.append((Pw, Nw, U, I, tex.get(ti), mat.get("alphaMode", "OPAQUE")))
    return out


def raster(meshes, view, W=320, H=480, box=None):
    """view: 'front' (looking at the model's face, from +Z), 'side' (from +X), 'back', 'top'."""
    R = {"front": np.eye(3), "back": np.diag([-1, 1, -1]),
         "side": np.array([[0, 0, -1], [0, 1, 0], [1, 0, 0]]),
         "top": np.array([[1, 0, 0], [0, 0, -1], [0, 1, 0]])}[view]
    allp = np.concatenate([m[0] for m in meshes]) @ R.T
    lo, hi = (allp.min(0), allp.max(0)) if box is None else box
    sc = 0.92 * min(W / max(hi[0] - lo[0], 1e-6), H / max(hi[1] - lo[1], 1e-6))
    cx, cy = (lo[0] + hi[0]) / 2, (lo[1] + hi[1]) / 2
    img = np.full((H, W, 3), 60, np.float64)
    zb = np.full((H, W), -np.inf)
    light = np.array([0.3, 0.5, 0.8])
    light /= np.linalg.norm(light)
    for P, N, U, I, T, am in sorted(meshes, key=lambda m: m[5] == "BLEND"):
        Q = P @ R.T
        Nv = N @ R.T
        sx = (Q[:, 0] - cx) * sc + W / 2
        sy = H / 2 - (Q[:, 1] - cy) * sc
        for tri in I:
            x, y, z = sx[tri], sy[tri], Q[tri, 2]
            x0, x1 = int(max(np.floor(x.min()), 0)), int(min(np.ceil(x.max()), W - 1))
            y0, y1 = int(max(np.floor(y.min()), 0)), int(min(np.ceil(y.max()), H - 1))
            if x1 < x0 or y1 < y0:
                continue
            d = (y[1] - y[2]) * (x[0] - x[2]) + (x[2] - x[1]) * (y[0] - y[2])
            if abs(d) < 1e-9:
                continue
            gx, gy = np.meshgrid(np.arange(x0, x1 + 1) + 0.5, np.arange(y0, y1 + 1) + 0.5)
            l0 = ((y[1] - y[2]) * (gx - x[2]) + (x[2] - x[1]) * (gy - y[2])) / d
            l1 = ((y[2] - y[0]) * (gx - x[2]) + (x[0] - x[2]) * (gy - y[2])) / d
            l2 = 1 - l0 - l1
            ins = (l0 >= -1e-6) & (l1 >= -1e-6) & (l2 >= -1e-6)
            if not ins.any():
                continue
            zz = l0 * z[0] + l1 * z[1] + l2 * z[2]
            sub = zb[y0:y1 + 1, x0:x1 + 1]
            vis = ins & (zz > sub)
            if not vis.any():
                continue
            n = l0[..., None] * Nv[tri[0]] + l1[..., None] * Nv[tri[1]] + l2[..., None] * Nv[tri[2]]
            n /= np.linalg.norm(n, axis=-1, keepdims=True) + 1e-9
            shade = 0.45 + 0.55 * np.abs(n @ light)
            if T is not None:
                u = l0 * U[tri[0], 0] + l1 * U[tri[1], 0] + l2 * U[tri[2], 0]
                v = l0 * U[tri[0], 1] + l1 * U[tri[1], 1] + l2 * U[tri[2], 1]
                th, tw = T.shape[:2]
                tx = (np.floor(u * tw).astype(int)) % tw
                ty = (np.floor(v * th).astype(int)) % th
                col = T[ty, tx].astype(np.float64)
                if am != "OPAQUE":
                    vis &= col[..., 3] >= 128
                col = col[..., :3]
            else:
                col = np.full(gx.shape + (3,), 200.0)
            reg = img[y0:y1 + 1, x0:x1 + 1]
            reg[vis] = (col * shade[..., None])[vis]
            sub[vis] = zz[vis]
    return np.clip(img, 0, 255).astype(np.uint8), (lo, hi)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("glb")
    ap.add_argument("out")
    ap.add_argument("--anim")
    ap.add_argument("--times", default="0")
    ap.add_argument("--views", default="front,side")
    ap.add_argument("--size", default="320x480")
    a = ap.parse_args()
    g, binary = glb.read(a.glb)
    tex = load_textures(g, binary)
    W, H = map(int, a.size.split("x"))
    anim = None
    if a.anim is not None:
        anims = g.get("animations", [])
        anim = anims[int(a.anim)] if a.anim.isdigit() else next(x for x in anims if x["name"] == a.anim)
    panels = []
    box = {}
    for t in [float(x) for x in a.times.split(",")]:
        over = sample(g, binary, anim, t) if anim else {}
        meshes = gather(g, binary, globals_(g, over), tex)
        for v in a.views.split(","):
            img, bx = raster(meshes, v, W, H, box.get(v))
            box.setdefault(v, (bx[0] - 0.15 * (bx[1] - bx[0]), bx[1] + 0.15 * (bx[1] - bx[0])) if anim else bx)
            panels.append(img)
    Image.fromarray(np.concatenate(panels, 1)).save(a.out)


if __name__ == "__main__":
    main()
