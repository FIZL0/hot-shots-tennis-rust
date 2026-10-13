# B31e — sun glare

## What the original does (the post-effect's draw, the branch B31d left out)
- Condition: dot(view forward, light direction) < −0.8 and the clear/cloudy flag (set at court setup to weather < 2;
  the lens flare uses the same flag). Read live from slot 5: the two vectors the draw dots are the **camera's forward
  row** and the **light direction (sun → ground)**, not "sun" and "view" as B31d guessed. So "sun height" in the
  formula is really how far the *camera looks up*: −forward.y (game y down) clamped to the data pair 0.0..0.8.
- FIX = (int)(((cos − 0.8)/0.2·0.5 + seen·0.2) · up/0.8 · 128/0.7), seen = the flare's sun read-back (sqrt sqrt of the
  weighted visible pixels/640, capped at the data value 1.0), not rain.
- Draw: TEX0 = the frame being drawn (feedback), bilinear (TEX1 0x61), ALPHA (Cs−Cd)·FIX+Cd, 20 strips 32 px wide
  covering 640×224, twice, uv offset by (−8,−8) then (+8,+8) sixteenths from texel centres. Each pass mixes the
  2×2 average next to every pixel into it. No 25 % persistence that frame.
- Slot 5's match camera looks 17° down (forward.y 0.29), so the glare never fires in normal match play there; it
  needs a camera looking up toward a low sun (cut-aways, replays, other courts).

## Port
- `hud_gamma.rs` `glare()`: the weight from the match camera (`Orbit`) and `shadow::Sun::light`, weather < 2; `seen`
  = 1 while the sun is in the picture (as `play/flare.rs`'s fade). FIX truncated to its byte as the game does.
  `persist` sends it as `weight.y` (persistence weight 0 then) with the game's field texel in uv (640 columns per 4:3
  width, 224 lines) as `weight.zw`.
- `persist.wgsl`: with `weight.y` > 0, both passes in one fragment: pass 1 at the 4 points pass 2 needs (9 bilinear
  samples), pass 2 mixes their average. The scene image now also has a sampler (binding 3).
- Test `hud_gamma::tests::glare_strength` (looking at a 40° sun: 102/128; level or down: off).

## Checked by eye
- Original: `research/b31e_glare_poke.py` (slot 5, light poked to face the camera, the look-up clamp floor poked
  to 0.8 → FIX 91): `context/b31e/lk_on.png` vs `lk_off.png`, `pcsx2_on/off.png`: the court and net mesh go soft,
  a slight blur (crop `lk_crop.png`, `pcsx2_crop.png`).
- Port: `HST_EYE` from `context/b31e/eye_on` (court 10, looking at slot 5's sun) → `port_on.png`: the temple goes soft
  the same way, HUD sharp; `eye_off` (60° away) → `port_off.png` sharp.

## Not verified / not 1:1
- No same-frame side-by-side: the original had to be forced (light and clamp poked, which also darkens its lighting),
  the pause didn't hold between pokes so the shots are frames apart, and its match camera never looks up at the sun
  in the recorded slots. Compared as "a slight blur of the same size" only (render-only, PLAN).
- `seen` (the flare's occlusion read-back) is 1 while the sun is in the picture: a player or the stands in front of
  the sun don't weaken the glare (same ponytail as the flare). The original also keeps the last read-back while the
  sun is off-picture sideways; the port uses 0 then.
- The blur reads the port's scene at its own resolution with the original's texel size; the original's field
  half-line shift is not reproduced (progressive frame, as B31d).
- Draw order: the original blurs whatever was drawn before the post-effect (incl. the speed readout and balloon,
  B31d's 2045/2046); the port blurs the whole 3D scene image (incl. the lens flare meshes) and none of the UI.
