# N2a — the ball bounce before the toss

The bounce is the serve stance itself; there is no separate bounce code or ball physics.
- Stance motion 0x20 is a 52-frame loop (t 0..51, speed 1) that plays until the server walks or tosses
  (`research/serve_bounce.py` over anim_s05: stance runs of 45–118 frames, 1–2 wraps, toss at any t).
- The character's `sh_pcNN_serve_ad00_ball` track takes the ball from hand height (0.50–0.70 m) down to 0.06 m
  (the ball on the court; game y is negative up) and back, twice per loop: lows at t≈18 and t≈44 (character 10
  at t≈15 and 41, character 5's track is a little different). Character 7's track is flat (−0.89): that
  character never bounces. So the number of bounces = how long the player waits; nothing random, no input gate.
- The serve input handler (mode 0/1) switches to the toss on any shot press, at any stance time: pressing cuts
  the bounce short. Sideways stick switches to the walk (mode 1), ball to the left hand (N2).
- The mode-0 placement code has no sound call; a bounce sound, if any, comes from elsewhere (motion events?) —
  left for N3 (audio).

Verified: `anim_s05_held_ball` (tests/serve.rs) now also asserts the verified stance frames include the bounce:
1316 stance frames bit-exact vs match_s05, 92 of them with the ball at the court. The app already plays it
since N2 (`motions` loops 0x20, `held_ball` places the ball on the track).

Open: bounce sound (N3).
