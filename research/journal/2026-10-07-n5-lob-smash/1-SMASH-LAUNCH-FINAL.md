# N5 — lob smash launch (FINAL, 5 of 9 exact)

## How the game builds a smash
- The hit function calls the class-3 wrapper (`0x37b0f0`) with the floats (sx, sz', blend, 1.0) from `0x35b640`:
  - sx = +0x3ed0/20
  - sz = +0x3ecc/10, doubled when negative, then sz' = sz·(+0x3ee0) − (+0x3edc)
  - blend = +0x3ed8/10, scaled by timing
  - Then the target +0x3e90, kind (△ → 1, else 0) and the ball.
- `0x37b110`:
  - Writes class and kind to ball +0x58/+0x5c.
  - For class 3, clamps the blend to [−1, 1] and stores it at ball +0x60. Mode is 4.
  - `0x37f740` pulls the target inside the court, using margins from the 0x408d20 table (by class, kind and sign of the blend).
  - Scatter = 1.5·(sx, sz') in the shot frame.
  - Ball +0x70 = hit, ball +0x80 = target + scatter.
- `0x37e420`:
  - Looks the table up from (hit − scatter) to the pre-scatter target. The launch heads for target + scatter.
  - Elevation is multiplied by the scale (1.0 for a smash).
  - The class-3 "blend" does a second lookup with the same mode-4 table, so it changes nothing in the table result. It only blends the spin record (`0x37c590`, list 0x408e60 by group fa8[hitter]+3).
- Table:
  - Per player, the table set lives at gm+0x9c, +0x50 + group·0x154 + kind·0x44 + class·4.
  - In slot 5, group 3 / class 3 gives kind 0 = `tr_pc00_smsh0`, kind 1 = `smsh1`. The list's mode-1 entry is `smsh0_dw1`.
  - pc00/01/02 smash tables are identical.
- Bounds (`0x37c9a0`, class 3):
  - x [−0.5, −18.17]
  - y [−1.7, −3.05]
  - z [6.9425 for kind 0, else 3.0, 16.17]
- Spin: every recorded smash is 0.08726646 (5°). First-bounce spin, restitution, curve and bend are all 0.

## Verification
`shot_tables.rs smashes_launch_like_the_game`, on match_s05.bin (9 smashes, all kind 0, found by class 3 at ball frame 0 with speed > 0.3):
- 5 match to ≤ 9e-6 (velocity) with flight frames exact: vsync 9363, 22490, 22626, 27442, 30254.
- 4 are off: 8422 by 2e-2, 16993 by 3e-3, 18811 by 6e-2, 32632 by 3e-2. All four have +0x3ecc = −4.
  - No scatter along the shot fixes 18811, 8422 or 32632. Brute-force scatter at any direction doesn't reach 1e-5 either.
  - The near-net lift (0x41096c, |z| 0.5–6 and height < 1.3) can't apply, because the heights are > 1.7.
  - Unexplained so far. Suspects: the +0x3ecc/+0x3ee0 values at the hit frame differ from the recorded post-hit frame, or the `0x37f740` margin re-clamp.
- Contact frame and branch: already verified for 9 smash decisions in `swing.rs match_s05_contact_search` (P4).

## Port
- `shot::Bounds::smash(kind)`.
- `play.rs` loads `smsh0/1` from TRAJ00B.XB. On the smash branch it launches class 3 with smash kind (△ → 1) and 5° spin.
- Hit sounds now get the smash kind, as the game does (△ key 5).
- ponytail: not ported:
  - the timing scatter (as for strokes, P3)
  - kind 1's spin
  - the movement to get under a lob (the auto-approach, P7)
