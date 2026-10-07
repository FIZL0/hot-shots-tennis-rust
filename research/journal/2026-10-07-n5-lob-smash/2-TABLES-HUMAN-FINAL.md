# N5 pass 2 — every smash launches exactly (reopened 2026-10-07)

## The 4 "late offset 4" smashes: wrong character's table
- They weren't a scatter problem. The first pass looked every smash up in character 00's `smsh` table, but each
  character has its own `tr_pcNN_smsh{0,1}.dat` in `TRAJ{NN}B.XB`. The 4 off smashes were hit by a different character.
- Fix: the test (`shot_tables.rs smashes`) and play.rs (`smash_tables[who][kind]`, loaded per character like the rally
  tables) use the hitter's own tables. All 9 match_s05 smashes are now exact (flight frames and velocity).

## Timing scatter, corrected
- The scatter function accumulates and clamps in place: `+0x3ed0 = clamp(+0x3ed0 + +0x3ed4, −10, 10)`. The recorded
  `+0x3ed0` is already the result, so side = 3ed0/10/2 (the first pass's (3ed0+3ed4)/20 double-counted).
  late = 3ecc/10, clamped to [−10, 10], or [−10, 0] when +0x3eca is set.
- Smash launch: u = normalize(unscattered target − hit) and side axis = (u.z, −u.x).
  Scatter = 1.5·side·(u.z, −u.x) + 1.5·late'·u, where late' = (late < 0 ? 2·late : late)·3ee0 − 3edc.
  The ball's stored target +0x80 is already scattered. The test recovers u from w = target − hit:
  c = sqrt(|w|² − side²), u = (c·w − side·perp(w))/|w|².

## Auto-approach
- `swing::approach` is the walk a press runs before its contact search. It matches the frames of match_s05's 4
  approaches exactly (`swing.rs match_s05_approaches`). Play runs it and then searches once, which replaces the
  28-frame standing hold.

## Human smashes (✕, ○, △)
- `tools/record_human_smash.py <slot> <out> <frames>` (slot 4, PCSX2 at NominalScalar 0.5).
  - The opponents' locked strokes get +0x3ee4 = 4 (△) so they lob.
  - P1 steers to where the ball next comes down through 2.8 m, stepping the ball's current acceleration. The camera
    is behind P1: on the near half stick right is −x and stick down is +z; on the far half both are reversed.
  - P1 presses when that point is ≤ 4 frames away (earlier presses, e.g. at 3.9 m, whiff in smash pose).
  - Serving presses come every 40 frames, or the serve faults.
- `context/fixtures/human_smash_s04.bin` (7200 frames, local only) has P1 smashes at vsync 8022 ✕, 11375 ○,
  13715 △, 13825 ✕, 13931 ○ and 14042 △. Test `human_smashes_launch_like_the_game`: kind 0 8/8, kind 1 3/3 exact.
- vpad: the copy pads swapped cross and circle, so "cross" arrived as ○ (pad word 0x4000, shot code 2, slice).
  The swap was removed. The port binds were already right: South/J ✕, East/K ○ (slice), North/L △.

## Not done
- ponytail: play.rs has no rally timing scatter (P3), so its smashes launch along the unscattered lookup.
- No in-port playthrough of a smash: there's no headless rally harness. The table lookup, kind and approach are
  checked against the game only through the hst-sim tests.
