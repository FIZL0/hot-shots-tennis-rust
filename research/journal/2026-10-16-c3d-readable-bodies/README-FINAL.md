# C3d — the bodies read without the decompile (FINAL)

Same rules as C3: no behaviour change. This pass names the raw codes, bytes and magic constants in
`hst-sim/src/{player,motion,pose,foot,face,position,stats,npc,umpire}.rs`.

## What changed

- **motion.rs**
  - `motion::id` names every motion number the sim sets or tests: stances/runs (`STAND`, `TURN_FORE`/`BACK`,
    `RUN_*`, `DASH`, `TIRED`), strokes (`FORE_TOPSPIN`, `GROUND_STROKES`, `STROKES`, `SOFT_FORE`/`BACK`,
    `RECEIVE`, `SMASH`), the serve (`SERVE_STANCE`, `SERVE_LEFT`/`RIGHT`, `TOSS`, `TOSS_UNDER`, `SERVE`,
    `SERVE_UNDER`), misses (`WHIFF_*`) and reactions (`HIT_BY_BALL`, `WON`, `LOST`, `WON_GAME`, `LOST_GAME`,
    `TEAM`).
  - Stroke starts, whiffs, recoveries, serve walk/toss/swing and reactions use them plus `shot::branch`/`kind`;
    lock and recovery frame counts are named (`SMASH_LOCK`, `LOCK`, `QUICK_RECOVERY`, `RECOVERY`); the team
    reactions name their characters (`player::character::{JJ, BIG_CHIEF, WILL}`).
- **player.rs**
  - `player::character` (JJ, Big Chief, Will, Suzuki: the menu name sheet's numbers), `player::pad` (stick
    centre, deadzone, d-pad bits, `axis`).
  - Weather via `weather::RAIN..=HEAVY_RAIN`, phases via `rally::phase`, stamina via `shot::branch`; the court
    bounds (`SIDE`, `BACK`, `lines::SERVICE_LINE`), run speeds, turn/run-motion angles (std `PI`/`TAU`/
    `FRAC_PI_4` or named decimals), stick and targeting constants are named; run/stand motions return `id::*`.
- **stats.rs**: `MAX_FRAMES` (99:59:59 at 60 Hz), `MAX_COUNT`, `SERVE_SCALE`, `NET_ZONE` (service line), `MPH`;
  branches and verdicts by name.
- **umpire.rs**: `RIGHT`/`LEFT` beside `IDLE`, `program::*`, `word::*` (the call words), `announce::*`, a private
  `part` for the score call's halves; `NET_ZONE`, `HEAD_LOW`, `BACK_TICKS`, `BACK_STEP`; court 5 as
  `npc::STADIUM`.
- **npc.rs**: `STADIUM`; layout categories via `layout::category::{PATH_MARKER, CREATURE}`; the roster's kind
  ranges (`roster::*`); walker `anim::*` and `mode::*`; trigger messages `call::*` and the match messages it acts
  on (`CHANGE_ENDS`/`SERVE` from `rally::msg`, `RESET` 0x1a); draws via `rng::r15`/`coin`/`TWO_POW_M32`; walker
  constants (`WALKER_R`, court half sizes from `lines`, `DODGE_*`, `FREEZE_*`, `TURN_TICKS`, `MIN_HIT_T`,
  `SLIDE_*`, `DEG`). Doc offsets (+0xc4 etc.) gone.
- **position.rs**: `ai::formation::STAGGERED`, `ai::style::{NET, BASE}`, `rng::r15`/`TWO_POW_M32`, the court
  sizes from `lines`.
- **foot.rs**: `character::SUZUKI`, `id::{LOST_GAME, WON, RECEIVE}`, `LATE_SUB`; the from_bits floats named
  (`PUFF_Y`, `PRINT_Y`, `AHEAD`, `ACROSS`, `TWO_POW_M31`, `GRAVITY`, `DEG`, `TWO_PI`, `TWO_POW_M32`).
- **pose.rs**: `TINY_ANGLE`, `THREE_QUARTERS_PI`, `NEAR2`, `ALIGNED`, `COS_30`; motion numbers in docs by name.
- **face.rs**: doc only (motion names instead of hex).
- Every replaced `f32::from_bits` is pinned by a `constant_bits` test (player.rs, foot.rs, pose.rs).

## Proof

- Hash trace recorded before any edit and after the last one, `cmp`: identical (1 game 10703 lines, 3 games 14713
  lines), files in `context/c3d/` (`base*.txt` vs `after*_b.txt`).
- `tools/check.sh -p hst-sim`: 259 tests passed (incl. the new `constant_bits` tests).
- `done.sh`'s float-lint gate (`cargo clippy -p hst-sim --lib -- -A clippy::all`) clean; no new clippy findings in
  these files (the remaining are the old negated-compare / too-many-arguments style ones).

## Not verified / not 1:1

- Behaviour: none (rename-only; hash traces and tests bit-identical).
- Names inferred from use, not from the original: the character numbers (from the menu name sheet order), the
  motion names in `motion::id` (from the ANI2 motion name table and where the code sets them), walker `anim::*`
  (by what each does on screen), `call::*` and `RESET` (0x1a: the match message that resets without the serve
  set-up's keeping), `STADIUM` (court 5's own crowds), `LATE_SUB`, `GRAVITY` (value 9.8/3600 to f32, meaning
  guessed), the angle names in player.rs.
- Left as is: trigger type numbers in `npc.rs`'s `callback` (54 types, each named only by its behaviour comment;
  a `ty` module would be a long list of guesses), message 0 in `callback` (never sent by the port), `self.msg == 6`
  in `Trigger::reset` (unknown match message), `roll() >> 16 & 3` draws (two bits, no helper in `rng`).
- Left as integer codes with named consts, not enums (serialized in snapshots, read by hst-play): motion ids,
  walker anim/mode, umpire program/word, formation bytes.
- hst-play's `npcs.rs` still passes raw 6/7/0xe/0xc to `Trigger::message`/`hear`; play.rs untouched (other agents
  edit it): C3f can switch them to `npc::call::*` / `rally::msg::*`.
