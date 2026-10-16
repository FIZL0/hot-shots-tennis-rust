# B429 — Get a Grip umpires clip through the umpire stand

Bug (user 2026-10-15): a getagrip umpire on a getagrip court sinks its legs/thighs into the chair and its shins
pass through the ladder.

## Cause

Two parts:

1. **Anchor.** Before B433 the drawn umpire stood at the disc layout's chair point, not at the court mod's own
   chair. B433 (concurrent, `court_mod::umpire_seat` + mods commit 6876461: each GAG court exports its
   `pc_locator` as `umpire_seat` / `umpire_seat__singles|__doubles`) moves the umpire onto the court's own seat,
   so B429 does not duplicate it (I had a copy of the same anchor code and dropped it to keep the merge clean).
   With the seat point, the seat top lies 2–3 cm under the pelvis; thighs and seat rest on it.
2. **Pose.** GAG's umpires sit with their shins drawn back under the seat (feet tucked onto a rung). The mod is
   scaled ×~1.48 (calibrated on Gloria so the umpire's size matches HST's), which is larger than GAG's real-size
   chairs, so the tucked shins end up crossing the stand's front ladder rails. HST's disc umpires let the shins
   hang down and forward and never touch the ladder. The GAG umpire rig has no leg bones (the legs are skinned to
   Bip01Pelvis only), so no animation can move them.

## Fix

In the mods repo exporter (`../opentennis-mods/tools/psp2gltf/export_getagrip_umpires.py`, commit c2ef499,
pushed): when baking each umpire, take the pelvis-only vertices below the thighs and rotate them 45° forward
about a knee axis (game x, at the thighs' front edge minus 6 cm), eased over 7 cm so the knee doesn't crease;
the normals turn with them. The pose is read from idle's first frame. Umpires whose feet don't hang below the
seat (Robo T.) are skipped; their geometry is unchanged. All 8 umpires re-exported into `out/mods`;
`mod.json` unchanged.

No hst code changed: stock umpires and disc umpires draw exactly as before, and nothing touches the sim/RNG.

## Checks

- A point-clash script (leg vertices within 1.5 cm of the stand mesh, posed in Python like the Rust LBS) on the
  courts' own stands: contacts dropped sharply on c01, c02, c06, c08 and c17, and partly on c09, where what
  remains is the feet resting at its footrest.
- Side-view `--shot`s (custom eye on the stand; scratchpad, not in git), with B433's patch applied temporarily
  to place the umpire on the seat:
  - GAG courts: c01 singles Natasha (old shin crossed the ladder rail, new is clear, foot out front), c02 Nate,
    c06 doubles Kanani, c08 Suzuki, c09 Gloria and c17 Helghast. All sit on the seat with legs in front.
  - HST disc courts: 1 Natasha, 2 Gloria, 5 Nate, 6 Helghast, 7 Suzuki and 10 Xavier. All sit on the disc stand
    (or court 10's column) with legs in front.
- `tools/check.sh` (see done.sh). One failure is unrelated to B429:
  `play::court_mod_match::a_court_mod_is_seen_and_heard_and_never_moves_the_match` reports "0 of the mod's 0
  people placed", because the built gag_c01_yamate has no people since mods commit 02c3719 (M112). B429 changed
  no hst code.

## Not verified / not 1:1

- The seated leg pose is not GAG's: the shins are swung 45° forward, an angle chosen by eye and by the clash
  test, because GAG's real tuck can't fit the ×1.48-scaled umpire into HST-sized stands.
- Not compared with Get a Grip running (the PSP game wasn't run). Checked only against our own stands.
- c04 doubles wasn't seen: a tree blocked the side view.
- c09: the feet touch its footrest (they rest there, not through it).
- Helghast's long coat hangs slightly below and behind the seat edge. Left as is.
- The anchor fix is B433's (it wasn't merged yet when this was checked). Without B433 the umpire still stands
  at the disc layout's chair point on a GAG court.
