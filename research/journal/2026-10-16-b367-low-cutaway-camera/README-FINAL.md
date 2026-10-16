# B367 — replay camera at foot level; mods facing away at VICTORY

## What the user saw
Replays are off in the app (`REPLAYS = cfg!(test)`, B127), so the low "replay camera" is the post-point cut-away
(`hst_sim::cutaway::Shot`, driven from `play/cutaway.rs`). It happened with Hot Shots Golf mods (T-Bone) and with
stock JJ: a cause shared by every rig.

## Cause
Cut-away shot 0x64 is the low-angle shot from the ground in front of the player (eye ~0.1 m up, looking up at the
head). Its channel 8–10 offset puts the eye on the offset basis. Our ground-clear step (record float 0xac) tilted
the camera toward that basis point, so the look vector had length 0, the asin argument went ≥ 1 and the view axes
blew up (~1e38): the camera sat at the eye position on the ground with a garbage orientation, i.e. "at foot level".
The original tilts toward the subject frame's own spot (the move-to frame, byte 2, else the subject frame, byte 0),
before the offset. Ported that: `cutaway.rs` `Shot::build`.

Who gets 0x64: `pick` never offers it for characters 0, 2, 4, 6, 10, 12 nor when `low`; JJ and the male golf
donors (T-Bone's donor) do. A mod uses the donor's number, so mods over male donors hit it as often as the donor.

## Mod rigs checked
Every installed mod's Bip01Head / Spine2 / Pelvis bones (the cut-away and replay camera's subject frames) read at
stock heights after the rig retarget (head ~1.0–1.1 m up, same forward axis as stock): the camera's aim isn't
scaled or offset by the mod's own skeleton. No height fix was needed beyond the shot above.

## VICTORY facing
`match_stats/ceremony.rs`: the ceremony z-mirrored every left-hander, which is right only for the `_n` files that
Carol and Will (6, 11) use (made for a model turned π). A left-handed mod over a right-handed donor got that z mirror
on files facing the camera, so it showed its back. New `stance(far, hand, turned)`: turned files + left = old z
mirror (stock unchanged); unturned + left = x mirror (like the match's `draw`); turned + right = turned π; else
identity. Racket counter-scale only for the z-mirror case.

## Tests
- `hst-sim/tests/cutaway.rs` `low_angle_shot_stays_finite`: shot 0x64 over 120 frames, both mirrors: view finite,
  eye at y ≈ −0.1, head inside the fov.
- `ceremony` `mods_face_the_camera`: forward sign per stance, stock left-hander equals the old transform.

## Not verified / not 1:1
- Shot 0x64 not compared with a PCSX2 recording: no cut-away fixture contains it and the pick is random; forcing it
  in the real game needs a RAM poke of the shot pick. Presentation only has to look right (PLAN), and the port
  follows the decompile.
- The ceremony stance for left-handed mods (and right-handed mods over Carol/Will) is our design: the original has
  no mod case.
- Racket counter-scale for the x-mirrored case: kept at 1 (mirrored with the body, as in the match), by eye only.
- The original's point also blends with a held matrix / a second blend branch in rare states; not ported here
  (not reached by the shots that showed the bug).
