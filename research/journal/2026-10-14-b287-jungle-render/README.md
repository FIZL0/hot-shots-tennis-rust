# B287 — Jungle court rendering (court 8)

User report (2026-10-14): far-side shadows vanish, extra grass clips through the ground, the sky cuts off.

## Method
- Original: scratch slot 8 = court 8 doubles at the first serve (near end), slot 9 = the same match after the change of
  ends (far end), via `research/b287_shots.py`. Screenshots plus the view (eye/forward/down/tan) read from the camera
  in RAM, also 6 low cut-away views from slot 9. Images in `context/b287/run1..run4/`.
- Remaster: `tools/shot.sh` with `HST_EYE` set to the same views and `HST_WEATHER=0` (clear, as the original's
  match). **Compare at 16:9** (`HST_SIZE="1348 758"`, no `--4x3`): the original's screenshots are a true 16:9 view,
  and 4:3 shots don't line up.
- The original's sun moves with the match's hour, so a later game's shadows fall differently from hour 1's.

## Findings
1. **Shadows** — the far half's canopy shadows faded out in Bevy's later cascades (first cascade 30 m of 120 m, 4
   cascades). With the first cascade at 80 m the whole court is in it from either baseline camera and the far
   shadows match the original (checked from both ends). `graphics.rs lights()`. The WebGL2 path (1 cascade, 60 m) is
   unchanged.
2. **Sky cut off / flat-topped wall** — the hole model's midground strip (`tyuukei01/02`, a 12-sided ring of radius
   ~105 m, 54 m tall, two-sided) uses a 256×320 texture with v over 0..0.62. The GS samples over the power-of-two
   height (TH = 9 → 512 rows; 320/512 = 0.625), but we uploaded 320 rows, so the texture was stretched 1.6× and its
   cut-out tree tops fell past the top of the strip. That left a flat-topped vine wall against the sky. Textures that
   aren't a power-of-two size are now padded to it (last row/column repeated) before upload: `main.rs pow2()`, used
   by `gs_models_anim`. The low cut-aways now match the original: tree silhouettes against the sky gradient, the hut
   and the bushes. Other textures this changes: court 3's backdrop (128×192, v ≤ 0.7496 = 192/256, was stretched the
   same way), court 9's `teppan` sheet (100×100, v ≤ 0.78125 = 100/128), court 11's `hashi01` planks (64×63,
   tiled, now 64 rows with the last one repeated).
3. **Grass** — with the strip fixed, the ground beside the court, the net-post tufts and the low cut-aways match the
   original from both ends. Before the fix, the strip's dense ground-level hedge band was stretched across its lower
   part (from 14 m below the ground upwards), which is the likeliest source of the grass seen through the ground. No other layer
   overlaps: the hole's opaque layers don't z-fight (grid scan of its triangles), `grass_nuke`/`ingrass_nuke` are
   2.5–9 mm fade rings above the dirt, and hiding them changed almost nothing. Several flower/tuft props
   (`c8hana-red` etc., e.g. at the net posts) sit 0.2–0.8 m below the ground. That comes straight from the disc's plant
   records, so the original places them the same way.
- The static shadow casters match the original's list in RAM (38/38).

## Not verified / not 1:1
- Shadow shapes on the far half compared by eye only, and at a different match hour from the original's far-end shot
  (the original's sun had moved by game 2). Rendering only has to look right.
- Cascades: Bevy's cascade split still differs from the original's single shadow map. Past 80 m (cut-aways
  looking out), shadows come from the later cascades, as before.
- The "grass clips through the ground" report wasn't reproduced as something separate from the stretched strip. It is
  matched by eye from the match cameras and 3 low cut-aways, not from every possible camera.
- The GS reads whatever VRAM holds past a texture's stored rows. The padding repeats the edge texels instead, which
  only matters if UVs reach past the stored rows (none of the four affected models' UVs do).
- Court 3, 9 and 11's padded textures weren't compared with the original: court 3's backdrop isn't in view from the
  default match camera. They follow the same GS rule (UVs normalised to the power-of-two size).
- Overall ground colour is slightly greener/darker than the original's (colour, not geometry; not in this task).
