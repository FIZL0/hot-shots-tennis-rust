# P17s3 — per-vertex, unnormalised model light (2026-10-08)

**Question.** VU1 lights each vertex with the bone-weighted normal sum as is (two-entry vertices carry
pre-weighted normals, |n_e| = w_e); the port lit per fragment with Bevy's normalised skinned normal. Does the dump
agree for the multi-bone vertices (nodes 2, 54, 55, 58, 59, 3, 35 of pc05)?

**Answer: yes.** `research/p17s3_multibone.py ../s4/context/p17s/s5.gs context/p17s1/pc05_t01_c00.txt 0.60392
0.58745 0.58745` fits every bone's light direction (bone space) at once from pc05's lit vertices; it is linear,
d = Σ n_e·L_bone(e), and then predicts GS = ⌊vc·A·(1 + max(d, 0))⌋ (A = light on court 10).

| normal | \|L\| per bone | single-bone (1457) | two-bone (251) |
|---|---|---|---|
| as is (Σ n_e) | 0.996 … 1.038 | 1409 exact, 48 ±1 | 247 exact, 4 ±1 |
| normalised (bone-space proxy) | 0.835 … 2.424 | 24 off by > 1 (max 174) | 127 off by > 1 (max 294) |

No vertex on the disc has more than two bones (all 10 characters, `skinned()` counts).

**Port.**

- `mdl::SkinVertex::normals`: the bind-space normals of the entries on `joints[0]` and `joints[1]`, pre-weighted and
  not normalised (was one normalised sum). `characters.rs` test: |normals[k]| = weights[k].
- `character.rs` puts the first in the normal attribute and the second in the tangent slot.
- `gs.wgsl` has GsMaterial's own vertex stage: n = J0·n0 + J1·n1 (joint matrices, 3×3, no normalise; rigid meshes
  keep Bevy's normal), VU1's ambient + light × diffuse and the specular term per vertex, vertex × material colour ×
  light truncated to the GS's 8 bits, Gouraud-shaded to the fragment stage (which no longer lights). The court is
  lit per vertex too now, as on VU1.
- Screenshot: `context/shots/p17s3/port.png` (court 10, `HST_AUTOPLAY=1 hst <iso> --stage 10 --play --shot-at 3`).
