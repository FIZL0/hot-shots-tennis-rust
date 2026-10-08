# P11a: AI parameter table (AIParam.csv)

P11 is split into P11a–P11h (PLAN.md, plan/P11a.md … plan/P11h.md). P11a is done: `hst_sim::ai` holds the table and the
row choice. Test: `crates/hst-sim/tests/ai.rs` — the parsed table equals RAM (context/ram/s05.bin) for all 168 rows byte
for byte; the slot-5 bots (characters 0,2,1,5, outfit 0 → rows 84,86,85,89, level 3, reach 3.2) match `menu_row` /
`Choice`; unit asserts for outfits and the forced level.

## The original

- AIParam.csv (PCDATA/PCDATA.XB → data/taguchi/Data/AIParam.csv, Shift-JIS): 168 rows. Rows 0–83 are singles, 6 blocks
  of 14 characters; rows 84–167 are the doubles copy (row + 84).
- Loader: strtok on "\n\r" for lines, "," for cells, "/" for sub-cells. Skips the '#' cell and the name. Rank =
  atoi % 17. Style ALL = 3, NET = 1, BASE = 2. Stops after row 167. The record is 0x118 bytes at 0x3174c0; the layout
  is `AiParams::bytes` in crates/hst-sim/src/ai.rs.
- Decimal cells use the game's own reader: the fraction digits first (w = 0.1, w /= 10), then the integer digits right
  to left (w *= 10), each a multiply-add. Reproduced bit-exact with `ps2::madd`.
- Row choice from the menu: block = (outfit > 4) + 2·(outfit == 4) + 2·(outfit == 9), block 2 → 3; row =
  block·14 + character. It is stored in the low byte of the player slot's setup word (slots of 12 bytes at 0x2ef7f4:
  +0 character, +1 outfit, +8 setup word). Challenge mode takes the row from challenge.csv (COM番号).
- At match start (function 0x35cf70): if setup byte 1 is 1..4 → level = byte1 − 1, row = character + 14·level. Else
  level 3 (a six-entry table, all 3), row = byte 0. Clamp to 83; +84 in doubles; byte 2 is kept as the strategy.
- The AI object sits at player+0x80 (vtables 0x1d20e0, 0x1d2100 for doubles): +12 row pointer, +16 level, +17
  strategy, +20 float 2.5 (+0.35 at level 1, +0.7 from level 2).
- Code ranges for the later sub-tasks: ~0x35cf70–0x35f450 and 0x3c81b0–0x3d4400. Update methods 0x3c8850 and
  0x3ce7c0. Original call-out timing 0x3d5b20.

## In the app

- `play.rs`: `Player` has `ai: AiParams`, set at setup with outfit 0 (the app has no outfit choice yet).
- `bot()`'s stand-in "yours" call-out is gated to `g.shots >= 2`, so it never fires on the serve (user note).

## P11e1 (doubles formation)

See 2-P11E1-FORMATION-FINAL.md. P11e is split into P11e1–P11e4; P11e1 is done (`hst_sim::position`, test
`tests/position.rs` against `context/fixtures/ai_pos_s05.bin` from `tools/record_ai_pos.py`).

## P11e2 (singles centre and net dash)

See 3-P11E2-SINGLES-FINAL.md. `hst_sim::position::Single`; test `tests/singles.rs` against `context/fixtures/ai_pos_singles.bin` (bot-only singles recorded from a menu save state; the route is in the entry).

## P11e3 (guessing)

See 3-P11E3-GUESS-FINAL.md. Done: `AiParams::guess`, `Guess`, tests `guesses_match_the_game` and
`guess_verdicts_match_the_game` (fixture `ai_guess_s05.bin`).

## P11b (AI object)

See 4-P11B-MIND-FINAL.md. Done: `hst_sim::ai::Mind` and the serve spot/wait, test `minds_match_the_game` (fixture
`ai_mind_s05.bin`); `play.rs` `ai_update` / `ai_heard_shot` / `bot_serve`.

## P11c (reaction)

See 5-P11C-REACTION-FINAL.md. Done: `AiParams::reaction`, test `reactions_match_the_game` (fixture `ai_pos_s05.bin`);
`play.rs` `ai_draw_guess` / `ai_guessing` / `bot` hold the bot for it. The shot record's kind is the branch (3 dive, 4 smash).

## P11e4 (strong-side run-round)

See 6-P11E4-RUNROUND-FINAL.md. Done: `AiParams::run_round` / `run_round_width` / `stand_side`, test
`stand_sides_match_the_game` (fixture `ai_side_s05.bin` from `tools/record_ai_side.py`); `play.rs` `ai_stand_x`.

## P11f (target choice)

See 7-P11F-TARGET-FINAL.md. Done: `hst_sim::aim` (doubles `pair_aim`/`aim_at`, singles `rally_aim`/`receive_aim`,
`lets_go`), tests `aims_match_the_game` (fixture `ai_aim_s05.bin`) and `singles_aims_match_the_game`
(`ai_aim_singles.bin`, from `tools/record_ai_aim.py ... singles`); `play.rs` `ai_aim` / `ai_lets_go`.

## P11g (shot and serve kind)

See 8-P11G-SHOTKIND-FINAL.md. Done: `AiParams::picks` (checked on all 155 timing draws in `ai_s05.bin`),
`aim::button`, the singles kind lock, serve toss/swing/aim and the quick-serve contact pick; `play.rs`
`ai_press_kind` / `ai_contact_stick` / `ai_toss_kind` / `ai_serve_kind`.

## P11h (dive and call-out)

See 9-P11H-DIVE-FINAL.md. Done: the AI dive (drawn chance, `swing::dive` on its own path, the kept draw) and call-out
timing (after the reaction hold, from shot 2, not while the partner swings), test `ai_dives_match_the_game` (fixture
`ai_dive_s05.bin` from `tools/record_ai_dive.py`); `play.rs` `ai_dives` / `ai_mate_busy` / `bot`.
