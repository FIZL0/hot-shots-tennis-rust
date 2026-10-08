"""Writes modding/hst_skeleton.json: the HST player skeleton every mod character must carry.

python3 modding/tools/make_skeleton.py   DIR   (DIR: pcNN_c00.glb from `cargo run -p hst-gltf -- hst context/xb DIR`)

Canonical frame data is pc00's (Ashley); every HST character shares names, parents and bone axes (bind orientations
within ~10°), only proportions differ, so `characters` lists each one's measures as the allowed range.
"""
import glob
import json
import os
import sys

import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
sys.path.insert(0, HERE)
sys.path.insert(0, HERE)
import glb  # noqa: E402
from rerig import binds, mat_quat  # noqa: E402

NAMES = ["Ashley", "Cody", "Jun", "Kaito", "Momoko", "JJ", "Carol", "Big Chief", "Miranda", "Kent", "Lola", "Will",
         "Gloria", "Suzuki"]
FACE = ["blink_eye", "joy_eye", "joy_mouth", "anger_eye", "anger_mouth", "sorrow_eye", "sorrow_mouth", "doki_eye",
        "doki_mouth"]


def main():
    chars = []
    ref = None
    for p in sorted(glob.glob(os.path.join(sys.argv[1], "pc*_c00.glb"))):
        g, b = glb.read(p)
        B, parent = binds(g, b)
        core = [n for n in B if n.startswith("Bip01")] + ["Racket"]
        if ref is None:
            ref = (core, B, parent)
        targets = sorted({t.split("\x01")[-1] for m in g["meshes"] for t in m.get("extras", {}).get("targetNames", [])})
        n = int(os.path.basename(p)[2:4])
        chars.append({"index": n, "name": NAMES[n], "bip01_height": round(-B["Bip01"][1, 3], 4),
                      "head_height": round(-B["Bip01Head"][1, 3], 4), "face_targets": targets})
    core, B, parent = ref
    joints = []
    for n in core:
        P = parent[n]
        local = np.linalg.inv(B[P]) @ B[n] if P in core else B[n]
        joints.append({"name": n, "parent": P if P in core else None,
                       "bind_position": [round(float(x), 5) for x in B[n][:3, 3]],
                       "bind_rotation": [round(float(x), 6) for x in mat_quat(B[n][:3, :3])],
                       "rest_translation": [round(float(x), 5) for x in local[:3, 3]],
                       "rest_rotation": [round(float(x), 6) for x in mat_quat(local[:3, :3])]})
    out = {"about": "HST player skeleton (game space: metres, Y down, feet at y=0, facing +Z). Canonical frames from "
                    "pc00; see modding/HST-CHARACTER-STANDARD.md.",
           "joints": joints, "face_channels": FACE, "characters": chars}
    path = os.path.join(ROOT, "hst_skeleton.json")
    json.dump(out, open(path, "w"), indent=1)
    print(path, len(joints), "joints,", len(chars), "characters")


if __name__ == "__main__":
    main()
