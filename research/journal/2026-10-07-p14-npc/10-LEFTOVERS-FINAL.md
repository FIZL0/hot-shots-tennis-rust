# 10 — Startled creatures' sounds and leftovers (P14c3d)

## Kept and checked
- **Startle sounds (+200 timer).** `step_near` already returned the timer's sound. Play now pushes every returned
  sound (program 7, volume 0x40, at the creature). Check: `startled_creatures_match_the_game` asserts that a
  stepped tick returns a sound exactly when the creature's sound handle (+0xc4) changes. Court 1: 2 sounds.
- **+0xbc is "had its go", the port's `Trigger::done`.** The startle callback sets it. The reset leaves the creature
  inactive when the row doesn't rearm it and +0xbc is set. Msg 6 and 0x1b clear it. Check: both proximity tests
  read `done` from +0xbc and compare the whole `Trigger`.
- **"Controller speed" is not a speed.** Msg 0/4 writes 1.0 (type 32) or 0 (the others) to
  `*(*(*(o+0x5c))+0xc)+0x5c`. That is the model's mesh object, whose +0x5c `FUN_00148940` multiplies into the
  per-sub-mesh factor it draws with. It is kept as `Trigger::weight`. Save slot 7 (court 1) RAM shows 0.0 for
  types 0 and 1. No state has a type 32 (trg_BA-02), so its 1.0 is from code only.
- **Facing angles (+0x184/+0x188 = heading of home row 2).** These are now also set at the 27–29, 34 and type-15
  route-0 resets. Check: `passing_ball_matches_the_game` compares pitch/yaw through its 3 resets. Writing 9.0
  there fails it at vsync 44009.

## Message 0x14 to the ball
- The ball's handler (378ca0 → 378fb0) is the same one a body hit runs: one ball step against the plane facing
  back along its flat heading. Play now runs `Flight::step_plane` when a creature reports `struck`. It runs in
  the creature step, after this tick's ball step and so before the next one.
- `prox_c07_ball.bin` (context/fixtures, 230 samples) was recorded from court 7 (copy 2's slot 9 copied to slot
  8) with `HST_BALL=1` (the new record_npc.py opt-in that appends the ball's 0x290 object):
  `HST_PAD="sleep 300;press cross 100;sleep 330;press cross 100" HST_POKE="125:0:b:0:0;150:1:b:0:0;175:2:b:0:0;200:3:b:0:0"`.
  - The serve swing never landed. The poke at sample 125 struck the falling toss.
  - Two earlier runs, which poked the rising toss, did the same.
- **Mismatch.** In the struck tick the ball goes from vel (0.0035791, 0.0621199, 0.0040526), frame 59, state 0, to
  vel (0, 0.0074268, 0.0003243), frame 61, state 1, with bounces and contacts at 1. x does not move by a single
  bit, and every object's message turns 0x11.
  - Frame +2 and the unmoved x mean the plane step ran first and then the ball's own step from a zero vx.
  - The port's plane step finds no contact here. The toss moves 0.006 a frame flat, which is less than 0.98r, so
    `contact::sweep` drops it.
  - Letting that through still gives a reflected vx ≈ −0.0032, not 0.
  - So the toss-time response comes from more than `step_plane`. Possibly 0x14's other listeners (players' 35f070,
    the AI) or the 0x11 they lead to.
  - The ordinary steps before and after match bit for bit.

## Open (→ PLAN)
- P14c3e: the struck toss: find what stops it dead (vx 0, vz 0.000324) and sends 0x11. Done when a test steps
  `prox_c07_ball.bin` through sample 125→126 bit for bit. Also 0x14's other listeners: the players (35ef00(p, 1)),
  the AI (as 0x16), paths (35e150), and the landing markers hiding.
- P14c3f:
  - draw with `Trigger::weight` (find what the sub-mesh factor does in 149140; record a type 32);
  - msg 0xe's save/restore of +0xbc via +0xbd (replays);
  - +0x280 cleared at resets.
