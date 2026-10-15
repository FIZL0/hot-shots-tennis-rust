# B410 — Sophie's pigtails: spring bones under a mirrored (left-handed) root

## Symptom
Sophie's (oob_pc02_sophie) pigtails and ribbons didn't move like hair: they jerked about and stuck out (e.g. one
pigtail standing straight up while her body swung).

## Cause
`spring::swing` turns each spring joint's parent by the world-space `turn` from the posed direction to the mass. It
brought that turn back into the parent's frame with `above.to_scale_rotation_translation()`'s rotation. A
left-hander's root has scale x −1 (`play.rs` mirrors left-handers like the game). For a matrix with a reflection,
glam's decomposition moves the reflection into the scale, so `up⁻¹·turn·up` is the turn conjugated by the wrong
basis: its axis comes out reflected. In a match, before the fix, the turned tip landed a median of 38° (90th percentile 92°,
max 177°) away from the mass it should point at (debug-print run, `context/b410/run1`).

Out of Bounds characters wear a golf glove on their racket hand, and almost all of them are left-handed
(`hand: left`). Every OOB mod with spring joints except Alex had broken spring physics: Jasmine, Sophie, Sasha,
Fernando, Sonia, Kate, Dino, Anya, Suzuki, Gloria. Alex (right-handed) was right only because his root isn't mirrored.

Second, smaller fault: on drawn frames without a 60 Hz step (most frames at an uncapped frame rate), the joint was
aimed at the last step's *world* point while the body had moved on. The hair then swung round as fast as the body
moved and ignored its limit (up to 95° off on such frames in the log).

## Fix (`crates/hst/src/spring.rs`)
- `turned()`: the local rotation from the full linear part of the parent's world matrix: `A⁻¹·turn·A·local`. This
  is right for any root scale, mirrored or not.
- Between steps the hair keeps the last step's direction from the anchor it had then (stored with the state), clamped
  to the limit against this frame's pose.

Stock disc characters: they have no spring joints. Their `.NOI` sway (`noise.rs`, also Fore mods' hair) works in
the rig's own model space (root = identity), so the mirror applies to the whole mesh like the original's. It has no
such bug. No other code decomposes a mirrored world matrix for character physics (grep of `crates/hst/src`).

## Checks
- Unit `a_turn_lands_under_a_mirrored_parent`: fails with the old decomposition for hand −1 (passes for +1), passes now.
- Unit `a_still_chain_settles`: Sophie's 10-link pigtail parameters on a head that jumps 30 cm swing out (> 20°) and
  then come to a steady rest.
- In-match debug run after the fix: miss median 0.000 rad, max 0.001; never past a joint's limit (Sophie, Alex).
- Visual: the root was mirrored and shaken (temporary hack, not committed). `context/b410/shake/grid.png` shows no
  springs / old / fixed at three moments. Old: a pigtail stands straight up. Fixed: the pigtails trail and hang.
- Smoke: a match with each of the 11 spring OOB mods runs with no panic (`context/b410/smoke/`).

## Not verified / not 1:1
- Out of Bounds' own dynamics solver is still not ported (as before, `spring.rs` header; notes/oob.md). The hair is
  a plain mass-spring that looks the part, not Out of Bounds' motion. There's no reference to compare against in
  this project: it's a PS3 game and the hair is a mod addition, not HST gameplay.
- The look was checked on stills from a shaken viewer rig and match shots, not side by side with Out of Bounds.
