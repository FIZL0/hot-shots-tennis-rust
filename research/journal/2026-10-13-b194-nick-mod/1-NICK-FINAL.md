# B194 — mod Nick (oob_pc01_nick) broken on the custom select

## Cause
Every costume failed `mods::check` with `rest pose must be the bind pose (max diff 3.17e0)`; the select's preview loader
marked each costume broken, so the card couldn't be picked. The only joint off was `__bansoko:tape_mirror` (under
`Bip01Head`). Nick is left-handed (gloved left hand), so the packager pre-mirrors his costumes with opentennis-mods
`tools/rerig/mirror.py`. His head tape `__bansoko:tape` has no left/right twin, so mirror.py adds a mirrored copy. That
joint has a non-uniform scale (4.99, 1.48, 1.48), and the copy wrote only translation and rotation, so the scale was lost.
Nick is the only mod with a scaled joint that needed a copy.

## Fix
- opentennis-mods 1376d3b: `mirror.py` `copy()` splits the copy's local matrix into rotation and column-norm scale and writes
  `scale` when it isn't 1.
- I re-mirrored Nick's six costumes from `out/rerig/oob/pc01_nick/` into `out/mods/oob_pc01_nick/model/`. Nothing else
  changed, because mirror.py is the only step that writes those files. `mods/hst-mods` is a symlink to opentennis-mods
  `out/mods`, so the fixed mod is already in place.

## Checks
- `tools/rerig/check.py` on all 6 costumes: every check OK (before: FAIL rest = bind).
- `HST_MODS=mods/hst-mods cargo test -p hst --bin hst loads_every_mod -- --ignored`: 361 of 361 costumes of 92 mods
  load (before: Nick's 6 failed).
- I ran a bot singles match with `--mod mods/hst-mods/oob_pc01_nick` (`tools/shot.sh`, shot at 25 s,
  `context/b194/nick_c00.png`). Nick serves with the racket in his left hand and the point is scored (15-0), the same
  as the Sophie run (`context/b194/sophie.png`).

## Not verified / not 1:1
- I didn't click through the select menu with a pad. The menu only marks a costume broken when the same `load` fails
  (`previews.rs` `broke`), and that load now passes for all 6.
- Neither match shot drew the court, which is normal when launching without a court (user 2026-10-13). Launched
  this way, mod characters also have no rendered HUD icons; the user says that doesn't need handling.
- I only looked at costume 0 in a match. Costumes 1–5 pass the loader test and check.py, but I didn't look at them
  in a match.

## Also
- `mods::tests::mod_eyes_follow_the_donors_uva` was already failing on main. It wrote an absolute costume path into its
  temp mod.json, which M16's "path inside the mod folder" check rejects. It now copies Phoebe's .glb into the temp mod
  folder.
