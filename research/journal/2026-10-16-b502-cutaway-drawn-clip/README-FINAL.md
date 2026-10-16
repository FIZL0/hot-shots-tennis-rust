# B502 — post-point cut-aways frame a mod's drawn reaction

Found in B366: the cut-away (`crates/hst-play/src/play/cutaway.rs`) took its subject frames from the gameplay
(donor) clip, so a mod character whose own end-of-point reactions (0x2c..0x2f, `CharacterData::visual`) face up to
180° away was shot from behind.

## Change
- `cutaway::bone` reads `CharacterData::drawn(id)` (the mod's own clip, else the gameplay one). That covers
  `predicted` (the subject frames at pick time, `t` = 117/378 frames in: the drawn clock starts with the gameplay
  one on `Motion::set` and runs at its speed, so `t` is the same on both; `bone` clamps to the drawn clip's length)
  and the live head in `show`.
- `cutaway::drawn_time(data, m)`: `m.drawn.sampled` for a mod's own clip, else `m.clock.sampled`; `show` uses it.
- `replay_camera::drawn_bone` now calls those two instead of its own copy (same result).
- The donor root path (`data.paths`, team reactions) still places the player: that's gameplay position.

## Tests
`frames_follow_the_drawn_clip` in `play/cutaway.rs` (new) and `play/replay_camera.rs` (B366): stock frames equal
the gameplay clip on the gameplay clock; with a `visual` 0x2c they follow that clip on the drawn clock. Both pass.

## Not verified / not 1:1
- No in-game look at a mod character's cut-away (no PCSX2 needed: mods don't exist in the original; presentation
  only per PLAN, "feels right"). The test proves the frames come from the drawn clip, not how the shot looks.
- A mod clip shorter than 117/378 frames is framed at its last frame (clamped), as the drawn pose holds there;
  a looping one would wrap on the drawn clock but the prediction clamps — reactions 0x2c..0x2f don't loop.
