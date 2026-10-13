# B173 — the receiver kept chasing a net-touched serve

## Repro

- `round1.bin` showed nothing wrong.
- A scan of headless singles matches (snapshot.rs `app()`, a random-walk human on 1P, 30k ticks per seed) found it
  with seed 1 at tick ~11245:
  - The human's serve clips the net and drops on the human's half (z −0.1 .. −2).
  - The bot (receiver, end −1) kept walking to its stand spot.
  - It then locked a contact in the point-over phase.
- The many net dashes in the scan follow the original's rules: a dash after a volley or smash, or the NET zone pick.

## What the original does (dispatcher message handler, P11k3 journal 14; path copy check, rally.rs `copy_path`)

1. **The ball and hits fed to the AI.** The path copy check gives the ball up when it touched the net (bounces ≠
   court contacts), is below the net top (y down: `!(y <= -0.91)`) and is on the hitter's half. A net-touched serve
   (shots 1, bounces ≠ contacts) has no path at all. The app fed:
   - `hits: [shots; 2]` (always equal);
   - the ball y flipped up.

   So neither case could fire. It now feeds `[flight.bounces, flight.contacts]` (a net touch counts only as a
   bounce, `ball.rs respond`) and the ball as the ball object holds it (y down).
2. **Message 0x16 (new path, the net touch) re-enters the state as 0x14 does.** Receive with state ≠ 3 →
   set_state(0); rally with sub ≠ 3 → set_sub(0). Without this the receive stayed in its walk state, which never
   looks at the path again. `Shared::hear` now marks `struck` on 0x16 too.
3. **0x16 on shot 1 sets the hold flag (`Rally::hold`)**, so receive/rally return before doing anything for the
   rest of that serve. Hold was never set in the app.
   - The original clears hold and `held` (+0xbc) in the AI object's point set-up; the app clears them on the serve
     message 0xe, which `next_point` sends there.
   - `held` is also cleared on 0x15, and on 0x16 in the rally phase, as the handler does.

## Result

On the repro the bot stands still from the net touch to the next serve and doesn't go for the ball. Test:
`snapshot.rs receiver_gives_up_a_net_touched_serve` replays the same random human (seed 1) to that serve and
asserts the receiver doesn't move and doesn't lock a contact until the next serve.

## Not verified / not 1:1

- **No PCSX2 recording of a net-touched serve.** The behaviour is ported from the decompiled handler and the path
  check, not compared to a capture: hitting the net on a serve on purpose through vpad wasn't attempted in this run.
  `ai_rally.rs` already checks the routines against recordings with the real hits/ball.
- **Hold and `held` are cleared on the serve message**, not in the original's point set-up routine. Both come in
  `next_point` with nothing reading the AI in between, so it should be the same. Not checked tick for tick.
- **`path_mode`/`path_gap` are still the plain path's (P11k6, existing ponytail).** So `held` is never set in the
  app; its clears are ported but nothing exercises them.
- **A net cord in a rally (shots > 1)** goes through the same path check and the 0x16 re-entry. The scan had no
  such case on the human's half to look at, so it isn't covered by a test.
