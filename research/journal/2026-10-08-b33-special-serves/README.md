# B33: special serves (FINAL)

User: special serves (Will's curve, Lola's curve) with their own effects and sounds seemed missing.

## What the original does

- **Trigger** (`shot::special`): a strong-toss serve with topspin/slice hit within a frame of the sweet one; any
  other shot needs grade 1/2. The ball path (bend, curve, first-bounce turn, Lola's up1 slice table) was already
  ported (P6) and wired in `strike`/`serve_bent`. Will's two special serves in `serve_recording.p2m2` (contacts at
  vsync 10881 and 11498) bend 0.575 and turn −10°, as `shot::effect` gives.
- **Effects**: frame-by-frame screenshots of the 10881 serve (`context/b33/fl_*.png`) show only the clean-hit set:
  topspin impact burst, ball sparkle, ♪ balloon, speed readout, flight ribbon, bounce mark. All of them were already
  ported (P9/B21/B23). There is no extra visual for a special in live play. The code paths that call the special
  check are the flight sound (below), the ball path, and the instant replay (gm+0x344, which is out of scope
  until P0b4d).
- **Sound**: the flight-sound object (`*(0x3165c8)`). Each hit (33ea10) restarts it in a mode chosen by the shot:
  - Kind 0 or 2 (topspin, flat): the rush (mode 2) on a serve at ≥140 (km/h × 0.9).
  - Kind 1 (slice): the rush only for character 10's special serve.
  - Kind 3 (lob): the whistle (mode 1).
  - Kind 4 (drop): nothing.
  - Smash (fx+0xdd = 4): the rush at ≥140.
  - A framed mis-hit plays the whistle in place of all of the above, except on a lob.

  Rush start (1a1b60):
  - Plays program 5 key 6 at a volume band chosen by speed: ≤149, ≤159, ≤169, ≤179, ≤199, above.
    - Smash: 0x46, 0x50, 0x5a, 100, 0x6e, 0x80, speed threshold 50.
    - Otherwise: 0x55, 0x5a, 100, 0x6e, 0x73, 0x80, threshold 100.
  - Character 10 with a special: key 0 (the same tone), volume 0x80, threshold 50, play speed 1.5.

  Rush per frame (1a2680):
  - Re-places the rush at the ball every frame.
  - Once the ball speed (+0x130, km/h) has fallen to the threshold, it sets the sequence volume to (volume after
    falloff) / n for n = 2..4, then stops at n = 5 (or when that quotient is < 1).
  - The first bounce stops it (1a10c0).

  Whistle variants on a special lob:
  - Characters 5 and 13: key 5.
  - Characters 5, 6, 9 and 13: start range low 0.3, top 8 m.

So Will's "special serve sound" is the fast-serve rush, which every topspin or flat serve at ≥140 gets. Lola's
special serve is the only special with a sound of its own (key 0 at 1.5×). The other per-character specials are the
four characters' lob whistles. No other character has a special-only sound.

## Captures

- `tools/play_p2m2.py` gained `HST_SAVE_AT=<vsync>`, which saves scratch slot 9 during the replay. The PINE save
  lands about 14 frames late: saved at 10720, 60 frames before the toss.
- `rec_serve.py` (scratch) loads slot 9, keeps writing the recording's pad bytes, and records `record_sound.py
  … hits` samples:
  - `context/fixtures/special_serve.bin`.
  - `special_serve_lola.bin`: the same serve with 0x422fa8[0] (the server's character) poked to 10 at vsync 10870.
- Will's serve keys program 5 key 6 (`a7ff/9fed`, co_se10 sample 0x2a9a0, bank at SPU 0x89b90) at 0x73
  (216 km/h × 0.9 = 194). It is re-placed for 25 frames and keyed off at the bounce.
- With Lola: key 0, scale 0x1800, volume 0x80.
- `hits_s05.bin`'s smash (143) keys it at 0x46.

## Port

- `sound::Flight` (crates/hst-sim/src/sound.rs) has three parts:
  - `start`: whistle or rush, with every variant above.
  - `play`: the start play.
  - `frame`: the per-frame play, and the fade volume or stop.
- Test `rushes_match_the_game` covers the three captures:
  - Every predicted rush keys at the hit (L/R, scale), is re-placed every frame and stops at the bounce.
  - No rush keys without a prediction.
- play.rs:
  - `flight_sound` (new) is called from `strike`. The whistle flag now follows `Flight::start`, so framed mis-hits
    whistle as they do in the game.
  - `play_sounds` plays `Flight::play`/`frame` and does the fade with `Sound::turn`.
  - The AI's special-serve reaction frames are now drawn (`Game::special_serve`, 3467b0's record +0xd): strong toss,
    |offset| < 2, topspin by 11/12 or slice by 10.

Not exercised by a capture:
- The fade: no rush in the captures falls to its threshold before the bounce.
- The lob variants of characters 5/6/9/13.
Both are ported from the code. The per-frame threshold uses the port's ball velocity; the game reads +0x130, which
differs from +0x140 by under 1 km/h in flight.
