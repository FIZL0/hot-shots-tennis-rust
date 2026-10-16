# B366 — mod characters' replays don't frame them from the front

## Cause
Not the rig. A probe over all 36 character mods in `context/mods` (getagrip, opentee2) posed each mod's skeleton
with its donor's gameplay clips and compared Bip01 / Pelvis / Spine2 / Head model matrices with the donor disc
character's: all rows 0° apart, left-handed mods included (handedness goes through `player_matrix` and
`head_frame`'s left flag the same way as Carol/Will).

The difference is the clips drawn: every one of those mods ships its own visual clips (`CharacterData::visual`, 4 each)
for motions 0x2c..0x2f, the end-of-point reactions. Their Bip01/Spine2/Head yaw differs from the donor's by up to
108–180° (most 179–180°). The replay camera's frames (`replay_camera::live`, used for the point's end record taken
at Post and for every live frame while the replay shows) read the bones through `cutaway::bone`, i.e. the donor's
gameplay clip on the gameplay clock. So the closing shot framed the donor's reaction front while the body on screen
played the mod's own reaction, facing elsewhere.

## Fix
`replay_camera::drawn_bone`: the bone from `CharacterData::drawn(id)` (the mod's own clip if it has one) on
`Motion::drawn`'s clock, as `character::animate` draws it; otherwise the gameplay clip on the gameplay clock, which is
exactly the old call, so stock characters (no `visual`) are unchanged. Presentation only: no sim state or RNG read or
written differently.

Test: `replay_camera::tests::frames_follow_the_drawn_clip` (stock = old `cutaway::bone`, bit for bit; with a visual
clip the frames come from it at the drawn clock). `the_match_point_replay_cuts_and_holds_its_closing_shot` still passes.

## Follow-up
B502: the post-point cut-aways (`play/cutaway.rs` `bone`/`predicted`) read the donor's clips the same way.

## Not verified / not 1:1
- No in-game screenshot of a mod's replay before/after: the evidence is the probe's bone angles and the unit test.
  Replays are "feel right" presentation (PLAN), and mod characters don't exist in the original, so there's nothing
  to compare against PCSX2.
- The camera still ignores the crossfade mix between clips (as before, for stock too).
