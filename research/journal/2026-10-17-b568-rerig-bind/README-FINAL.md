# B568: rerigged_mod_plays_forehand_and_run, "rest·inverse bind off by 0.0037"

The test mod wasn't at fault: `context/mods/test_pc00/model/c00.glb` dates from 2026-10-11, before the failure.
The rig code changed on purpose. `mods::level_feet` (B543, then B563 and the feet-ik merge) turns every foot-subtree
joint's inverse bind onto the donor's foot frame and lays the sole flat. After that, rest · inverse_bind is no
longer the identity for those joints. The test's rest = bind check still expected the identity.

Fix (test only): for a joint on a foot (`Feet::sides`), the check now undoes the turn that `level_feet` recorded
(`Feet::turn`: inverse_bind · T(a)·R(q)·T(−a)), the same way `own_clips_skin_the_feet_unturned` does. Every other
joint is still checked as before, to under 1e-3. `tools/check.sh -p hst-base rerigged_mod` passes.

## Not verified / not 1:1
- none: this is a test fix for mod rigging, and no gameplay or rendering changed.
