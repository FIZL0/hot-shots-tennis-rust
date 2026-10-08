# 1. Lighting pair (FINAL)

`HST_ISO=context/m1d/iso/hst.iso` (a symlink in a folder without texture replacements; keep it unresolved,
`realpath -s`), viewer `--radius 3`, `tools/shot.sh` 2048×1152, grey diff, then a 5 px min-filter
(erosion) to drop edge lines:

| pose | max | px > 2 | px > 8 | eroded max |
|---|---|---|---|---|
| stand, motion 0, 0.5 s | 65 | 2433 | 802 | 1 |
| stand, motion 0, 3 s (hair swayed) | 69 | 3297 | 691 | 1 |
| forehand, motion 16, 0.6 s | 55 | 1253 | 272 | 1 |

Before: the mod had no highlights on the shoes, skirt edge or hair (MTL words 0, normals only from bone 0).

Pitfalls:
- Tiled windows change size as other agents open windows: a pair at 1890 vs 938 px wide is useless. Fixed by
  floating windows.
- `realpath` on the ISO symlink brought back the replacements beside the real ISO. The disc side then had
  upscaled, sharper textures and showed a ~28k px "difference" in the textures, not the lighting.
- `--shot` timing is not frame-exact, so two shots of the same run differ a little at the edges.
