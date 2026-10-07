# P0b

- [ ] **P0b — Tennis rules and serve flow.** Proper match flow before polishing anything else, exactly as the
  original: server serves from behind the baseline alternating deuce/ad sides, the serve must land in the
  diagonal service box, faults and double faults, second serve, receiver positions, server rotation each game,
  point → game (deuce/advantage) → set scoring, side changes, and the original's serve timing: toss height and
  duration, the contact window and how the press timing during the toss affects the serve (port `37af20` and the
  serve branch of the hit routine `3467b0`), plus the delays between points. Verify against recorded serves
  and full points from the save states with P0's replay harness. (P6/P12 cover the remaining details.)
  Part 1 done: score, rotation, ends (`hst_sim::score`, wired into `play.rs`); its round1 replay test only
  covers the points captured so far. Part 2 done: line calls, point-over check, hit legality, umpire verdict
  (`hst_sim::judge`, in `Flight` and `play.rs`): 158 recorded calls bit-exact, 30 verdicts match. Faults/lets/
  net points are unverified until a capture includes the live ball `*(gm+0x88)` (the round1 capture's ball is
  the path predictor) — see journal `2-JUDGE-PART.md`. Rest split (journal `research/journal/2026-10-06-p0b-rules/`):
  - [x] **P0b3 — Live-ball verdicts.** `tools/record_p2m2.py 5 …` (fixture + live ball + rally block 0x3165f0) →
    `context/fixtures/match_s05.bin` (whole slot-5 match, 26400 frames); `tests/score.rs::match_s05_rally_block`:
    hit check and point-over check on the game's frames (the check sees the ball and body hit as of the previous
    frame), rally block equal every frame, 36 decisions (26 points, 8 faults, 2 out after the net) frame-exact
    with the recorded winner. Lets, double faults, illegal hits, rally outs not in this match — still unverified.
    Journal `3-LIVE-VERDICTS-FINAL.md`.
  - [ ] **P0b4** → `plan/P0b4.md`
  - [x] **P0b5 — Serve.** Done: `hst_sim::serve` (walk, three tosses, toss launch, serve contact search, box aim
    with strong-toss mistiming errors), `tests/serve.rs` 36 recorded serves exact. Shot parameters → P6. Original: Toss height/duration (`37af20`), the contact window and how the press timing during the
    toss changes the serve (serve branch of `3467b0`). Verify with vpad-driven serves from slot 3/4 (pad + ball
    - player state over PINE) through the replay harness.
