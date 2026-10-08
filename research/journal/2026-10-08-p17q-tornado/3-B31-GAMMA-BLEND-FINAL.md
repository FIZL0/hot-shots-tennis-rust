# B31 — 3D scene blended on gamma values (FINAL)

## What changed
- The 3D cameras render HDR (`Rgba16Float`, nothing encoded, Tonemapping::None skips the tonemap node) into the
  `scene` image, now raw `Rgba8Unorm`; every 3D shader outputs encoded colour, so all blends (translucent, `@add`,
  `@sub`, Bevy `AlphaMode::Add`) happen on the stored values as on the GS.
  - `gs.wgsl`: returns its PS2-unit colour as it is (no `srgb_to_linear`).
  - `StandardMaterial` (characters, NPCs, effects, tornado, markers, balloons): `hud_gamma.rs::encode_pbr_output`
    patches the loaded `bevy_pbr/render/pbr_functions.wgsl` asset once (on `AssetEvent::Added`): encode right
    before `PREMULTIPLY_ALPHA` at the end of `main_pass_post_lighting_processing`. Asserts the anchor is found once
    (Bevy 0.19.1).
  - Clear colour: scene cameras get `ClearColorConfig::Custom` = `ClearColor`'s sRGB components taken as linear.
  - Composite (`hud_gamma.wgsl`): the scene is already encoded (`b`, not `enc(b)`).
  - Tornado: back to sRGB-decoded texels (the raw-texel workaround from P17q would now be double-encoded); its MTL
    colours are RGB 1.0, so it outputs exactly texel × alpha.
- Opaque draws are unchanged: `--stage 11 --shot` before/after differ only on court lines, markers and moving
  players (static scenery pixel-identical) — `context/shots/b31/cmp_s11.png`.

## Checks
- Court lines (`line@add`, stage 11, rows 318/319, 270+ px): line − grass per channel before
  68/62/78 (dark blue channel gains most: linear-light add), after 84/83/83 (constant: gamma add of white).
- Tornado on/off pairs (`research/p17q_freeze.py 5 1.0` + on byte cleared → `context/shots/b31/orig_on/off.png`;
  port: temporary hook pausing `Time<Virtual>` once on, not committed → `port_before_*`, `port_after_*`,
  `port_after53_*` frozen at a scale like the original's 5.33). Slope of the added value vs the background value
  (all changed px/channels): orig −0.09, port before −0.48, after −0.23 / −0.33 (rest is clamping at 255 where the
  port adds more). Per-channel mean add: orig 25.6/25.7/25.6, after 45.7/46.7/43.8.
- So the blend space matches; the port's tornado still adds ~1.8× the original's per pixel (percentiles 50/90/99
  after 28/123/183 vs 17/66/114) at a similar scale: new gap B31b (alpha/vertex alpha/texture of the five layers).
- `tools/check.sh`: 238 tests pass.

## Pitfalls
- `--stage 10 --shot` came out black at 461×244 several times (also with the old binary): the window got a
  half-height tile from other agents' windows. Stage 11 or the hook's own screenshot worked.
