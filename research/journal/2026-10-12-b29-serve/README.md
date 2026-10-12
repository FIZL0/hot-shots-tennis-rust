# B29 — the serve, 1:1 with the original

Recorder: `tools/record_serve.py <slot> <frames> <out.bin> [vpad cmds]` (one sample per vsync: pad, match globals,
rally block, the server's player block and motion, live ball, path bounces, game phase); decoder
`research/serve_timeline.py [-a]`. Captures in `context/b29/run{1,2,3}/` (slot 3 doubles, Carol right-handed at
x=3, z=−12.25; slot 5 bots).

## The original's serve, frame by frame

Serve modes: 0 stance, 1 walk, 2 toss, 3 swing. Game phase: 0 input gate (~50 frames after a load: presses lost),
2 serve, 3 rally, 4 point over.

- **Walk**: x ± 0.0333/tick, walk motion by side (stick right on a right-hander = the 0x22 walk, left 0x21); release
  → stance motion. The walked stance is kept for the next point (slot 5: |x| = 2.7667).
- **Toss** (press P): mode 2, toss motion from t0 (underhand its own motion); ball released at P+45 (motion t 45),
  vy −0.0879 strong/weak, −0.0376 underhand (bot: −0.0844 for its own toss).
- **Swing press** P2: mode 3, countdown k frames to contact, swing motion at speed 8/k so contact lands on its
  frame 8; contact at P2+k. Rally phase 3 starts at P2+1. Recorded k: strong 17, weak 16, underhand 20, early 12;
  bots 4 and 8.
- **After contact**: the swing plays to its end (t 61, ~54 ticks), then stance mode, play motion.
- **Untouched toss**: at the bounce the server returns to stance and tosses again, no fault (re-toss).
- **Whiff** (press with no contact in reach): the swing plays at speed 1, turns into the miss motion ~9 ticks
  after the press (its contact frame); the toss bounces → next tick faults = 1, call Out, phase 4 two ticks after
  the bounce; the second serve starts from walk mode. User confirmed: a whiff is a fault.

## Port changes

The port had the whiff as a re-toss and shouted at the press. Now (`play.rs` `serve_whiff`): the whiffed swing plays
on into the miss motion, the shout (and its RNG draw) comes at the contact pose, and the toss's bounce is called out
with no shot → `Rally::check` counts the fault, `judge` gives fault / double fault. `simulate` runs the ball through
the fault. Test: `play/serve_check.rs` `whiffed_serve_is_a_fault` (press at the window's end, 140 ticks → Post with
1 fault) and the timeline dump `weak_toss_timeline` (ignored; prints the port's per-tick serve state for comparison).

## Frame alignment (port tick ↔ capture sample)

The original steps the ball before the players in a frame; the port runs `control` (serve_turn) before `simulate`.
So a port tick r's flying ball equals the capture's sample r+1, a held ball and the serve state equal sample r, and
the port's motion clock reads one ahead of the sampled one (it advances in the same tick). With that:

- the toss was released one tick early (`s.t` counts the press tick as 1; release `s.t > 45` now) — after the fix the
  port's release state (hand y −1.423, vy −0.0879) and the whole toss arc line up with B_weak tick for tick;
- the swing at B's press tick: frames 16, offset 8, speed 0.5, contact 16 ticks on (test `weak_toss_timeline`);
- the miss motion now starts the tick after the swing's contact pose (P2+9, as E2/E3), timed by the swing's motion
  clock (the serve counter stops when the fault ends the serve), the shout with it.

## Not verified / not 1:1

- **Serve contact IK** (body step and arm turns over the swing countdown, D: x drifts 0.00189): not ported, the
  server stands still → split out as **B29a**.
- **Rally phase start**: the original enters its rally phase the tick after the swing press, the port at contact (or
  at a whiffed toss's bounce). Nothing in the serve reads it; the receivers' AI might (their wake-up), not checked.
- **Phase 4 vs Post**: on a whiff the original's fault count rises the tick after the bounce (port: same tick), its
  point-over phase a tick later; the port's Post covers both, post-point timeline as `flow.rs`.
- **Miss shout tick**: put with the miss motion (P2+9); the original's draw order for it wasn't traced, only the
  motion switch was seen.
- **Left-hander**: not captured (slot 3's Carol is right-handed in the save); the walk/toss/swing mirroring is the
  port's existing handedness code.
- **Resets after a net touch, an out, a winner**: only seen via the whiff fault (second serve from walk mode) and the
  slot 5 bot points (walked stance kept); no capture each.
- **Swing speed 1.0 at countdown 1** (presentation: the original sets the swing's speed to 1 on its last frame
  before contact; the port keeps 8/k until contact — pose differs by half a frame for one tick).
- **Aim, scatter, speed of the hit**: with the release fixed, the toss arc and the contact point are bit-exact vs
  B_weak (release y −1.4231917 / vy −0.0878997, contact y −2.5034206), but the launch differs: vy one tick after
  contact −0.0140681 (port) vs −0.0141301. The serve target takes coins from the shared RNG and a scatter, and the app
  run's RNG state isn't the capture's (different match history), so this run can't tell an RNG difference from a
  launch one; `serve::target/scatter/depth_error` stay as ported and checked in `hst-sim/tests/serve.rs`. A bit-exact
  launch check needs the capture's RNG seeds poked into the port run (or a save state with the port's seeds).
