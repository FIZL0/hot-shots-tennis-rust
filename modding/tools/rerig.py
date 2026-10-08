"""Rerig a character .glb onto the HST player skeleton (modding/HST-CHARACTER-STANDARD.md).

python3 modding/tools/rerig.py SRC.glb OUT.glb [--preview DONOR.glb] [--motions NAME,NAME…]

SRC is any skinned glTF whose joints are named like the 3ds Max Biped (`Bip01 L Thigh` or `Bip01LThigh`). Output:
scene root `game_space` (180° about Z) over HST game space (metres, Y down, feet at y = 0, facing +Z); the 54 core
joints with HST's names, parents and bone axes, placed at the source's own joint positions; every other source node
kept as an extra under its nearest kept ancestor; weights rebound; face morph targets renamed to HST's channels.
--preview copies an HST character's motions (as the game binds them: rotations as is, Bip01 position scaled by
height, other positions by bone length) so the result can be rendered and checked.
"""
import argparse
import re
import json
import os
import sys

import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
sys.path.insert(0, HERE)
import glb  # noqa: E402

G = np.diag([-1.0, -1.0, 1.0, 1.0])  # glTF Y-up world <-> HST game space (its own inverse)
GAME_SPACE = [0.0, 0.0, 1.0, 0.0]
# the child each core joint's bone points at (its direction fixes the joint's swing)
CHILD = {"Bip01Spine": "Bip01Spine1", "Bip01Spine1": "Bip01Spine2", "Bip01Spine2": "Bip01Neck", "Bip01Neck": "Bip01Head"}
for s in "LR":
    CHILD.update({f"Bip01{s}Clavicle": f"Bip01{s}UpperArm", f"Bip01{s}UpperArm": f"Bip01{s}Forearm",
                  f"Bip01{s}Forearm": f"Bip01{s}Hand", f"Bip01{s}Hand": f"Bip01{s}Finger2",
                  f"Bip01{s}Thigh": f"Bip01{s}Calf", f"Bip01{s}Calf": f"Bip01{s}Foot", f"Bip01{s}Foot": f"Bip01{s}Toe0"})
    for f in range(5):
        CHILD.update({f"Bip01{s}Finger{f}": f"Bip01{s}Finger{f}1", f"Bip01{s}Finger{f}1": f"Bip01{s}Finger{f}2"})
# HST face channel ← source target names to try, in order (an approximation is better than a frozen face)
# HST face channel <- source expression words to try, best first (an approximation beats a frozen face). Words are
# target names with the part (_eye/_mouth), side markers (L/R), variant letters and digits stripped.
FACE_WORDS = {"blink": ["blink", "close", "wink"],
              "joy": ["joy", "smile", "laugh", "bigsmile", "grin", "joe"],
              "anger": ["anger", "angry", "vexing", "muka", "scowl", "miken", "frown", "katamayu", "provocation", "clench"],
              "sorrow": ["sorrow", "sad", "cry", "worry", "disapoint", "beaten", "tukare", "nihil", "pain"],
              "doki": ["doki", "surprise", "suprise", "amaze", "awa", "shy", "open", "o", "ayo", "oh"]}


def read(g, b, i):
    """Accessor i as a dense array (glb.get_accessor plus sparse, bufferless and strided accessors)."""
    a = g["accessors"][i]
    n = glb._NC[a["type"]]
    dt = glb._DT[a["componentType"]]
    if "bufferView" in a:
        v = g["bufferViews"][a["bufferView"]]
        st = v.get("byteStride", 0)
        if st and st != n * np.dtype(dt).itemsize:
            off = v.get("byteOffset", 0) + a.get("byteOffset", 0)
            raw = np.frombuffer(b, np.uint8, (a["count"] - 1) * st + n * np.dtype(dt).itemsize, off)
            arr = np.stack([raw[k * st: k * st + n * np.dtype(dt).itemsize].view(dt) for k in range(a["count"])])
        else:
            arr = np.array(glb.get_accessor(g, b, i)).reshape(a["count"], n)
            arr = arr.astype(dt) if not a.get("normalized") else arr
    else:
        arr = np.zeros((a["count"], n), dt)
    if "sparse" in a:
        sp = a["sparse"]
        arr = arr.copy()
        iv = g["bufferViews"][sp["indices"]["bufferView"]]
        ix = np.frombuffer(b, glb._DT[sp["indices"]["componentType"]], sp["count"], iv.get("byteOffset", 0) + sp["indices"].get("byteOffset", 0))
        vv = g["bufferViews"][sp["values"]["bufferView"]]
        arr[ix.astype(np.int64)] = np.frombuffer(b, dt, sp["count"] * n, vv.get("byteOffset", 0) + sp["values"].get("byteOffset", 0)).reshape(-1, n)
    if a.get("normalized") and arr.dtype.kind != "f":
        arr = arr.astype(np.float64) / np.iinfo(dt).max
    return arr if n > 1 else arr.ravel()


def norm(name):
    return name.replace(" ", "")


def quat_mat(q):
    x, y, z, w = q
    return np.array([[1 - 2 * (y * y + z * z), 2 * (x * y - z * w), 2 * (x * z + y * w)],
                     [2 * (x * y + z * w), 1 - 2 * (x * x + z * z), 2 * (y * z - x * w)],
                     [2 * (x * z - y * w), 2 * (y * z + x * w), 1 - 2 * (x * x + y * y)]])


def mat_quat(m):
    from scipy.spatial.transform import Rotation
    return Rotation.from_matrix(m).as_quat()  # x y z w


def node_mat(n):
    if "matrix" in n:
        return np.array(n["matrix"]).reshape(4, 4).T
    M = np.eye(4)
    M[:3, :3] = quat_mat(n.get("rotation", [0, 0, 0, 1])) @ np.diag(n.get("scale", [1, 1, 1]))
    M[:3, 3] = n.get("translation", [0, 0, 0])
    return M


def worlds(g):
    parent = {c: i for i, n in enumerate(g["nodes"]) for c in n.get("children", [])}
    W = {}

    def world(i):
        if i not in W:
            W[i] = (world(parent[i]) if i in parent else np.eye(4)) @ node_mat(g["nodes"][i])
        return W[i]

    for i in range(len(g["nodes"])):
        world(i)
    return W, parent


def skin_binds(g, b):
    """Per skinned node index: its bind matrix in game space; and per skin: K (mesh space -> glTF world at bind)."""
    W, _ = worlds(g)
    B, K = {}, []
    for s in g.get("skins", []):
        ibm = read(g, b, s["inverseBindMatrices"]).reshape(-1, 4, 4).transpose(0, 2, 1)
        names = [norm(g["nodes"][j]["name"]) for j in s["joints"]]
        k = names.index("Bip01Pelvis") if "Bip01Pelvis" in names else 0
        Ks = W[s["joints"][k]] @ ibm[k]
        K.append(Ks)
        for j, m in zip(s["joints"], ibm):
            B.setdefault(j, G @ Ks @ np.linalg.inv(m))
    return B, K


def binds(g, b):
    """Name -> bind matrix (game space) for skin joints, and name -> parent name; for make_skeleton/check."""
    B, _ = skin_binds(g, b)
    _, parent = worlds(g)
    nodes = g["nodes"]
    return ({norm(nodes[j]["name"]): m for j, m in B.items()},
            {norm(nodes[j]["name"]): norm(nodes[parent[j]]["name"]) if j in parent else None for j in B})


def swing(a, b):
    """Shortest rotation taking direction a to direction b."""
    a, b = a / np.linalg.norm(a), b / np.linalg.norm(b)
    v, c = np.cross(a, b), float(np.dot(a, b))
    if c < -0.9999:
        p = np.cross(a, [1, 0, 0]) if abs(a[0]) < 0.9 else np.cross(a, [0, 1, 0])
        p /= np.linalg.norm(p)
        return 2 * np.outer(p, p) - np.eye(3)
    vx = np.array([[0, -v[2], v[1]], [v[2], 0, -v[0]], [-v[1], v[0], 0]])
    return np.eye(3) + vx + vx @ vx / (1 + c)


def load_standard():
    s = json.load(open(os.path.join(ROOT, "hst_skeleton.json")))
    J = {}
    for j in s["joints"]:
        M = np.eye(4)
        M[:3, :3] = quat_mat(j["bind_rotation"])
        M[:3, 3] = j["bind_position"]
        J[j["name"]] = (j["parent"], M)
    return J, s


def fit(J, src):
    """The HST core skeleton fitted to source binds `src` (name -> game-space matrix): name -> new bind matrix."""
    Ph = {n: m[:3, 3] for n, (_, m) in J.items()}
    par = {n: p for n, (p, _) in J.items()}
    s = 1.0
    if "Bip01Head" in src and "Bip01Pelvis" in src:
        s = np.linalg.norm(src["Bip01Head"][:3, 3] - src["Bip01Pelvis"][:3, 3]) / np.linalg.norm(Ph["Bip01Head"] - Ph["Bip01Pelvis"])
    P, D = {}, {}

    def anchor(n):  # first source-mapped joint down n's CHILD chain, with the HST path length to it
        L, c = 0.0, n
        while c not in src:
            k = CHILD.get(c)
            if k is None:
                return None, 0
            L += np.linalg.norm(Ph[k] - Ph[c])
            c = k
        return c, L

    def pos(n):
        if n in P:
            return P[n]
        p = par[n]
        if n in src:
            P[n] = src[n][:3, 3]
        elif p is None:  # no Bip01: sits over the pelvis as in HST
            P[n] = pos("Bip01Pelvis") - (Ph["Bip01Pelvis"] - Ph[n]) * s
        else:
            a, L = anchor(n)
            if a is not None:  # on the line to the next mapped joint, at HST's ratio
                l0 = np.linalg.norm(Ph[n] - Ph[p])
                P[n] = pos(p) + l0 / (l0 + L) * (src[a][:3, 3] - pos(p))
            else:
                P[n] = pos(p) + rot(p) @ (Ph[n] - Ph[p]) * s
        return P[n]

    def rot(n):  # swing from HST's bind frame to this skeleton's
        if n in D:
            return D[n]
        c = CHILD.get(n)
        if c and anchor(c)[0] is not None:
            D[n] = swing(Ph[c] - Ph[n], pos(c) - pos(n))
        elif c and n + "Nub" in src:  # a leaf chain with only an end marker (single-segment fingers)
            D[n] = swing(Ph[c] - Ph[n], src[n + "Nub"][:3, 3] - pos(n))
        else:
            D[n] = rot(par[n]) if par[n] else np.eye(3)
        return D[n]

    out = {}
    for n, (_, Mh) in J.items():
        M = np.eye(4)
        M[:3, :3] = rot(n) @ Mh[:3, :3]
        M[:3, 3] = pos(n)
        out[n] = M
    return out


def face_word(name):
    """'close_eyeL' -> ('close', 'eye', 'L', 0); 'sorrowA_mouth' -> ('sorrow', 'mouth', '', 1); None if no part."""
    m = re.fullmatch(r"(?:([LR])_?)?([a-zA-Z-]+?)(\d*|[A-C])?(?:_([LR]|LR))?_(eye|mouth)(\d*|[LR])", name)
    if not m:
        return None
    side = m.group(1) or m.group(4) or (m.group(6) if m.group(6) in ("L", "R") else "")
    var = m.group(3) or (m.group(6) if m.group(6) not in ("L", "R") else "") or ""
    rank = 0 if not var else (int(var) if var.isdigit() else " ABC".index(var))
    return m.group(2).lower().rstrip("-"), m.group(5), side, rank


def face_names(names):
    """Source target names -> (short names, aliases [(HST channel, [source indices summed])] to append)."""
    short = [n.split("\x01")[-1] for n in names]
    parsed = [face_word(n) for n in short]
    alias = []
    for ch, words in FACE_WORDS.items():
        for part in ("eye", "mouth"):
            want = f"{ch}_{part}"
            if want in short or (ch == "blink" and part == "mouth"):
                continue
            for w in words:
                hits = [(p[3], p[2], i) for i, p in enumerate(parsed) if p and p[0] == w and p[1] == part]
                if not hits:
                    continue
                best = min(h[0] for h in hits)
                hits = [h for h in hits if h[0] == best]
                both = [h for h in hits if h[1] == ""] or ([h for h in hits if h[1] == "LR"]) or hits  # L+R halves sum
                alias.append((want, [h[2] for h in (both if both[0][1] in ("", "LR") else hits)]))
                break
    return short, alias


def rerig(src_path, out_path, donor=None, motions=None):
    J, std = load_standard()
    g, b = glb.read(src_path)
    nodes = g["nodes"]
    W, parent = worlds(g)
    SB, K = skin_binds(g, b)
    by_name = {}
    for j in SB:
        by_name.setdefault(norm(nodes[j]["name"]), j)
    core_src = {n: by_name[n] for n in J if n in by_name}
    new = fit(J, {n: SB[j] for n, j in core_src.items()})

    # source nodes above the core (scene roots, locators) are dropped; the rest are kept as extras
    mapped_nodes = set(core_src.values())

    def under_core(i):  # has a mapped core joint above it
        while i in parent:
            i = parent[i]
            if i in mapped_nodes:
                return True
        return False

    above = set()
    for j in core_src.values():
        i = j
        while i in parent:
            i = parent[i]
            if i not in mapped_nodes and not under_core(i):
                above.add(i)
    skinned_mesh = {i for i, n in enumerate(nodes) if "mesh" in n and "skin" in n}
    out = glb.Builder()
    out.g["asset"]["generator"] = "HST-MODS rerig"
    root = out.node(name="game_space", rotation=GAME_SPACE, children=[])
    idx, M = {}, {}
    def depth(n):
        return 0 if J[n][0] is None else 1 + depth(J[n][0])

    order = sorted(J, key=depth)  # parents first
    for n in order:
        p = J[n][0]
        M[n] = new[n]
        loc = np.linalg.inv(M[p]) @ M[n] if p else M[n]
        idx[n] = out.node(name=n, translation=[float(x) for x in loc[:3, 3]], rotation=[float(x) for x in mat_quat(loc[:3, :3])], children=[])
        out.g["nodes"][idx[p] if p else root]["children"].append(idx[n])
    core_of = {j: n for n, j in core_src.items()}
    src_new = {j: idx[n] for j, n in core_of.items()}
    used = set(J)

    def keep(i):  # new node for source node i (extras), created on demand under its nearest kept ancestor
        if i in src_new:
            return src_new[i]
        if i in above or i in skinned_mesh:
            return None
        p = parent.get(i)
        while p is not None and keep(p) is None:
            p = parent.get(p)
        pn = keep(p) if p is not None else None
        Mi = SB[i] if i in SB else G @ W[i]
        Mp = np.eye(4) if pn is None else M[pn]
        loc = np.linalg.inv(Mp) @ Mi
        name = norm(nodes[i].get("name", f"node{i}"))
        if name in used:
            name = "x_" + name
        used.add(name)
        sc = np.linalg.norm(loc[:3, :3], axis=0)
        if np.linalg.det(loc[:3, :3]) < 0:  # mirrored helper (3ds Max): carry the flip as a negative scale
            sc[0] = -sc[0]
        k = out.node(name=name, translation=[float(x) for x in loc[:3, 3]], rotation=[float(x) for x in mat_quat(loc[:3, :3] / sc)],
                     scale=None if np.allclose(sc, 1, atol=1e-4) else [float(x) for x in sc], children=[])
        out.g["nodes"][root if pn is None else pn]["children"].append(k)
        src_new[i] = k
        M[k] = Mi
        return k

    for n in M.copy():  # core matrices also by new node index, for keep()
        M[idx[n]] = M[n]
    for i in range(len(nodes)):
        if i not in above and i not in skinned_mesh and i not in src_new and (i in SB or "mesh" in nodes[i]):
            keep(i)

    # one skin: core joints then every kept skinned extra
    joints = [idx[n] for n in order] + sorted({src_new[j] for j in SB if j in src_new and j not in core_of})
    jpos = {k: t for t, k in enumerate(joints)}
    fallback = jpos[idx["Bip01Pelvis"]]
    ibm = np.stack([np.linalg.inv(M[k]).T for k in joints])
    skin = out.add("skins", {"joints": joints, "inverseBindMatrices": out.accessor(ibm, glb.FLOAT, "MAT4"), "skeleton": idx["Bip01"]})

    # textures, materials
    imgs = []
    for im in g.get("images", []):
        v = g["bufferViews"][im["bufferView"]]
        data = b[v.get("byteOffset", 0): v.get("byteOffset", 0) + v["byteLength"]]
        imgs.append(out.add("images", {"bufferView": out.view(bytes(data)), "mimeType": im.get("mimeType", "image/png"), "name": im.get("name", "")}))
    for key in ("samplers", "materials"):
        if key in g:
            out.g[key] = json.loads(json.dumps(g[key]))
    if "textures" in g:
        out.g["textures"] = [dict(t, source=imgs[t["source"]]) for t in g["textures"]]
    for key in ("extensionsUsed", "extensionsRequired"):
        if key in g:
            out.g[key] = g[key]

    def acc(i, fn=None, comp=None):
        a = g["accessors"][i]
        arr = read(g, b, i)
        if fn is not None:
            arr = fn(arr)
        c = comp or (glb.FLOAT if a.get("normalized") else a["componentType"])
        return out.accessor(arr, c, a["type"], minmax=a["type"] == "VEC3")

    def lin(Kx):
        A = (G @ Kx)[:3, :3]
        return A

    face_meshes = []
    for i in sorted(skinned_mesh) + sorted(k for k in src_new if "mesh" in nodes[k]):
        n = nodes[i]
        m = g["meshes"][n["mesh"]]
        skinned = "skin" in n
        Kx = K[n["skin"]] if skinned else np.eye(4)
        A = lin(Kx) if skinned else np.eye(3)
        T = (G @ Kx)[:3, 3] if skinned else np.zeros(3)
        sj = g["skins"][n["skin"]]["joints"] if skinned else []
        jmap = np.array([jpos.get(src_new.get(j), fallback) for j in sj] or [0], dtype=np.uint16)
        names = m.get("extras", {}).get("targetNames", [])
        short, alias = face_names(names) if names else ([], [])
        prims = []
        cache = {}
        for pr in m["primitives"]:
            at = {}
            for k, v in pr["attributes"].items():
                if not skinned:
                    at[k] = acc(v)
                elif k == "POSITION":
                    at[k] = acc(v, lambda x: x @ A.T + T)
                elif k in ("NORMAL", "TANGENT"):
                    def f(x, k=k):
                        y = x[:, :3] @ A.T
                        y = y / np.maximum(np.linalg.norm(y, axis=1, keepdims=True), 1e-12)
                        return np.hstack([y, x[:, 3:]]) if k == "TANGENT" else y
                    at[k] = acc(v, f)
                elif k.startswith("JOINTS_"):
                    at[k] = acc(v, lambda x: jmap[x.astype(np.int64)], glb.USHORT)
                else:
                    at[k] = acc(v)
            p2 = {k: v for k, v in pr.items() if k not in ("attributes", "targets", "indices")}
            p2["attributes"] = at
            if "indices" in pr:
                p2["indices"] = acc(pr["indices"])
            if "targets" in pr:
                def tgt(t):
                    d = {}
                    for k, v in t.items():
                        if (k, v) not in cache:
                            if k in ("POSITION", "NORMAL", "TANGENT") and skinned:
                                cache[k, v] = acc(v, lambda x: x[:, :3] @ A.T)
                            else:
                                cache[k, v] = acc(v)
                        d[k] = cache[k, v]
                    return d
                ts = [tgt(t) for t in pr["targets"]]
                for _, src in alias:
                    if len(src) == 1:
                        ts.append(ts[src[0]])
                        continue
                    d = {}
                    for k in pr["targets"][src[0]]:
                        d[k] = out.accessor(sum(np.asarray(glb.get_accessor(out.g, bytes(out.bin), ts[j][k]), np.float64) for j in src), glb.FLOAT, "VEC3")
                    ts.append(d)
                p2["targets"] = ts
            prims.append(p2)
        m2 = {"name": m.get("name", ""), "primitives": prims}
        if m.get("extras"):  # e.g. `noise` deformers (standard §3a)
            m2["extras"] = dict(m["extras"])
        if names:
            m2["extras"] = dict(m2.get("extras", {}), targetNames=short + [a for a, _ in alias])
            m2["weights"] = [0.0] * len(m2["extras"]["targetNames"])
        mi = out.add("meshes", m2)
        if skinned:
            k = out.node(name=n.get("name", "mesh"), mesh=mi, skin=skin)
            out.g["scenes"][0]["nodes"].append(k)
        else:
            k = src_new[i]
            out.g["nodes"][k]["mesh"] = mi
        if names:
            face_meshes.append((k, m2["extras"]["targetNames"]))
    out.g["scenes"][0]["nodes"].insert(0, root)
    for n in out.g["nodes"]:
        if not n.get("children"):
            n.pop("children", None)

    if donor:
        preview(out, donor, idx, M, face_meshes, motions)
    out.g["extras"] = {"hst_standard": 1, "source": os.path.basename(src_path),
                       "mapped": sorted(core_src), "synthesized": sorted(set(J) - set(core_src))}
    out.write(out_path)
    return out


def preview(out, donor, idx, M, face_meshes, motions):
    """Copy an HST character's motions onto the new skeleton the way the game binds them (hst_sim::pose::Clip)."""
    dg, db = glb.read(donor)
    dB, dpar = binds(dg, db)
    rest_len = lambda Mx, n, par: np.linalg.norm((np.linalg.inv(Mx[par]) @ Mx[n])[:3, 3]) if par else abs(Mx[n][1, 3])
    jpar = {}
    for p in out.g["nodes"]:
        for c in p.get("children", []):
            jpar[c] = p
    dnames = [n.get("name", "") for n in dg["nodes"]]
    dtargets = {}
    for i, n in enumerate(dg["nodes"]):
        if "mesh" in n:
            t = dg["meshes"][n["mesh"]].get("extras", {}).get("targetNames")
            if t:
                dtargets[i] = [x.split("\x01")[-1] for x in t]
    for a in dg.get("animations", []):
        if motions and a["name"] not in motions:
            continue
        ch, sm = [], []
        for c in a["channels"]:
            s = a["samplers"][c["sampler"]]
            node = c["target"]["node"]
            path = c["target"]["path"]
            t_in = glb.get_accessor(dg, db, s["input"])
            val = glb.get_accessor(dg, db, s["output"])
            if path == "weights":
                src = dtargets.get(node)
                if not src:
                    continue
                v = val.reshape(len(t_in), len(src))
                for k, names in face_meshes:
                    o = np.zeros((len(t_in), len(names)), np.float32)
                    for j, nm in enumerate(names):
                        if nm in src:
                            o[:, j] = v[:, src.index(nm)]
                    sm.append({"input": out.accessor(t_in, glb.FLOAT, "SCALAR", minmax=False), "output": out.accessor(o.ravel(), glb.FLOAT, "SCALAR"), "interpolation": s.get("interpolation", "LINEAR")})
                    ch.append({"sampler": len(sm) - 1, "target": {"node": k, "path": "weights"}})
                continue
            name = norm(dnames[node])
            if name not in idx:
                continue
            if path == "translation":
                if name == "Bip01Pelvis":
                    f = 1.0
                else:
                    # the game: Bip01 by |rest y|, others by rest bone length; ratio new/donor
                    pn = dpar.get(name)
                    jp = [k for k, v in idx.items() if v == out.g["nodes"].index(jpar[idx[name]])] if idx[name] in jpar else []
                    f = rest_len(M, name, jp[0] if jp and name != "Bip01" else None) / max(rest_len(dB, name, pn if name != "Bip01" else None), 1e-9)
                val = val * f
            sm.append({"input": out.accessor(t_in, glb.FLOAT, "SCALAR"), "output": out.accessor(val, glb.FLOAT, {"rotation": "VEC4", "translation": "VEC3", "scale": "VEC3"}[path]), "interpolation": s.get("interpolation", "LINEAR")})
            ch.append({"sampler": len(sm) - 1, "target": {"node": idx[name], "path": path}})
        for s in sm:  # glTF requires input min/max
            ai = out.g["accessors"][s["input"]]
            t = glb.get_accessor(out.g, bytes(out.bin), s["input"])
            ai["min"], ai["max"] = [float(t.min())], [float(t.max())]
        if ch:
            out.add("animations", {"name": a["name"], "channels": ch, "samplers": sm})


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("src")
    ap.add_argument("out")
    ap.add_argument("--preview", help="HST character .glb whose motions to copy for checking")
    ap.add_argument("--motions", help="comma-separated motion names (default: all)")
    a = ap.parse_args()
    o = rerig(a.src, a.out, a.preview, a.motions.split(",") if a.motions else None)
    e = o.g["extras"]
    print(f"{a.out}: {len(e['mapped'])} core joints mapped, synthesized {e['synthesized']}")


if __name__ == "__main__":
    main()
