# B54 — the "teeth" wedge was a modelling helper triangle (2026-10-13)

## What the user saw
`context/b54/user-teeth-{1,2}.png` (same image): Jun (character 2) in a close-up; a grey band (~115 grey, the real
teeth) with a bright yellow-green pointed wedge (203,245,153) poking out between the lips.

## Cause
Every body model (`PC/PCnnC00.XB`, all 14) carries one extra untextured material with a single 3-vertex triangle,
rigidly bound (weight 1) to the node **"sankaku"** (三角, child of `Bip01Head`, ~14 cm forward / 4 cm down of the head
origin = at the mouth). The triangle lies flat in the node's xz plane (±1.5 cm, normal −y), pointing forward: a
3ds Max direction helper the exporter left in. Its material is named "iki" (息) on most characters (Jun's colour
(1.52, 1.88, 1.16) = the green), "Standardmaterial"/"NN - Default" on others (grey; likely R1's "white sliver between
closed lips" on Ashley). No ANI2 motion animates "sankaku"; the strings "sankaku"/"iki" aren't in the program or
overlays. The real teeth ("HA" / "S_HA", 24 verts, two-sided) are separate and fine.

## Fix
`mdl::Model::is_helper` (a packet whose vertices are all bound only to "sankaku"); `Model::skinned` gives that packet no
triangles, so `character::skinned_parts`, `noise` (same "drawn" filter) and `hst-gltf` all drop it consistently.
Test: `hst-data/tests/characters.rs mouth_helper_triangle_is_not_drawn` (each of the 14 bodies: exactly one helper,
3 verts, no triangles).

## Checked
Port close-ups (`HST_EYE` wrapper, `--character 2`), `context/b54/run5/grid.png`: before = the green tip between the
lips (exactly the user's defect), after = gone. `context/b54/run6/mouths.png` (motions 16, 44): gritted and open
mouths show the teeth light grey (~117) inside the lips, nothing poking out, no green.

## Not verified / not 1:1
- **Whether/how the original skips the helper**: not found. No MTL header field marks these materials (mode 0x1e = 5
  like real ones) and the program never names "sankaku". RAM pokes of the helper's VIF copies (×40 scale) changed
  nothing, but positive controls (teeth/shoe vertex pokes) didn't either, so those copies aren't what's drawn and
  the test is inconclusive. Hidden because it's no part of the character and the user named it as the defect.
- **Side by side with the original in the same close-up**: no original face close-up obtained (match cameras too far
  at native res; Items screen preview `context/b40b4/orig/c02_k0.png` too small; camera/view pokes during F8 had no
  effect). Teeth colour judged by eye only.
- **Teeth read light grey (~117/255), not bright white**: that's the faithful untextured path (GS draws VU1's 8-bit
  colour as is, lit HA material); not changed without an original close-up to compare against.
