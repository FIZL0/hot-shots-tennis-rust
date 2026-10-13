# B69 — the rally played on through a fault

## Cause
`strike` set `Phase::Rally` (and ran the rally's hit check) on every hit. A swing that locked onto the serve
before it landed out still came due after the fault was called (`Phase::Post`). Its strike put the match back into
the rally phase, so the receiver's return was live, the point-over check ran again, and the rally played on.

## What the original does (decompile)
- The players' message handler cancels a lock (player +0x3ec8) on 0x14/0x15/0x16 and on **0x19** (the players'
  reaction). On 0x17 (point over) it does nothing to players in a match (only in solo, player count < 2). A fault
  has no reaction, so a lock that comes due during a fault's call still strikes.
- The hit check (0x15 → match manager) runs only while gm+0x55 == 3 (rally) and sub ≠ 4. After the call the
  phase is post-point, so a late hit never reaches the judge. The ball flies, but the point stays over.

## Port (`crates/hst/src/play.rs`, additions only)
- `strike`: when the phase is `Post`, skip `Rally::on_hit` and leave the phase and the umpire's rally flag alone.
  The ball is still launched, the shot is counted, and 0x15 still reaches the AI (the next serve's set-up resets
  all of it).
- `react` (0x19): `cancel_locks(g, None)`, as the handler does.
- Test: `play/serve_check.rs` `a_faulted_serve_cant_be_returned`. A bot match runs to its first fault, then the
  receiver strikes during the call. The test checks that the phase stays `Post`, the rally state is unchanged and
  the second serve follows with faults = 1. It fails without the fix (phase back to `Rally`).
- Bot-only probe (60k ticks of doubles bots): every return came off a serve called `In`. The bots never reached the
  bug; it needs a human press locked before the bounce.

## Not verified / not 1:1
- No PINE lock-step recording of a faulted serve being swung at. It would need a human receiver pressing into a
  serve that lands out, which slots 3/4 can't produce without driving the server too. The behaviour is taken from
  the decompile (hit check gated on the rally phase, lock cancel only on 0x19).
- What the original's hit routine does with the ball and the shot count after the call (the hit routine is 6.7 KB,
  not traced). The port launches the ball as usual. It's cosmetic until the next serve's set-up resets it.
- 0x19's other effect on players (state +0x3fa4 = 2, the swing setup reset) isn't ported; the reaction motions
  already take over.
