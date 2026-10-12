# 11 — P14c3e: the struck toss and 0x14's listeners

## The struck toss (bit-exact)
- The original's ball step, in plane mode (a nonzero plane normal passed in), sweeps once against the plane and
  **responds whether or not the sweep met it**. On a miss the hit record still has the normal its stack slot held
  (never written by the sweep), t = FLT_MAX and material 0. On court 7 that normal is (bits 0x00988fcc, 0, 0, 0).
  normalize(−n) of that tiny x gives about −4.77 on the EE, and the response stops the toss dead:
  vx 0, vz 0.000324, bounces/contacts 1, state 1. The ball's own step follows (frame +2).
- Port: `Flight::step_plane` responds off `MISSED_NORMAL` when `contact::sweep` misses.
- Test: `crates/hst-sim/tests/bodyhit.rs struck_toss_c07` loads `prox_c07_ball.bin` sample 124 and runs the
  plane step and then the world step against court 7's world. It matches sample 125 bit for bit. A further world
  step matches 126. A 1-bit change of the normal fails it.
- 0x11 is the toss coming down (bounces > 0). The port already re-tosses on that (`serve_turn`, `hold_ball`).

## 0x14's other listeners, in play (`play/npcs.rs`, where a creature reports `struck`)
- Players: `Human::reset` by z, through `Shared::hear(…, 0x14)`. This was already there.
- Path object: claim −1 and every shot record n = −1 (`hear`). Its +0x144 (the yellow marker's point count) and
  red marker go too: `g.marks.red` and `g.marks.smash` are cleared, and the landing markers hide.
- AI: as 0x16, an AI in its receive (state ≠ 3) or rally (sub ≠ 3) goes back to its state's start
  (`doubles_ai.rs`, the `Mine::struck` flag set by `hear`). Unlike 0x16 there is no shots condition. The
  shots == 1 and mode-3 writes are 0x16-only.

## Not verified / not 1:1
- `MISSED_NORMAL` is uninitialised stack content (it looks like a heap pointer's bits). It may differ with
  another memory layout or call path. The port uses the one recorded value; where the stack slot gets it wasn't
  traced.
- The re-toss timing: in the original the struck toss keeps falling for 7 frames (samples 125–131) before it is
  back in the server's hand (132), and the ball's state turns 3 at ~145. The port's `serve_turn` re-holds on the
  first tick with bounces > 0. The serve flow after a dropped toss wasn't compared frame by frame, because no
  serve-side recording of it exists.
- The AI re-entry happens at that AI's next step, not inside the creature step mid-tick. The rolls are the
  same, but the order against other draws in that tick wasn't checked against a recording; no fixture has an AI
  rallying while a creature is struck.
- The path object's other 0x14 writes (+0x5c, +0x5d, +0x1a0 bytes, +0x54/+0x58, the +0x74… records) have no
  port counterpart beyond the claim and records. Their readers weren't identified.
- The marker hide is presentation and was not screenshotted against the original.
