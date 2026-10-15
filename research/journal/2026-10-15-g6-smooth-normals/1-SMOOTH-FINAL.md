# G6: smoothed normals (Settings → Graphics → Smooth Normals)

Split from G1. VU1 lights per vertex (`gs.wgsl`). The disc splits a vertex wherever packets meet or a curve was
exported faceted, so the same position carries a normal per face.

## How faceted the disc is (scratch scan of every MDL, not kept)
- Court props (COURT/01): 7 % of vertices sit at a position with a soft split (< 70°), 21 % with a hard one.
  Over all courts the split angles spread evenly over 0–70° with a big peak at 90° (box corners).
- Rackets: 13–21 % of vertices split, all at 10–40° (the round frame).
- Characters (skinned, all PC*.XB): about 1 % soft splits, mostly seams. They are already smooth.

## What it does
- `Graphics::smooth` (`--smooth-normals`, settings.txt `smooth_normals = on|off`). It is a row on Settings →
  Graphics after Bloom. Presets set it (off in all of them). `graphics::SMOOTH` (kept in step by `mesh_detail`) is
  forced off in Original Mode.
- `smooth.rs`: within one mesh, each vertex takes the normalised sum of the distinct unit normals at its position
  (1e-4 grid) that lie within 45.6° (cos 0.70) of its own. A seam's repeated normals count once. Vertex count,
  indices and the other attributes are unchanged.
- Skinned meshes: the normal slot (first bone's share) and the tangent slot (the rest) are summed into the bind
  normal and averaged. Each slot keeps its own length on the new direction, so gs.wgsl's pre-weighted sum still
  works. This is the same for mod characters' w0·N / (1 − w0)·N.
- It runs at load time, before `lod::detail`, in `main.rs` `mesh()` (court props, also feeding `merge.rs`),
  `character.rs` (skinned parts at the Mesh setting and the racket) and `mods.rs` (mod body and racket). Meshes
  are RENDER_WORLD only, so a change applies from the next load (like Mesh).
- Crease: 63° (cos 0.45) also smoothed roof ridges and bevels. On Stage 3 that changed 31k px across whole flat
  roof and wall planes (Gouraud tinted their big triangles). At 45.6° only rounded props change: Stage 3 4.0k px,
  Stage 4 2.7k, Stage 10 8.8k (stone curbs and ruins arcs).

## Checks
- Unit tests in `smooth.rs`: soft splits close, a 90° crease and lone vertices stay the same, and skinned
  per-slot lengths are kept. `Graphics` text/flag round trip covers `smooth`. `tools/check.sh -p hst` passes.
- `tools/shot.sh` static `--stage N` off vs on (context/g6/run2, run3). Static views are deterministic (Stage 3
  off vs off: 0 px), so every difference comes from the smoothing.
- Original Mode + `--smooth-normals` is pixel-identical to Original Mode (context/g6/run5).
- Pitfall: side-by-side composites viewed as images looked darker on one side even though the crop means were
  equal. Judge by diff maps and numbers.

## Not verified / not 1:1
- By design not in the original: nothing to match. Judged by diff maps on Stages 1, 3, 4, 10 only.
- Characters and rackets: the `--character` viewer and `--play` animate, and two "off" shots already differ as
  much as off vs on (context/g6/run1, run4). The change on them was not isolated in an image. It is covered by
  the unit tests and the scan above (characters have ~1 % split vertices).
- The 45.6° crease is a compromise: a deliberate bevel under 46° gets smoothed, and a faceted prop with fewer
  than 8 sides stays faceted.
- Settings page with the new row not screenshotted (rows are sized from the row count, as for G3's Bloom).
- Browser builds not run (no browser test here). The setting has no wasm-specific path.
