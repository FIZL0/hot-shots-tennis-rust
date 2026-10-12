# P17v7 — net shadow (bit-exact texture pass dropped)

**User redirect (2026-10-12):** "we dont need to go that exact for shadow casts, just note that the net doesn't draw
shadows for the holes and fix that. it doesnt need to be 1:1". P17v7's bit-exact shadow-texture vertices and
`pass_texture` into `shade::build` are **dropped**. P17v8 (planes from the disc) is still listed, but nothing needs
it now.

## The bug
On every court the net cast no visible shadow on the hole ground. The net (`tree/znet_s1000` doubles,
`znet_s0100` singles, category 17, code `00 00 30 00`) is a `shadow::Caster`, and all 5 of its meshes go into
Bevy's shadow pass (checked with a debug system: no `NotShadowCaster`, all visible). On court 10 the sun runs almost
along the net line (game dir ≈ (−0.59, 0.77, 0.25)), so the shadow is a strip about 0.3 m wide of 1 cm strands and
a thin band. That is below the shadow map's resolution and bias. Only a net scaled 4× left a faint mark.

## Fix (not 1:1)
`main.rs` plant loop: for `znet*` with a sun, each net mesh (once per mesh) is copied with its vertices flattened along
the sun onto game y = 0 (lifted 1 cm). It is drawn as a GS decal: MODULATE, colour black, Normal blend, `Test::Never`
(no Z), no fog, no cull. The result is Cd·(1 − A), with A = the texture's alpha × the shadow darkening.
`weather::CourtLook.decals` holds those materials; `weather::apply` sets their alpha to the same `darken` the hole
receivers get, so weather changes and rain scale it the same way. The Bevy net caster stays.

Shots (context/p17v7/): `side_before.png` / `side_after.png` (+ `_crop`), from a side eye
`HST_EYE=9,-4,7,-0.8,0.35,-0.5,0,1,0,0.6`; `before.png` / `after.png`, `after_singles.png` match view;
`orig.png` = PCSX2 slot 5. From the match camera the original's net shadow is hardly visible either.

## Not verified / not 1:1
- Not the game's shadow-texture pass: it is a planar projection onto y = 0, not onto the hole's triangles. Off-flat
  ground under the net would float or clip (court ground at the net is flat on the courts checked: 10).
- Overlapping net parts (net mesh + band, the two posts' faces) darken twice where they overlap. The game's
  shadow texture would not.
- The texture is sampled with the camera's LOD (gs.wgsl), not the shadow texture's filtering.
- Only checked on court 10 (doubles + singles). Other courts' suns are not looked at.
- No test: a rendering-only change, checked by eye.
