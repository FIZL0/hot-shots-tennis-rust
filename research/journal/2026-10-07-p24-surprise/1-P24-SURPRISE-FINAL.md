# P24 — surprise pop-ups ("!", sweat drop, "...")

Ported in `hst-sim/src/surprise.rs` (stage machine, who gets the sweat) and `hst/src/play/surprise.rs` (raising,
dropping, drawing). Captures in `context/p24/` (git-ignored); scripts `research/surprise_rec.py`, `surprise_point.py`.

## The pop-up list
The timing balloons, the "!" and the sweat drop all live in one list in the pop-up manager object (0x30-byte entries:
kind, sprite, player, stage, timer, alpha). Per-kind table (stride 0x18, read from the running game):

| kind | use | scales s0,s1,s2 | frames fade-in, hold, fade-out |
|---|---|---|---|
| 0 | timing balloon | 0.3, 0.15, 0.3 | 3, 45, 3 |
| 1 | sweat / "..." (doubles) | 0.3, 0.15, 0.3 | 3, 60, 3 (hold never counts down) |
| 6 | swirl (singles) | 0.35, 0.15, 0.3 | 3, 60, 3 (hold never counts down) |
| 7 | "!" | 0.3, 0.15, 0.3 | 3, 30, 3 |

Ageing: stage 0 t−−, alpha = min(128 − t·128/in, 128), at t<0 → stage 1 t=hold; stage 1 counts down unless kind 1/6
(or 3 in doubles); stage 2 alpha = max(t·128/out, 0), removed at t<0. Measured "!": 43, 86, 33×128, 85, 42, 0 —
38 frames (test `bang_frames`). The list ages the frame it spawns, before drawing.

Draw (kinds 0/1/6/7 alike): camera-facing square, bottom edge 0.5 over the head node, half-width
s0 · max(1, s1·z·t) · min(1, s2·z·t), z the view depth, t = tan(fov/2). Same as P10's balloons, so the port reuses
that anchor and formula.

**P10 off-by-one?** With hold 45 the original gives 48 opaque frames; P10's `balloon_alpha` gives 47. Not touched
here.

## "!" (kind 7, burefukidashi)
- Raised when the player's surprise counter (set to 2) is positive. The setter only runs when the controller id is a
  CPU (0 = pad 1, 33 = CPU; checked in slots 3, 4, 5), so it is **AI only**.
- The setter runs from the AI's change-of-pace and fast-ball branches (= port's `ai::Timing::reacted`), and when a
  guess is wrong (= `ai::Verdict::Wrong`). Port: `Player::surprised`, set in `ai_draw` and the `Wrong` arm.
- One per player; none while a note/sweet balloon is up for that player; spawning it removes the player's timing
  balloon; while it's up bunny/turtle balloons are dropped; a note/sweet balloon removes it.
- Not ported: kind 7 sprite 12 (a skull), spawned from a separate flag in another object; not a "caught off guard"
  case.

## Sweat drop (message 0x17 to the pop-up manager)
- Sent the frame the umpire's call is set (capture: call 255 → 5 and both pop-ups appear on the same vsync).
- Doubles, call ∈ {1 out, 3 double fault, 5 out after net, 6 illegal hit}: who = last hitter if any shot was
  hit, else the server. Removes "!" and balloons of who and partner, spawns kind 1 sprite 4 (A_fukidasi_10, sweat) over
  who and kind 1 sprite 5 (A_fukidasi_11, "...") over the partner. Call 6 covers the "hit a serve that wasn't served to
  them" case.
- Holds until message 0x18 (removes kinds 1, 6, 3). Capture: on for 197 frames, gone one frame before the call
  resets for the next point → port clears at the next serve set-up (`Phase::Serve`).
- Singles: kind 6 sprite 6 (e_guruguru swirl, 5 cells of 80 px across a 512 texture, cell every 3 frames) over who.
  **Not ported** (P24 asks for the "!" and sweat only) — follow-up if wanted.
- Change of ends: the port keeps the sweat through `Phase::ChangeEnds` until the next serve; the original's clear
  there wasn't captured.

## Verification
- Original: `context/p24/rec6/o01..o22.png` (slot 5, F8 at chosen stage/timer moments): "!" in o07/o08/o11, sweat
  + "..." after the "Net" call in o19–o22. `pt/point.txt`: per-frame list around the sweat point.
- Port: `context/p24/app/bang_3.6/3.9/4.3.png`, `crop.png` — "!" over the far CPU, right sprite, over the head. Width
  ≈3.0% of screen over the far player vs ≈2.8–3.4% for the original's balloon / "!" in o08 (same formula as the
  verified P10 balloons). Frame-exact alpha pinned by `bang_frames` / `sweat_holds`.
- F8 caveat: PCSX2 wrote fewer PNGs than F8s sent, so the F8→file mapping was matched by eye, not by count.
