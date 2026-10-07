# N4 landing markers — part 1 (held for the PCSX2 split)

## Where it stands
- Done, tested (`tools/check.sh -p hst-sim --lib effect`):
  - `hst_sim::effect::Effect` has a `looping` flag. The morph and alpha clocks wrap and the effect never ends.
  - `hst_sim::effect::SmashSearch` and `PathEntry` hold the original's smash-point search, with two unit tests.
- Not started: the app side (`effects.rs` loader, `play.rs` wiring), screenshots, and verification against the real game.

## What the original does (from the decompile)
The marker object hangs off the match manager. It loads two models from `PCDATA/PCCG0.XB`, under `taguchi/Other/`. Neither has an ANI, so pass an empty `ani::Anim`.

### `chakudan_p` (red)
- What it looks like: a red soft blob plus four Add-blend white rings pulsing outward.
- Clock: a looping 120-frame cycle (tpf 80, 9600 ticks). It is never restarted; it is only ticked while shown.
- Where it goes: identity matrix at (x, 0, z) of the shot's **aim target**, not the predicted bounce. That is `strike(..., target, ...)` in `play.rs`.
- When it shows: at every launch where `players == 1`, or where `shots > 1` (the serve return onward; never on a doubles or singles serve).
- When it hides: at the next strike (message 0x14) and when the next point is set up (serve phase entry and similar). It stays up through the point-over phase.
- Practice only (`players == 1`): a 30-frame timer counts down once the ball has bounced, then the marker hides.

### `smash_p` (yellow)
- What it looks like: a yellow soft blob plus six shrinking white rings.
- Clock: a one-shot of 120 frames. `Effect::start()` runs when the first point is placed, then it ticks every frame.
- When it shows: only while the live ball has 0 bounces, and until the next strike.
- Candidates:
  - Humans on the receiving team. The team is the last hitter's slot `& 1`: an even hitter gives slots (1,3), an odd one gives (0,2).
  - Each candidate uses its smash top `pair(64,0)/100` and the middle of the smash window `pair(64,1)/100`. The app's `reach()` only reads character 0, so `pair(64,1)` still needs adding.
  - Practice mode (`players == 1`) picks the candidate by comparing sides. Not ported.
- Predicted path:
  - At launch: entry 0 is the launch state, then 14 steps.
  - Each frame after that: 15 more entries while the last entry has fewer than 2 bounces. The cap is 600 entries.
  - y and vy are stored **y-up**: negate the app's Y-down values.
  - The search runs after each extension. Feed `SmashSearch::search` the whole path so far.
  - Unknown: whether the per-frame extend also runs on the launch frame. This is a possible 1-frame offset.

## Next
1. In `effects.rs`, add a `LandingMarks` resource.
   - Load both models with `model()` from `PCDATA/PCCG0.XB`. Allow a missing `.ani` (use an empty `Anim`).
   - Set `pulse.looping = true` and call `start()` once.
   - Write a `frame(red: Option<[f32;2]>, smash: Option<[f32;2]>, start_smash: bool)` that ticks, places and hides, and a draw that uses `pose()`.
2. In `play.rs`:
   - Drop the `LandingMark` cylinder, `landing()` and `mark_landing`.
   - In `strike()`, set the red marker to `target` when the rule above holds. Start a `SmashSearch` with the humans (`g.humans`) on the receiving team, and a path from a copy of `g.flight`.
   - Each fixed tick, extend the path by 15 and search.
   - Clear both markers on the next strike or on `Phase::Serve`. Hide the yellow one once `g.flight.bounces > 0`.
3. Verify with `--shot` screenshots of the app. Under the agent's own PCSX2, take frame-stepped `tools/screenshot.sh` shots of the original: load slot 9 (a rally with the red marker live) or slot 4, and compare position, size and colour.
4. Tick N4, write `-FINAL`, and update the REFERENCE Done list.

## Resume
- Model dumps come from `context/xb/PCDATA/PCCG0.XB/data/taguchi/Other/`. They were dumped with a throwaway example that is now deleted; `mdl::parse` and `mor::parse(..., 1)` recreate them.
- Decompile: run `sh context/fn.sh <addr>` for the marker functions (handler, update, search, place). Their addresses are in the session notes and are not repeated here.
