# B563: a mod's own motions draw the feet in the costume's own frame

## Cause
B543's `mods::level_feet` turns each foot subtree's inverse binds about the ankle so donor clips lay the sole flat.
A mod's own clips (§6 `motions.glb`) key the foot in the costume's own frame, so with the turned binds their feet
came out tilted by that same turn, toes up (Fore Hubert's `re_gu`, motion 44).

## Fix (draw only)
- `Feet` now also records each foot's turn (`turn`: ankle and rotation) and the joints in each foot's subtree (`sides`).
- `Feet::unturn(joints, local, u)` changes the drawn locals of the foot joints so they skin with the unturned bind:
  per joint `C = inverse_bind · turn⁻ᵘ · inverse_bind⁻¹`, local' = C(parent)⁻¹ · local · C. This keeps one set of
  binds, so the skins don't need swapping.
- In `animate`, u = 1 while one of the mod's own clips draws and 0 for a donor clip. Across a fade between the two,
  u follows the fade weight, so the foot turns smoothly.
- The sim, `data.motions` and the root lift are unchanged (the lift was already 0 for own clips).

## Checked
- `tools/check.sh -p hst-base own_clips_skin_the_feet_unturned`: on Fore Hubert's own clip 44 at 8 times, every
  foot joint's skinning (points 30 cm out) matches the unturned bind within 1 mm. Joints outside the feet don't
  move, and u = 0 leaves a donor pose unchanged.
- Viewer `--mod mods/hst-mods/fore_pc21_hubert --motion 44 --shot-at 0.6`: context/b563/run1/ab_0.6.png has
  the old run (context/feet/run3) on the left and this run on the right. The front foot no longer tips up.

## Not verified / not 1:1
- Only Fore Hubert's motion 44 was looked at by eye. The other mods with `motions.glb` are covered by the same code
  path, not by screenshots.
- No PCSX2 comparison: mod looks only, there is no original to compare against.
- The "uncommitted level_feet work (stand/soles, Jak's toes)" from another session was not in this worktree;
  this builds on committed ab6172d8.
