# B100 — web build: shadows missing on the court

## Cause
WebGL2 only. Bevy 0.19 caps a directional light at **one cascade** under WebGL2 (`MAX_CASCADES_PER_LIGHT = 1`,
warning "The number of cascades configured for a directional light exceeds the supported limit of 1"). `graphics::lights`
configured 2 (Low) or 4 (High) cascades with `first_cascade_far_bound = 30`, so WebGL2 kept only the 0–30 m slice. The
match camera sits farther from the court than that, so `fetch_directional_shadow` found no cascade for the court and
returned "lit". The phone Low preset isn't the cause: Low and High both lost the shadows. WebGPU was never affected,
because it gets all the cascades, the same as native.

## Fix
`graphics.rs` `lights`: on WebGL2 builds (`wasm32` without the `webgpu` feature), one cascade out to 60 m with a 2048
map for both qualities. Native and WebGPU are unchanged.

## Verified (headless Chromium on the real GPU, `--use-angle=vulkan`; captures in `context/b100/`)
- run2: WebGL2 before the fix, `?webgl2&play&stage=10&shadows=high`: no shadows on the court (reproduced), and the cascade warning.
- run3: WebGL2 after the fix, same URL: the players', umpire chair's and props' shadows are on the court, and the warning is gone.
- run5: WebGL2 after the fix with the phone preset (`shadows=low&scale=67`): the same shadows.
- run4: the WebGPU build at the same settings: the shadows look the same as the WebGL2 fix.
- `native.png`: native `--play --stage 10 --shadows high`: the same shadows.

## Not verified / not 1:1
- WebGL2 shadows are one 2048 cascade over 60 m, not native's 2 or 4. Edges are a little softer close up. This is
  WebGL2's limit, and rendering only has to look right.
- Cut-aways past 60 m from the camera get no shadows on WebGL2. None were seen; raise `far` if one shows it.
- The ball's shadow is its own model (`ballshadow.mdl`), not the shadow map. The fix doesn't touch it, and no
  capture shows a ball in flight.
- Only Chromium on Linux was tested (SwiftShader doesn't draw the court at all, so the real GPU was used). Firefox,
  Safari and phones are untested.
