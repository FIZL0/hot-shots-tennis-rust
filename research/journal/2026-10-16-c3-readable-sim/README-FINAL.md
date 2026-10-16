# C3 — readable sim, no RAM mirroring (umbrella)

C3a–C3f are all ticked (journals `2026-10-16-c3{a..f}-*`). The one loose end was C3f's follow-up "C3f1", which
never made it into the plan: the match glue still used motion numbers and d-pad bits as hex because C3d's names
weren't in main yet when C3f was written. C3d is merged now, so this task closes that gap and the umbrella.

## Changes (rename-only)

- `hst-play` glue on `motion::id::*`: play.rs (toss/serve paths, serve stance, IK stroke range
  `FORE_TOPSPIN..SOFT_FORE`, `RECEIVE` dive, `TEAM` reactions, serve-walk range, `WHIFF_SMASH` fallback),
  bodyhit.rs and foot_fx.rs (`HIT_BY_BALL`), partner_ball.rs (`RECEIVE`), skeleton_reach.rs (`SMASH`, `RECEIVE`),
  emote.rs (`MOTIONS` and its tests), cutaway.rs (`WON_GAME`, `TEAM`).
- The keyboard's d-pad bits in play.rs come from `player::pad::{UP, DOWN, LEFT, RIGHT}`.
- New `hst_sim::npc::match_phase::{START, AFTER_ENDS, PLAY}` (0/1/4) for the match state the trigger creatures
  read with a message; `Trigger::hear`/`serve_setup` and play/npcs.rs use it instead of bare numbers.

## Proof

- `hash_trace` at the base commit 428e3d4d (a detached worktree, ISO/mods/replacements/context symlinked) and after
  the edit, 1 game and `HST_HASH_GAMES=3`: `context/c3/{base,after}{,3}.txt`, 10703 / 14713 lines, `cmp` identical.
- `cargo check -p hst-play --tests`: no warnings. Tests: hst-sim `npc` (13), hst-play `emote` 10, `cutaway` 3,
  `npcs` 7, `partner_ball` 1, `foot_fx` 1; the full suite runs in `done.sh`.

## Not verified / not 1:1

- Behaviour: none (rename-only; hash traces identical).
- Left as numbers on purpose: the `special_serve` check's characters 10 and 12 (only `character::WILL` = 11 is
  named; naming the other two would need the roster order, out of this rename's scope); the trigger creatures'
  types (10, 15, 37: data type numbers, like `npc::EMITTERS`); sound volumes `0x40`/`0x80` and stick bytes in the
  tests (pad bytes, `pad::CENTRE` would read no better in a literal `Pad { rx: 0x80, .. }`).
- C31's dive/smash question (`branch::DIVE` in the singles AI's net-row check) is unchanged, as in C3f.
