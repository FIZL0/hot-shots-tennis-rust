# M1d mod mesh lighting (2026-10-08)

Mod costumes now light, test and sway like the disc's. A rerigged disc pc00 (`context/mods/test_pc00`, made
from `hst-gltf hst` + `rerig.py`) matches disc pc00 in `--shot` pairs to ≤ 1 grey level away from silhouette
edges. Detail: `1-lighting-FINAL.md`.

- Normals: VU1 gives each vertex n0 = w0·N (bone 0) and n1 = (1−w0)·N (bone 1). On every disc character
  (pc00–09, all costumes) n0 ∥ n1, |n0| = w0, |n0 + n1| = 1 and there are at most 2 influences. So a mod's
  `NORMAL = w0·N`, `TANGENT = (1−w0)·N` is exact for disc meshes. gs.wgsl now turns the tangent slot by the
  weight blend of bones 1..3: that is still bone 1 on the disc, and exactly Σ wᵢRᵢN for 3–4 influences.
- MTL words: `hst-gltf` writes `material.extras.hst_mtl` {shininess, highlight} (header +0x10/+0x14),
  `alphaMode` MASK/BLEND for TEST modes 10–19/20–29, and `doubleSided` for two-sided. mods.rs turns them back into
  header words, TEST 10/25, `two_sided` and PRIM (fog always on, as on every disc character batch).
- Sway: `hst-gltf` writes the `.NOI` deformers to `mesh.extras.noise` and `[deformer, share]` per vertex to
  `_HST_NOISE`; rerig.py keeps mesh extras. mods.rs builds a `noise::Costume` (entries p = w·invbind·pos).
- `tools/shot.sh`: `--shot` in a fixed 1280×720 floating window on a hidden Hyprland workspace (plan/REFERENCE.md).
- Test: `mods::tests::rerigged_mod_plays_forehand_and_run` also checks the deformer and swaying-vertex counts and
  the set of GS draws (key, shininess, highlight) against disc pc00.

## Not verified / not 1:1
- Mods exist only in the remaster, so the comparison is mod vs disc character in the remaster's renderer, not
  against PCSX2. The disc path itself is matched against PCSX2 elsewhere.
- Noise entry positions are recomputed in f32 from the inverse binds (~1e-7 m off the disc's stored entries).
- Mods with 3–4 influences, or with normals not parallel per bone, get Σ wᵢRᵢN. The disc never has such
  vertices, so there is no original to match.
- Fog is always on for mod materials (true of every disc character batch; a mod can't turn it off).
- A mod material without `hst_mtl` gets shininess/highlight 0 (matte), the standard's default.
- The mod side has 4 more GS draw entries than the disc (same distinct draws). Not traced; no visible effect.
- Mods already packaged in HST-MODS `out/` were not re-exported, so they have no highlights or sway until
  rebuilt with the new `hst-gltf` + `rerig.py`.
- Pixels within ~2 px of silhouette edges differ by up to 65 grey levels: the rerigged mesh's transforms differ
  sub-pixel. This is geometry, not lighting.
