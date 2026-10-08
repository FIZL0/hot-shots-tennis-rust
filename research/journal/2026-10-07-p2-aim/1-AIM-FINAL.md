# P2 — Aim, final

## What the original does
- The aim runs at contact countdown 0 (34ccf0, called from 349410 with the stick world vector and the buttons), then the compute 34c250. The result is written to player +0x3e90 (x, y, z, w) and +0x3e48 = 1.
- **Sweet** = branch ≠ 3 (dive) && !(+0x3f50 & 4, body shot) && |timing offset +0x3fa0| < 2.
- **Depth**: L = 8.885 (9.885 for a drop: kind 4 on branches 1–3). Not sweet: L −= 1.0 (lob, kind 3: −= 0.5). half = L/2; base z = (near + half)·end, near = 2 for a drop else 3.
- **Width** = 5.485 in doubles (player count global 0x422fa4 == 4), else 4.115 in singles; −0.5 when not sweet. So a sweet full-diagonal stick lands exactly on the corner (5.485, 11.885) doubles / (4.115, 11.885) singles. This was the user's report: the old aim had width 4.4/3.4 and depth 8 + 2.8, falling short.
- **Stick → square mapping**: off = n·(1/max(|nx|,|nz|))·|stick|·(width for x, half for z).
- **Button** (param 0x20) sets the z offset = half·end. A plain smash with a backward z offset gets 0; a drop gets −half·end.
- **Angle limit**: A = (doubles ? 3 : 0) + CON (TParam col 22 Serv for serves, 20 Strk for strokes, 21 Voley otherwise).
  - Plain smash with |offset| > 1: scaled via msub(1, 0.4, (clamp(|z|, 3, 11.885) − 3)/8.885).
  - A = 90 for stroke kinds 1/3 or rally kind 3.
  - With no incoming slice: Body ADJ (col 23/24) and Rizing ADJ (col 26, branches 1/2 below the underhand limit, col 62 cm/100) as a·v/100.
  - Incoming slice multiplies by 0.6 (not sweet) / 0.45 (sweet). A drop ×0.5.
  - If the angle to the target from the swing-start position (+0x3ef0/+0x3ef8, copied at swing/dive commit) exceeds A, x is pulled to from.x ± |dz|·tan(A).
- **Incoming**: shots this rally (0x423060) > 0 and the last shot's record (player +0x1400 → object +0x1b0: hitter, branch, kind, sweet byte; NOT the ball object) has branch < 4 and kind == 1 (slice). The aim runs before the hit routine writes this hit's record, so the record is the incoming shot's.
- Then the projection for a plain smash or rally kind < 2, and a near clamp t.x += short·dx/|dz|.

## Port
- Ponytail in shot.rs: the ±5/±10 timing nudge (+0x3ed4) and the smash's +0x3edc aren't returned. Bots aim through the same function with a random stick (the original AI aim is P11).

## Verification
`tools/record_aim.py` drives P1 (save slot 4, doubles) with the virtual pad. Findings while building it:
- Lined up with the ball, nothing swings: P1 must stand about 1 m beside the ball's crossing.
- The swing locks by itself, so the timing offset is the auto-lock's. Sweet hits are forced by writing +0x3fa0 = 0 during the lock on every other aim.
- Opponents are forced to slice (+0x3ee4 = 2) 2 in 3, half of them sweet, to cover the incoming multipliers.
- No singles save exists (all saves 3–10 are doubles), so AIM_SINGLES=1 writes 0x422fa4 = 2 over P1's contact (the aim reads the width from it) and restores 4.

Result: `crates/hst-sim/tests/aim.rs` `human_aims` — every recorded aim bit-exact (x and z) against the game's +0x3e90.
- Doubles: 9/9 (2 sweet full-diagonal corners at (5.485, 11.885), incoming sweet and non-sweet slices, drop, volley, part tilts, both ends).
- Singles: 9/9 (3 sweet full-diagonal corners at (4.115, 11.885), volleys, a drop) (sweet corner (4.115, 11.885)).
- The leftover 0.2–1° bearing offsets came from the old approximate target; the target is now the game's, bit for bit.
