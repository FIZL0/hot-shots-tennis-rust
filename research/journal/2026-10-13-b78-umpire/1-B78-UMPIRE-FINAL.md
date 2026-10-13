# B78: umpire Anna doesn't move (FINAL)

## Why she stood still
- Anna is menu umpire 4 = `u04` (the game's umpire byte reads 4 in slot 5); her files and clips load fine
  (all 30 nodes bound, 7–11 keyed tracks move per clip; same for u00..u03).
- In the original her idle stays on frame 0 during play too (sampled: time 0.0 in every idle sample). What moves
  during play is her **head**: the umpire actor's tick calls the motion player's look-at on `Bip01Head` toward
  her look target every tick. The port computed that target (`Umpire::head`) but never drew it, so she looked frozen.
- The motion wrap was inverted in the port: the original's tick advances the motion when turned or once the match
  is over, and loops back to frame 16 (14 for umpire 3, Robot) only when the motion is the **idle**; a turn holds
  its last frame. The port looped the turn and never wrapped the idle.

## Fix (`crates/hst/src/play/npcs.rs`)
- Wrap: idle-only, 14 for `--umpire 3`, else 16.
- New `look` system (Update, after `character::animate`): the target in `Bip01Head`'s frame at its *rest* local
  under the posed parent; yaw = clamp(0.7·atan2(y, −z), ±70°) about x, tilt = clamp(0.7·atan2(−x, √(y²+z²)),
  −25°..35°) about y; head local = rotY·rotX·rest (rows). The motion's own head keys are dropped, as the game does.
- `research/b78_head.py`: pauses slot 5 at 14 moments, reads the head node's model rest local, parent world and
  the actor's target, and recomputes the game's stored head local: max error 2–5e-7 in every idle sample
  (`context/b78/head_check.txt`). One sample mid-turn was 0.046 off (likely torn read / the motion player's other
  rest-matrix branch).

## Compared
- `context/b78/orig/` (slot 5, court 10, umpire 4: two rally shots, two turns at frame 40) vs `context/b78/port/`
  (`--play --stage 10 --umpire 4 --pads -,-,-,-`; `eye_*` from a close camera via `HST_EYE` wrapper `eye.sh`).
  `compare.png`: post-point turn leans toward the court like the original's turn0. `eye_grid*.png`: head now
  follows play left/right. `umpires.png`: u00..u03 close-ups at 8 s and 15 s, all move (heads, Suzuki's arm on
  the turn, Robot).

## Not verified / not 1:1
- Umpires 0..3 against the original: not captured (needs a new match through the menus per umpire); the original
  runs the same actor code for all five except the 14/16 wrap, which the port matches by umpire number.
- Close-up side-by-side: the original's camera can't be moved, so original comparisons are at match-camera size
  (umpire ~60 px tall); timing of moments differs (different rallies).
- Head look-at when the motion player's crossfade flag is set (other rest-matrix branch) not ported; the turn
  sample was 0.046 off.
- Her texture animation (TAF) still not drawn (unchanged, outside this bug).
