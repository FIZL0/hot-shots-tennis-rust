# P3e3 — court draw order in the app

## Result

`play/npcs.rs` `step` now makes every court-generator draw of a tick, in the order `court_draws_like_the_game`
(rng.rs) proves for the game:

1. A decided point (the umpire turning): `npc::cheerers`, then the gallery's applause (`Gallery::point`, which
   `simulate` now leaves in `Game::applause` instead of drawing at the verdict), then every walker reacts.
2. The walkers step, adding cheer marks.
3. The gallery's manager: its cheer (`Gallery::step`, moved out of `simulate`), its marks, its tick.
4. The trigger creatures and the sound emitters, interleaved in spawn order (`Npcs::figures`; the emitters
   are still `Game::emitters`, built by play.rs `emitters()` from the same `npc::spawn` list).

Before, `simulate` stepped the gallery and emitters at the top of the tick, before the verdict and before the
walkers, so the applause reached them a tick late and the cheerers drew after the applause.

`step` now runs before `play_sounds`, so the gallery's and emitters' sounds still play in the tick they are cued.
Without the court's figures (no `HOLE01.XB`), `step` still applauds, steps the gallery and the emitters.

## Checked

- `tools/check.sh`: 246 tests pass (rng.rs `court_draws_like_the_game` is the order's reference).
- `HST_AUTOPLAY=1 hst <iso> --play --stage 10` and `--stage 4` for 90 s: 7 and 8 points decided, no panic.

## Left

- Assumed: the game steps its emitter and modelled trigger objects in one list, in spawn order. rng_s05's court
  has only emitters, so the interleaving with modelled creatures is not recorded.
- The app's verdict tick vs the game's (the ball's step deciding the point) is not checked here.
