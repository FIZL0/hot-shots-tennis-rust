# C3f — the match glue reads without the decompile (FINAL)

Same rules as C3: no behaviour change. This pass names the raw codes and offsets in `hst-play/src/play.rs` and the
`play/` modules that hand them to the sim.

## What changed

- **play.rs**
  - Shot codes by name: `shot::class::*` (serve/stroke/volley/smash), `shot::kind::*` (topspin, slice, lob, drop,
    underhand), `shot::branch::*` (serve/ground/volley/dive/smash) in `strike`, `serve_turn`, `advance_stroke`, the
    dive contact, `aim_target`, `ai_heard_hit`, `ai_draw_guess`, `flight_sound` and `branch_code` (its doc no
    longer quotes a struct offset). `(1..4).contains(&branch)` is `branch::rally`, `!matches!(grade, 1 | 2)` is
    `!swing::grade::clean`.
  - The AI messages use `rally::msg::*` (`HIT`, `NEW_PATH`, `CHANGE_ENDS`, `REACTION`, `SERVE`) instead of
    0x15/0x16/0xc/0x19/0xe, in the calls and the comments.
  - `whiff`'s swing motions are `swing::anim::{TOPSPIN, SLICE, LOB, SMASH}`; `button_kind` maps
    `aim::button::*` to `shot::kind::*`; `ai_aim`'s contact kinds are `aim::contact::*`; `ai_serve_kind`'s toss
    codes are `aim::plan::TOSS_*`; the strong side is `ai::strong::{MINUS, PLUS}`; the AI style checks use
    `ai::style::*` and the 50 % draw `rng::r15`; ad court is `score::AD_COURT`; the near-net check's 6.4 is
    `lines::SERVICE_LINE`; serve side angles use `params::field::SIDE` and `lines::DEG` (same literal, same bits).
  - The mover's phase number (1–4) comes from `Phase::code` (new `play/phase_code.rs`), which `locomote` and
    `bot_run` had each spelled out; `rally::phase::CHANGE_ENDS` (1) added beside its SERVE/RALLY/POINT_OVER.
  - Comments quoting the original's struct offsets (+0x1510, +0x3b9c, +0x3ec1, +0x1fc, +0x12dc) say what they are.
- **timing.rs**, **smash_marker.rs**, **match_stats.rs**: class/kind/branch names, `player::TIRED` for the mis-hit's
  stamina 10.
- **instant_replay.rs**: the pad bits are `replay::Pad`'s (no second copy), shot kinds by name,
  `instant::option::ON` and `instant::FIRST_CPU` in the point message; the `gm+…` offsets are gone from the docs.
- **tick_input.rs**: pad bits from `replay::Pad`, `SHOTS` by kind name.
- **npcs.rs**: the trigger and AI messages via `rally::msg`; `+0x274`/`+0x282` in the comments are the trigger's
  speed / its draw switch. **hand_swap.rs**, **popups.rs**: offsets out of the docs.
- **serve_check.rs** (test reading slot 3's RAM dump): the RAM layout it reads is named consts (picks, match object,
  player fields, the two timing tables) instead of bare offsets in the assert.

## Proof

- `hash_trace` before any edit and after (1 game, 3 games), files in `context/c3f/`.
- `hash_trace` before any edit and after: 10703 / 14713 lines each, `cmp` identical.
- `cargo check -p hst-play --tests`: no new warnings. The touched modules' tests pass (`serve_check` 8, including
  `serve_data_per_character` against `context/ram/s03.bin`, not skipped; `smash_marker` 2, `instant_replay` 3,
  `tick_input` 2, `match_stats` 14, `npcs` 7); the full suite runs in `done.sh`.

## Not verified / not 1:1

- Behaviour: none (rename-only; hash traces identical, see Proof).
- Motion ids (0x1e dive, 0x20–0x26 serve, 0x29 smash whiff, 0x2b hit by the ball, 0x2c–0x2f reactions, 0x30+ team
  reactions, the 0x10..0x1c IK range) and the d-pad bits (0x10/0x20/0x40/0x80) are still numbers in play.rs,
  bodyhit.rs, partner_ball.rs, foot_fx.rs, cutaway.rs, skeleton_reach.rs, emote.rs: their names
  (`motion::id::*`, `player::pad::*`) are being added by C3d, which is finished on its branch but not in main yet,
  so this branch can't use them without merging it. Follow-up C3f1 switches them once C3d is in main.
- `npcs.rs` still passes the trigger creatures their match state as 0/1/4 (start / after change of ends / else):
  C3d's `npc.rs` names those; part of C3f1.
- Left as is on purpose: the smash's own kinds (0 plain, 1 the △ smash) have no module of their own and stay
  `kind == 0`/`(kind == kind::LOB) as i32` with the comment; the `special_serve` check's characters 10/11/12 (no
  character-name consts for those in main); UI rects/texture coordinates (`match_stats`, `popups`), which are layout,
  not sim state; `replay_camera.rs`'s record kinds (presentation, C3e's area).
- `x.kind == branch::DIVE as i32` in the singles AI's net-row check: the value is 3 = dive (as before), same
  question as C31 (whether the original meant the smash there); not changed, C31 covers it.
