# P17b — court fog against the original (confirmation)

## Check: every fogged vertex of a slot-5 GS dump
`research/p17b_fog_gs.py context/p17s7/s5.gs context/p17s7/s5.ram` (court 10, slot 5, 4 frames). It reads each
packed XYZF2 vertex of a PRIM-FGE draw with its F and Q. VU1 sends Q = 1/w, so w = 1/Q. It then recomputes F from the
court view's fog block in RAM (view 0x1e7d10 +0x520: F far 224.4, F near 255, slope −0.204, intercept 263.16):
- Court view, F = ⌊max(min(intercept + w·slope, F near), F far)⌋: **84 972 vertices, all exact.**
- Players: the same far F and depths with their own near F 245 (`exe::Game::player_light`[3], `shade.rs`):
  **60 860 vertices, all exact** (F 243–245 at 30–50 m).
- Far draws (w 600–2200 m): F 255 (the sky's flat F, envir row +0x34 = 255) and F 229 (the bg models' flat
  229.5, row +0x30). The port uses these same flat values (`gs::court_fog`, `weather::apply`).
- The last 272 vertices (F 0, Q 1, black, ALPHA 0x44) go to an offscreen buffer (FBP 0x8c, ~22×14 px, Z mask),
  not the screen. That's the shadow-texture pass that P17v ports. It's not a court-view draw.

## Change: F per vertex, interpolated as the GS does
The port used to work F out per pixel from the pixel's depth. VU1 works it out per vertex (above), and the GS
interpolates F linearly in screen space and then truncates it. `gs.wgsl` now computes F in the vertex shader
(`@interpolate(linear)`, ⌊F⌋ as FTOI) and floors it again per pixel. The two only differ on triangles that cross
the 40–190 m ramp. Large ground and backdrop triangles do, and they now shade like the PS2's.

## Screens
`context/shots/p17b/c10_0.png` is the original (`research/b74b_hunt.py 5 120 c10 1.0 5`, a held frame; the script
now takes the copy's snaps dir from `HST_PCSX2` and the output dir from `HST_SHOTS`).
`port_c10.png` is `--stage 10` from the same `HST_EYE`. They show the same view, and the far wall and temple haze
the same way. The match camera sits about 41 m from the court centre, so the court itself stays at F ≈ 255 in both.

## Not verified / not 1:1
- Pixel values were not diffed. Near the camera fog is F ≥ 250 (≤ 2 %), so the light and shade differences already
  known (P17r/P17v) would swamp it. The check here is per vertex, against the GS dump.
- The GPU clips triangles at the near plane. VU1's own clipping may split triangles differently (B92's open clip
  rule), so F along a clipped edge can differ.
- Gouraud F is interpolated in float, the GS in fixed-point DDA steps, so ±1 F level per pixel.
- Only time of day 0 and court 10 were checked (the only fogged save state with a GS dump). Other courts and weathers
  use the same code with their envir rows (`court_fog_row` covers rain from PINE values).
- The F 0 offscreen pass (FBP 0x8c) is not ported here; it belongs to the shade-map passes (P17v7–P17v9).
