# C3b — shots and match read without the decompile (FINAL)

Same rules as C3: no behaviour change. This pass names the raw codes, offsets and magic constants in
`params.rs shot.rs swing.rs serve.rs aim.rs instant.rs judge.rs flow.rs score.rs` and turns them into named
constants and small modules.

## What changed

- **`lines.rs`** (new): the court's lines as the game's literals: `BASELINE` 11.885, `LENGTH` 23.77,
  `SERVICE_LINE` 6.4, `SINGLES_SIDELINE` 4.115, `DOUBLES_SIDELINE` 5.485, `BOX_MIDDLE` 2.0575, `DEG` (the
  game's π/180), plus `sideline(doubles)`. The modules use these instead of repeating the literals. judge.rs
  drops its own copies (`SINGLES_HALF_WIDTH` …).
  - Values that are only *derived* from the lines (8.885, 10.385, 4.4425, 12.25 …) stay as literals. Computing
    them would change the f32 rounding risk for no gain.
  - flow.rs's server depth 12.25 is now `SERVE_DEPTH`, so it no longer clashes with `lines::BASELINE`.
- **params.rs**: the table shape is now `REC` 13 floats, `KIND = RECORDS*REC` and `CLASS = KINDS*KIND`
  instead of 0xd/0xdd/0x451. The archetype indices are `HIGH/MIDDLE/LOW`. `field::*` names the record's
  slices (launch, middle, spin group, side, restitution). The weight row is now indexed by float slot
  (`weight::ROW` 31, `SERVE_SMASH`, `STROKE`, …) instead of byte offsets. The blend matches on `kind::*`.
- **shot.rs**: the following are now named constants:
  - `class::{SERVE, STROKE, VOLLEY, SMASH}`
  - `kind::{TOPSPIN, SLICE, FLAT, LOB, DROP, UNDERHAND}`
  - `branch::{SERVE, GROUND, VOLLEY, DIVE, SMASH}` and `branch::rally`
  - `AimStats::{STROKE, VOLLEY, SERVE}`
  - `FLAT_CONE`, `NET_NEAR`/`NET_FAR`, `STROKE_TOP`
  - the pad bits as `replay::Pad::{TRIANGLE, CROSS, CIRCLE, SQUARE}`
- **rng.rs**: `r15` (the 15-bit `rand()` draw of a 32-bit word), `coin` and `TWO_POW_M32`. These replace the
  repeated `>> 16 & 0x7fff`, `>> 16 & 1` and `2.3283064e-10` in swing.rs and aim.rs.
- **swing.rs**: the following are now named constants:
  - `anim::{TOPSPIN, SLICE, LOB, VOLLEY_HIGH, VOLLEY_LOW, BODY, SMASH}` (swing motions)
  - `grade::{NONE, SWEET, CLEAN, OFF, SLOW}` and `grade::clean`
  - `NET_CLEAR`, `STRETCH`, `DIVE_CONE`/`DIVE_STRAY` (cos 22.5°/25° as the game's bits), `MAX_ERROR`,
    `STAT_MAX`, `DIVE_SWEET`
  - the timing table now uses `serve::SWEET_FRAME`
  - `FRAC_1_SQRT_2` replaces 0.70710677 (same bits)
  - branch, class and kind checks use the shot.rs modules
- **serve.rs**:
  - the lines constants replace their literals
  - `grade::clean`/`OFF`/`SLOW` replace the grade numbers
  - `class::SERVE` is passed to `launch_turned`
  - the arm-solve constants are named: `CCD_DONE` (0.1 m²), `CCD_ALIGNED` (cos ≥ 0.99999), `CCD_MAX_BEND`
    (cos 30°) and `THREE_QUARTER_PI`
  - `HALF_PI`, `SQRT_2` and `world::IDENTITY` replace the repeated bit patterns and identity matrices
- **aim.rs**:
  - `plan::*` names the plan bytes: stroke 7/8/10, volley 0xc/0xd/0xf/0x10, smash 0x11, serve toss 0/1/2,
    serve swing 3/4/6
  - `button::{CROSS, CIRCLE, TRIANGLE}`, and the `BUTTONS` table is written with them
  - `contact::{TIERED, REACH, VOLLEY, SMASH, DIVE}` names `Look::kind`/`Pair::kind`
  - also named: `OPP_SMASH`, `DEFENSIVE` (formation 2), and `THIRD`/`TWO_THIRDS`/`FIVE_SIXTHS` (the
    game's bits)
- **instant.rs**: `rally::{UNRETURNED, NONE}`, `option::{OFF, ON, LONG}`, `FIRST_CPU` (0x20), the ball
  kinds/states and `CLOSE_CALL`. The verdict check uses `judge::verdict::POINT`.
- **judge.rs**:
  - `verdict::{POINT, OUT, FAULT, DOUBLE_FAULT, LET, NET_OUT, ILLEGAL_HIT}`, used by `Rally::judge`
  - the raw ball-object offsets (`+0xa5`, `+0x228`, `+0x230`, `+0xa4`, `+0x426`) are removed from the docs
- **flow.rs**: `stage::{FADE_IN, ROLL, FLASH, SETTLE, HOLD}` names the score show's steps. The verdict range
  and chain are written with `verdict::*`. Also named: `DEFENSIVE`, and `score::DEUCE_COURT` for the court.
- **score.rs**: `FORTY`, `TIEBREAK_TO`, `TIEBREAK_ENDS`, `MAX_SETS`, and `DEUCE_COURT`/`AD_COURT` (pub).
- Fixed docs:
  - kind 4 is the drop shot, not a "drive".
  - aim.rs's `Pair::formation` doc said 2 = "both up". ai.rs and position.rs say 2 is defensive (both back),
    and flow.rs agrees (7.9 m deep), so the doc and the const now say that.

## Proof

- `tools/check.sh -p hst-sim`: 252 tests passed.
- Hash trace: baseline recorded before any edit and again after, then compared with `cmp`. Both are identical.
  - 1-game trace: 10703 lines.
  - 3-game trace: 14713 lines.
  - Command: `HST_NETPLAY=1 [HST_HASH_GAMES=3] HST_HASH_TRACE=… tools/check.sh -p hst-play hash_trace --
    --ignored`. Since P61 it is `-p hst-play`, and it needs `HST_NETPLAY=1`; with `-p hst` it runs 0 tests.
- `cargo clippy -p hst-sim --all-targets`: none of these modules has a new finding.
  - One float-negation (`-SINGLES_SIDELINE`) was rewritten as `(-F(..)).0`.
  - Errors that were already there are untouched: serve.rs test `-0.7071`, effect.rs, weather.rs.
- hst-play builds unchanged; no edits outside hst-sim.

## Not verified / not 1:1

- Behaviour: none. The change is rename-only, and the hash traces and replay fixtures are bit-identical.
- Some names are inferred from how the code uses the values, not from the original:
  - `grade::OFF`/`SLOW` (grades 3/4)
  - `plan::VOLLEY_DROP` (0x10, matching the existing "drop" comments)
  - `contact::TIERED`/`REACH` (from rally.rs's "0 tiered, 1 reach" doc)
  - the instant replay's `BALL_FAST`/`BALL_FASTER`/`BALL_SPECIAL`, and `BALL_NO_REPLAY` (states 4/5,
    meaning unknown)
  - shot.rs's `STROKE_TOP`
- Codes that hst-play reads keep their integer types with named consts, not enums: shot class/kind/branch,
  `Verdict::call`, aim plans, `Score::side`, `ShowState::stage`. Making them enums would mean editing play.rs,
  which other agents are editing at the same time (C3f's scope). Converting them is left to C3f.
- rally.rs (C3c) has its own `button` table, and its doc says "1 ○, 2 ×", the reverse of aim.rs's ✕=1, ○=2.
  It is left untouched here and flagged for C3c to check.
