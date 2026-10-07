# P6a — serve ball logic

User report: lob serves don't clear the net; sweet-spot serves don't fly like the original's.

## What the game does (verified on every serve of match_s05, `serves_launch_like_the_game`)

- Table: the **server's own** `tr_pcNN_serv{k}` (TRAJnnA.XB). A **weak toss** serves from the `dw1`
  variant `tr_pcNN_serv{k}_dw1` (TRAJnnB.XB, kinds 0..2): at the swing the game sets the shot's variant
  value to −5 for a weak toss, 0 for strong/underhand, and the selector picks `dw1` below −0.2.
- Spin: field 7 (degrees) of the character's shot record, class 0, × 0.017453292 with the FPU's rounding
  (`params::spin`; one ulp off with a host multiply on 25°). Weak toss: the variant record's spin.
- Variant records (the 0x48-byte per-character list in GAME.BIN, record blank on disc): built at start-up as
  lerp(record, archetype 0 if weight > 0 else 2, |weight|). All serve variants weigh −0.5 (dw1 only).
  `ShotParams::variant` + `exe::shot_variants`; bit-exact against slot5_ee.bin for all 14 characters
  (`variant_records_equal_the_games`).
- Scatter (mistimed strong toss): looked up from contact − scatter toward the aim, launched toward
  aim + scatter (`serve::launch`; `serve::target` now returns (aim, scatter)). Same scheme as the smash.
- Speed factor is 1.0 for every serve; no sweet-spot boost exists.

Results: 23 strong, 7 weak, 1 underhand serve: velocity < 2e-5 (launch trig isn't the game's, as for
smashes), flight frames and spin exact.

## The bugs

- Lob: the app spun underhand serves 337.5° (a stroke kind's spin). The records give c0 +25°, c1/c2 −12.5°,
  c5 −31.25°: the lob was diving into the net.
- Weak toss used the base table (≈0.1 m/frame too fast) and strong-toss spin; every character used pc00.
  play.rs now loads per-player tables + spins (`serve_tables`) and passes the scatter.

## Open

- Slice serves (kind 1): ball carries a wind (+0x240 ≠ 0) and no table matches (err ≈ 0.13): the
  curved-serve path, P6.
- Contact-height depth error (+0x3ecc from contact height vs ideal, ×6 for serves) and the stat scaling
  of the dw1 threshold are not ported; with the measured toss grades the weak/strong split above holds.
