# B31d — the draws after the tornado

## What they are (GS dump `context/b31b/on.gs`, last frame, `research/tools/gsdump.py`)
Identified from the draws' screen boxes, vertex colours and strip counts, compared with `on.png`:

| draw | TBP | what |
|---|---|---|
| 2035–2039 | 0x2f80… | ball tornado (`@add`, ALPHA 0x48) |
| 2040 | 0x23c0 | the ball's flight ribbon (`ballrolling`, purple: the shot kind's colour), from the ball back to the hit |
| 2041 | 0x23e0 | the hit sparks and impact burst round the racket |
| 2042 | 0x2400 | the hitter's swing trail (`zanzou`, first vertex alpha 0 = the faded tail) |
| 2043/2044 | 0x2440/0x24a0 | small effects near the far player: a bounce's puff and mark (outside the tornado) |
| 2045 | 0x24c0 | the reaction balloon (♪) |
| 2046 | 0x24e0 | the "71 mph" speed readout |
| 2047 | 0x8c0 | full-screen frame persistence (below) |

All of 2040–2046 are normal blends (ALPHA 0x44), FGE, z-test GEQUAL.

## Draw 2047: frame persistence
- Drawn by the post-effect object at slot 0x20 of the game's post-effect set (vtable draw at +0x18; it is enabled when
  the match scene is built and kept on while a match runs: its enable byte is 1 in RAM dumps s03/s04/s05/s08).
- Texture = the *other* frame buffer, picked by frame parity XOR 1. That is the frame shown last (0x8c0 = page 70 = the
  second 640×224 buffer).
- 20 sprites, each 32 px wide, cover the frame. The v coordinates are shifted ±0.5 line by the field parity, so the
  other field's frame lines up. ALPHA (Cs−Cd)·FIX + Cd with FIX 0x20, so each frame keeps 25 % of the last shown one.
  The result is an exponential ghost trail. It changes nothing in a still scene.
- **Instead** of this, while the camera looks within ~37° of the sun (−dot(view, sun) > 0.8) and a flag is set
  (1 in s05), it draws a sun glare. That is two passes of the frame at (−½,−½) and (+½,+½) texel offsets with
  FIX = ((−dot−0.8)/0.2·0.5 + rain·0.2) · clamp(sun height, 0, 0.8)/0.8 · 128/0.7, which works out to a half-pixel
  blur of up to 71 %.
- So the "≈0.85×" in B31b: in a still scene the 25 % mix changes nothing, so the dimming comes from the effects
  drawn over the tornado (ribbon, sparks, trail with normal blend) and from slow-mo frozen frames mixing in a frame
  where the tornado differs.

## Port
- `hud_gamma.rs`: `Persist` (a resource the match inserts, `play/tornado.rs` plugin; mode teardown drops it). In a
  match, two HDR Camera2d passes (order 48/49, one per frame parity) draw `persist.wgsl`, which mixes the scene with
  the image the composite showed last frame, into two `Rgba8Unorm` images that take turns. The composite shows this
  frame's image. The mix happens on encoded values, as on the GS. The weight is 0.25 per 60 Hz frame, scaled to the
  real frame time (0.25^(dt·60)), so the trail fades as fast at 144 Hz. Menus show the scene unmixed.
- `effects.rs`: the effect materials (`model`, `look`: impact, ribbon, sparks, trails, puffs, marks, foot dust,
  clap) sort at `depth_bias` `AFTER_SCENE` = 500. Their meshes sit at the court's origin, so before this they sorted
  before or after the tornado depending on where the ball was. Now they always come after the tornado and the
  court's blended draws, as in the original. Landing markers stay after them (1e3+).

Checked: `hst <iso> --stage 1 --play --shot … --shot-at 6` (debug build) renders the match through the persistence pass, no panic, normal brightness (`context/b31d/play1.png`).

## Not verified / not 1:1
- Draws 2040–2046 were identified by position, colour and strip count only. I didn't trace their texture uploads to
  disc files: the dump's VRAM uploads have no parser yet, and rendering is visual-only (PLAN).
- I didn't take a side-by-side screenshot of the persistence against the original in motion. Its look is a 25 %
  one-frame ghost trail, which a still `--shot` can't show, and PCSX2 shots of a moving ball can't be lined up with
  ours frame for frame.
- The half-line field shift isn't needed: the port renders progressive frames.
- At other frame rates the weight is scaled per frame time, while the original works per 60 Hz frame. At 60 Hz
  they are the same.
- The sun glare branch (the half-pixel blur used instead of the persistence when facing the sun) is not ported:
  B31e. Until then the port keeps the 25 % mix there.
- The 8-bit integer blend truncation is not reproduced (float mix stored to 8 bits; B31c covers GS truncation).
