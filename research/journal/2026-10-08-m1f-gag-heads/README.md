# M1f — Get a Grip mods have no head

## Finding
Doesn't reproduce on main at 1831c12. The Get a Grip (and Open Tee) heads and faces are mesh nodes **without a
skin**, children of `Bip01Head` (`Bip01Head9face00_…`, `Bip01Head9head300`). Before M1b (f87dc68, merged 16:46)
`mods::load` only built skinned nodes, so those meshes were dropped: headless. M1b added the "a mesh without a skin
rides the nearest joint at or above it" path (`rides` in `mods::load`), with HST-MODS 6c781b2 writing the matching
standard rule. The report (PLAN line added 17:21) was most likely made on a binary from before that merge.

Checked the transform: the standard's inverse bind matrices are game space (`ibm[Bip01Head] · space · W = I` on
Emi), so the rigid vertices' `space * w[node]` is right. A first guess that `space` was applied twice was wrong:
with it removed, the head went upside down under the court. Reverted.

## Verification
- `--inspect` shots of all 15 `getagrip_*` (costume 0) and all 31 `opentee*` mods, each with head and face:
  `context/m1f/montage_gag.png`, `context/m1f/montage_ot.png`.
- `--play --mod …getagrip_pc00_emi` (slot 0 near, slot 1 far, `--outfits 0,1,0,0` for set4): heads drawn,
  `context/m1f/emi_play.png`, `slot1.png`, `emi_c1.png`.
- New test `mods.rs psp_mods_keep_their_heads`: every costume of every Get a Grip/Open Tee mod linked in
  `context/mods` (local symlinks to ../HST-MODS/out/mods) has vertices on `Bip01Head` above the neck at rest
  (62 costumes). It fails if the unskinned head meshes are dropped again.

## Not verified / not 1:1
- Second costumes of Get a Grip mods besides Emi's were checked only by the test, not by a shot (the inspect viewer
  shows costume 0 only).
- No comparison with the original: these are mod characters the original doesn't have.
- The `--play` shots draw no court (blank background); that happens for all players, not just mods, and isn't
  part of M1f.
