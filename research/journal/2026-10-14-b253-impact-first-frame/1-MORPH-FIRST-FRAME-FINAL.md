# B253 — ball-hit effect "stretches into place" on its first frame

## Finding
- The impact model (`yumoto/impact_<kind>_a`) is static apart from one morph and the material alphas: its
  add-blended disc (`2 - Default@add`) has a morph target whose deltas are minus its vertices, so at weight w it is
  (1 − w) of full size. The MOR weight starts at 0.9 → the disc is 10 % size on the impact's first frame and grows
  as the weight falls to 0 (sim weights/alphas were already bit-exact, `tests/effect.rs`).
- In the port the first frame showed the disc at full size (huge grey/violet halo with long streaks), shrinking to the
  right size a frame later. Isolated by hiding the sparks, then each material, at the serve impact (frame 166).
- Cause is Bevy 0.19: `prepare_morph_descriptors` looks up the mesh instance that `collect_meshes_for_gpu_building`
  only adds later in the same frame, so a morphed mesh that was hidden (instance dropped) is drawn once with no
  morph descriptor = unmorphed when it shows again. Every effect model with a morph (impacts, landing marks, umpire
  calls, menus' banners …) had this one-frame glitch each time it appeared.

## Fix
`effects::model` puts the joints and parts under a holder entity (`Collapse`); morphed parts are always visible
(`Visibility::Visible`, `NoFrustumCulling`) so Bevy keeps their instance, and `effects::collapse_hidden`
(PostUpdate, before transform propagation) scales the holder to 0 while the effect's root (or an ancestor) is hidden.

## Checked
- Port frames 166–170 (serve, top) and 232 (rally) before/after: first frame now a compact starburst at the ball,
  as the original's frozen first frames (slot 5, MOR/MTA time poked to 0, 2, 3, 5, 8 —
  `context/b253/orig3/`); before/after shots in `context/b253/port7`, `port9`.

## Not verified / not 1:1
- The game shows the impact's t = 0 pose for two frames (the frame it starts and the next), the port for one: a
  one-frame timing difference left alone (presentation, rendering-only rules; the sim's effect clock matches the
  recording as in `tests/effect.rs`).
- Side-by-side only by eye at different camera framings (the original's capture is from slot 5's bot match, the
  port's from an autoplay match on court 1); sizes compared relative to the players, not pixel-exact.
- Other morphed effects (landing marks, umpire call models, menu banners) get the same fix but weren't re-shot.
