# B543 — mod characters' feet plant and bend like stock ones

## Cause
Two separate problems, in every mod (it isn't one pack):
1. **Foot bind frame.** `rerig.py`'s `fit()` swings each joint's bind frame to point at the source's child joint
   (Foot → Toe0). A mod's foot frame therefore sits at a different angle from HST's. The donor's motions turn the foot
   frame, so the whole sole (and the toe frame with it) comes out tilted.
2. **Leg proportions.** Donor clips play their rotations unchanged on the mod's own leg lengths and hip offsets, and
   only Bip01/Pelvis get position keys (Bip01 scaled by the hip-height ratio). Whenever the leg isn't straight down,
   the foot lands at a different height from the donor's (+30..+120 mm under the court on average).

## Fix (draw-only, sim untouched)
- `mods::level_feet` (at load): rotates each foot subtree's inverse binds about the ankle, so the foot's bind
  rotation equals the donor's foot bind from the disc model (`character::disc_body`). It also records each
  sole's heel/toe ground points, both the costume's and the donor's.
- `character::Feet::lift` (each frame in `animate`): FK of each foot chain twice, once for the costume's drawn pose
  and once for the donor's own skeleton under the same clip. The donor pass uses its rest rotations and
  translations, with keyed positions scaled by the bone ratio. The root then moves so the lower sole sits where
  the donor's sole would. A mod's own clips (all track scales 1) get 0.
- `data.motions` / `data.skeleton`, which gameplay reads, are not touched; only `Joint.inverse_bind` and the drawn
  pose change. No mod was rebuilt, so no `mod.json` version bump was needed.

## Measured
`HST_MODS=…/opentennis-mods/out/mods tools/check.sh -p hst-base feet_against_the_donor -- --ignored --nocapture`
CPU-skins each foot over motions 0,3,7,0x10,0x11,0x16,0x1e,0x20,0x25 (12 samples each). It reports the lower
foot's lowest point minus the stock donor's (mm, + = into the court), for all 82 character mods:

| | per-mod mean | stance heel/toe |
|---|---|---|
| before | +30 .. +120 | +10 .. +115 |
| leveled feet only (`HST_NOLIFT=1`) | −16 .. +74 (median +18) | |
| leveled + lift | −18 .. +15 | ~ −13 .. +1 |

Per-sample extremes after the fix are about ±100–150 mm. They come from run, dive and lunge frames where the
mod's sole shape or its other foot differ from the donor's.
An apparent +3 m spike in the serve (0x25) came from the measuring helper, not the game: it skipped
`rest.scale`, and some mods' `Toe0Nub` has scale 0.01. `animate` applies it.

Tests: `tools/check.sh -p hst-base mods` (48), `character` (6), `-p hst-play mod_match` (6) pass;
`loads_every_mod` loads every character costume (the 24 failures in that folder are court/umpire mods, by design).

## Not verified / not 1:1
- On-court side-by-side `--shot`: the viewer and the `--play` shots taken (context/b543/run1/) drew no court in
  this worktree's setup, so ground contact couldn't be judged by eye. The skinned-vertex measurement against the
  stock donor above stands in for it.
- Horizontal foot sliding: stride lengths differ with leg length; the lift only fixes height (no IK).
- The swing foot and odd frames (dives, lunges): the lift sets down only the lower foot, and the other foot follows
  the mod's own leg (±150 mm worst case).
- Sole model: two ground points per foot (heel, toe) from the lowest foot vertex, not the full sole outline.
- No PCSX2 comparison: looks only, against the stock character in our own renderer (stock is the disc's own pose).
