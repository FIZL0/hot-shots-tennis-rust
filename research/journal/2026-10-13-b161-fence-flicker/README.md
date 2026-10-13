# B161 — chain-link fence flicker on North Academy Grounds

## Cause
The fences are fine-grid see-through textures that come with no mip levels:
- The hole's `fense` (`_h01_`, 256×256, alpha only 0/255, MXL 0).
- The tall nets on concrete pillars around the courts (`sclwall-01/02/03`, materials `wallnet`/`wallnet2`, TEST 25, 256×64, 1-texel lines).

The GS samples them at level 0 only. At 640×448 the original's fences blur into a soft haze. At our resolution
(plus 4x MSAA) the 1-texel lines beat against the pixel grid, so you get moiré bands that slide as the camera moves.
It wasn't z-fighting or draw order: the bands moved when the yaw changed by 0.003 rad, with the geometry unchanged.

## Fix
- `main.rs` `cutout_mips` builds a box-filtered mip chain (alpha-weighted colour, averaged alpha) for textures without GS levels.
- It applies only to draws that blend their see-through texels: the hole model and TEST 20..29.
- `GsUniform.screen_lod = 1` makes `gs.wgsl` pick the level from the texel's on-screen size (+1 bias) instead of the GS depth formula.
- Pure cut-outs (TEST 10..19, the trees' leaves) are untouched. An averaged alpha would thin them out.

## Checked
- Orbit view, `--stage 1`, yaw 0.600 vs 0.603:
  - Old build: moiré bands on the sclwall nets that shift between the two yaws.
  - New build: an even net. (`context/b161/orb_*`)
- Match view, `--stage 1 --play`, shots at 9.0/9.1 s:
  - Old: the back fence between the bushes shows a woven moiré pattern.
  - New: a steady, darker haze, close to the original's look (PCSX2 slot 8 at the first serve on court 1, `context/b161/m7.png`, `o_tl.png`). Shots: `context/b161/pan/grid9.png`.
- Courts 2 and 3: same as before apart from the tennis nets (also TEST 25), which come out a touch smoother (`context/b161/pan/cmp_s2.png`, `cmp_s3.png`).

## Not verified / not 1:1
- Not a pixel match with the original. The original renders at 448 lines, so its fences are ~1.5–2 levels blurrier still. We keep the remaster's sharper look on purpose (rendering only has to look right).
- The flicker was judged from still shots a camera step apart, not a video. `--shot` timing isn't frame-locked, so frame-diff numbers were noisy (a whole-frame brightness change, and a focus-loss pause in some shots). Judged by eye instead.
