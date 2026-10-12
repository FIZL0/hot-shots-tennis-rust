# B51 — ball marks fading

**Question.** Do the ball's bounce marks fade out or get removed at some point in the original: after a time, a count, or at point end?

**Answer (from the decompile, checked live).** No messages clear them. A mark only goes when one of these happens:
- its 1800-tick life runs out. Alpha is `life·128/1800`, so it fades out over that life.
- a 21st mark pushes the oldest off.

They also outlast point ends, changes of ends and sets.

**The fading flag.** Life counts down only while the bounce object's fading flag is set. The message handler sets it like this:
- **On** at the play phase's enter (message 0x12). The match requests that phase as the server's swing starts (motion 0x25/0x26).
- **Off** at message 0x19, when the players react, about 33 ticks into the point-over phase.
- **Off** at the serve set-up's enter (0xe). This is what ends it after a fault, since no 0x19 comes then.
- **Off** at phase 5 (0x1a).

The serve set-up message also does replay bookkeeping:
- Outside a replay, it saves the marks.
- When a replay starts, it puts them back.

The port's instant replay already does the same: it restores `BallBounce` from its snapshot.

**Live check.** `context/b51/fade.txt`, from slot 5 over 4000 frames, polling bounce object +0xd08 (flag), +0xd00 (count) and the first mark's life. It shows:
- the flag goes to 1 on phase-3 tick 1;
- it goes to 0 on phase-4 tick 33/34 on every point;
- the count holds at 15–16 in steady play (expiry, not the cap);
- nothing is cleared between points.

**Port.** The marks already had the cap, the life and the alpha (P9f). The gap was the window: the port aged them through the whole serve phase and stopped right at the point's end. It now uses `play::marks_age`:
- serve phase: from the swing;
- rally: always;
- post-point: until the players react (`PostPoint::reacted`), so after a fault until the next serve.

## Not verified / not 1:1
- The port turns the flag on at its own swing start and off on the tick it runs `react`. The live check only covered the game's side. The port's aging was not compared tick for tick against a capture. Marks are rendering, so this follows PLAN's "look right" rule.
- The live check never hit a fault, so the fault case (aging until the next serve set-up) comes from the decompile alone.

## Tests
`tools/check.sh` passes everything except `net::tests::mod_looks_peer_to_peer_and_the_ready_gate`. That test also fails when run on its own: it times out on the local DHT testnet/iroh lobby path.
- This change doesn't touch that code: it is limited to `play::start_effects` / `marks_age` and a doc line in `effects.rs`.
- Main's latest commit already logs "join by code hangs" (efa1136).
- Rerunning it at efa1136 in a fresh worktree didn't build, so it is not confirmed there.

The task was ticked and committed by hand rather than through `done.sh`.
