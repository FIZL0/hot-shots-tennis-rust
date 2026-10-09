"""Retarget source motions onto a rerigged mod's skeleton as its own `motions.glb` (standard §6 `motions`).

python3 tools/rerig/motions.py COSTUME.glb SRC.glb OUT.glb HST_NAME=SRC_ANIM[+LOOP_ANIM][@FRAMES] …

Per joint the source's world rotation change from its bind pose is applied to the mod joint's world rest rotation
(both rigs share the bind pose: rerig.py places the mod's joints at the source's own), then turned back into a local
rotation under the mod's parent joint. Joints with no source (Spine2, the finger segments, the racket) keep their rest
local. The root (`Bip01`) follows the source pelvis's offset from its bind spot, scaled by the hips' height ratio; the
source's own locators (`ROOTNODE`, `pc_locator`) are left out, since the game moves the player itself. `@FRAMES` is the
HST clip's length at 60 Hz: the source plays at its own speed (60 Hz), the loop repeating to fill it, and only a
source longer than that is squeezed. HST then spreads the keys over the donor clip's length (mods.rs `own_motions`).
"""
import sys
import os

import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(os.path.dirname(HERE), "psp2gltf"))
sys.path.insert(0, HERE)
import glb  # noqa: E402
from rerig import node_mat, norm, quat_mat, mat_quat, read  # noqa: E402

FPS = 60.0
SKIP = {"ROOTNODE", "pc_locator"}


def parents(g):
    return {c: i for i, n in enumerate(g["nodes"]) for c in n.get("children", [])}


def sampler(g, b, anim):
    """node index -> {path: (times, values)} of one animation."""
    out = {}
    for c in anim["channels"]:
        s = anim["samplers"][c["sampler"]]
        t = read(g, b, s["input"]).ravel()
        v = read(g, b, s["output"]).reshape(len(t), -1)
        out.setdefault(c["target"]["node"], {})[c["target"]["path"]] = (t, v)
    return out


def at(tv, t):
    times, vals = tv
    if t <= times[0]:
        return vals[0]
    if t >= times[-1]:
        return vals[-1]
    i = np.searchsorted(times, t) - 1
    u = (t - times[i]) / (times[i + 1] - times[i])
    a, b = vals[i], vals[i + 1]
    if len(a) == 4 and np.dot(a, b) < 0:
        b = -b
    v = a * (1 - u) + b * u
    return v / np.linalg.norm(v) if len(a) == 4 else v


def source_worlds(g, b, anim, t):
    """Node index -> world matrix (glTF world) of the source at time t, its locators held at rest."""
    ch = sampler(g, b, anim) if anim else {}
    par = parents(g)
    W = {}

    def world(i):
        if i not in W:
            n = dict(g["nodes"][i])
            if i in ch and norm(n.get("name", "")) not in SKIP:
                for path, tv in ch[i].items():
                    n[path] = list(at(tv, t))
            W[i] = (world(par[i]) if i in par else np.eye(4)) @ node_mat(n)
        return W[i]

    for i in range(len(g["nodes"])):
        world(i)
    return W


def length(g, b, anim):
    return max(float(read(g, b, s["input"]).max()) for s in anim["samplers"])


def make(costume, src, out, clips):
    """clips: [(hst_name, [source anims: intro, then an optional loop], frames or None)]."""
    cg, cb = glb.read(costume)
    sg, sb = glb.read(src)
    anims = {a["name"]: a for a in sg["animations"]}
    sidx = {norm(n.get("name", "")): i for i, n in enumerate(sg["nodes"])}
    cpar = parents(cg)
    joints = cg["skins"][0]["joints"]
    joints_set = set(joints)
    cnames = {i: norm(cg["nodes"][i].get("name", "")) for i in range(len(cg["nodes"]))}
    # the costume's rest worlds (glTF world)
    W0, _ = __import__("rerig").worlds(cg)
    S0 = source_worlds(sg, sb, None, 0.0)
    root = next(j for j in joints if cnames[j] == "Bip01")
    hip = sidx["Bip01Pelvis"]
    scale = W0[root][1, 3] / S0[hip][1, 3]

    o = glb.Builder()
    # the costume's node tree, no meshes or skins (channels bind by joint name)
    for n in cg["nodes"]:
        o.node(**{k: n[k] for k in ("name", "children", "translation", "rotation", "scale", "matrix") if k in n})
    o.g["scenes"] = [dict(cg["scenes"][cg.get("scene", 0)])]

    for name, seq, frames in clips:
        lens = [length(sg, sb, anims[s]) for s in seq]
        n = frames if frames else int(round(sum(lens) * FPS))
        # frame f of the HST clip -> (source anim, time): the intro, then the loop over and over
        squeeze = lens[0] * FPS / n if lens[0] * FPS > n else 1.0
        plan = []
        for f in range(n + 1):
            t = f * squeeze / FPS
            if t <= lens[0] or len(seq) == 1:
                plan.append((seq[0], min(t, lens[0])))
            else:
                plan.append((seq[1], (t - lens[0]) % lens[1]))
        rot = {j: [] for j in joints}
        pos = []
        for a, t in plan:
            S = source_worlds(sg, sb, anims[a], t)
            Wa = {}

            def wa(j):
                if j in Wa:
                    return Wa[j]
                p = cpar.get(j)
                pw = wa(p) if p is not None and p in joints_set else W0[p] if p is not None else np.eye(4)
                s = sidx.get(cnames[j])
                if s is not None:
                    d = S[s][:3, :3] @ S0[s][:3, :3].T
                    m = np.eye(4)
                    m[:3, :3] = d @ W0[j][:3, :3]
                    m[:3, 3] = (pw @ node_mat(cg["nodes"][j]))[:3, 3]
                else:
                    m = pw @ node_mat(cg["nodes"][j])
                if j == root:
                    m[:3, 3] = W0[j][:3, 3] + scale * (S[hip][:3, 3] - S0[hip][:3, 3])
                Wa[j] = m
                return m

            for j in joints:
                p = cpar.get(j)
                pw = wa(p) if p in joints_set else W0[p] if p is not None else np.eye(4)
                L = np.linalg.inv(pw) @ wa(j)
                rot[j].append(mat_quat(L[:3, :3] / np.linalg.norm(L[:3, :3], axis=0)))
                if j == root:
                    pos.append(L[:3, 3])
        times = np.arange(n + 1, dtype=np.float32) / FPS
        ti = o.accessor(times, glb.FLOAT, "SCALAR", minmax=True)
        ch, sm = [], []
        for j in joints:
            if sidx.get(cnames[j]) is None and j != root:
                continue
            q = np.array(rot[j], np.float32)
            for k in range(1, len(q)):  # keep the quaternions on one hemisphere
                if np.dot(q[k], q[k - 1]) < 0:
                    q[k] = -q[k]
            sm.append({"input": ti, "output": o.accessor(q, glb.FLOAT, "VEC4"), "interpolation": "LINEAR"})
            ch.append({"sampler": len(sm) - 1, "target": {"node": j, "path": "rotation"}})
        sm.append({"input": ti, "output": o.accessor(np.array(pos, np.float32), glb.FLOAT, "VEC3"), "interpolation": "LINEAR"})
        ch.append({"sampler": len(sm) - 1, "target": {"node": root, "path": "translation"}})
        o.add("animations", {"name": name, "channels": ch, "samplers": sm})
    o.write(out)


def main():
    costume, src, out, *specs = sys.argv[1:]
    clips = []
    for s in specs:
        name, rest = s.split("=")
        rest, _, fr = rest.partition("@")
        clips.append((name, rest.split("+"), int(fr) if fr else None))
    make(costume, src, out, clips)


if __name__ == "__main__":
    main()


def ani_frames(path):
    """Length in 60 Hz frames of an HST `.ANI2` (last key tick / ticks per frame), as `hst_sim::pose::Clip`."""
    import struct
    d = open(path, "rb").read()
    al = lambda x: (x + 15) & ~15  # noqa: E731
    tpf, count = struct.unpack_from("<ii", d, 0)
    p, end, todo = 16, 0, [count]
    while todo:
        if not todo[-1]:
            todo.pop()
            continue
        todo[-1] -= 1
        p = al(p + 16 + struct.unpack_from("<i", d, p)[0])
        for _ in range(2):
            n = struct.unpack_from("<i", d, p)[0]
            p = al(p + 4)
            ticks = struct.unpack_from(f"<{n}i", d, p)
            p = al(p + 4 * n) + 16 * n
            end = max([end, *ticks])
        todo.append(struct.unpack_from("<i", d, p)[0])
        p = al(p + 4)
    return round(end / tpf)
