# P3e — court, sound and AI draw order

## Result

`crates/hst-sim/tests/rng.rs` `court_draws_like_the_game` replays the court generator over rng_s05's point
(reseeded at vsync 7566, through 8654; decided at 8501). It matches the game's generator at every one of the
1088 frames.

## How the test runs

- **Inputs:** the walkers' state at 7566 comes from npc_s05. The emitters and their countdowns come from
  trig_s05, the same deterministic run.
- **Ticks per frame:** read from the emitters' countdown deltas. The run has stalls (8652–8655: 0 ticks) and
  doubles (8656: 2 ticks).
- **Order in one tick:**
  1. On the decided tick only:
     - `npc::cheerers` picks who cheers.
     - The gallery applauds: `Reaction { cheer, event: Some(0), chain: false }`. The applause holds the
       rolling cheer back.
     - Every walker reacts.
  2. The walkers step. A walker that goes mode 1 → anim 5 adds a cheer mark: 0x39ca70, delay slot·3, at most 6.
  3. The gallery's manager runs (0x39b7a0):
     - its cheer (2 draws per roll);
     - its cheer marks: every 5th tick of a mark, one draw picks a jump from
       {0.05, 0.2, 0.1, −0.04, −0.21, −0.1};
     - its tick counter.
  4. The emitters step.
- **Walkers before the manager:** running the manager first fails at 8506.
- **One unexplained draw:** the point's first tick makes one draw before the walkers step. Without it the test
  fails at 7566. No ported handler makes it:
  - The walker message handler 0x39efe0 calls 0x39ece0 on 0xe when gm+0x54 ∉ {0,1}. 0x39ece0 draws only on msg 6.
  - Type 44 draws only on its reset and, in the deciding set, on its step.
  - Gap **P3e1**.

## Type 44 (0x3fad20, the deciding-set crowd)

- **The "on" flag (+0x281):** set when the deciding set is in its tiebreak: tiebreak flag (0x31661a) and sets
  played + 1 == 2·sets − 1.
- **msg 0 (creation):** the first four made take `deciding_voices`. A voice with a nonzero gap draws its call
  countdown.
- **msg 4 (reset):**
  - In the deciding set: one draw for the idle countdown. It also sets a fixed matrix and pauses the gallery's
    manager (+0x1b61 = 0).
  - Otherwise: resumes the manager, and a voiced one draws its call countdown.
  - This explains the 4 draws right after the 8655 reseed.
- **msg 2 (step):** acts only in the deciding set.
- **Ported:** `Trigger::give_voice`, `(44, 4)`, and `Near::deciding`. The matrix and the manager pause are
  **P3e4**.

## Sound

- **Not bounce sounds:** 0x3af430 and 0x3b0570 are the result/award screens' bit draws.
- **The hit bit (0x340860)** is drawn only for an unclean stroke: not a smash, not a drop, grade ∉ {1,2}. The
  app's `random_bit` now follows that.
- **Over the point,** the sound generator also makes 25/29-draw spark rerolls (0x343200) and one 20 at 7885 that
  is not explained. `effects.rs` still rerolls from its own xorshift.
- **No sound reseed at 7566.** 0x3a4030's reseed on 0xe when gm+0x54 ∉ {0,1} isn't seen in rng_s05.
- **No test for the sound counts:** checking them needs each hit's grade and kind, and no fixture has those.
  Gap **P3e2**.

## App

`play/npcs.rs` now:
- gives the type-44 voices at setup;
- sets `near.deciding` each tick;
- keeps `npc::Cheers`: added as above, stepped after the walkers, cleared at a new point and at the change of
  ends.

Still different from the game:
- The order: the gallery and emitters step in `simulate` before the walkers (**P3e3**).
- The double spawn: `emitters()` in play.rs draws on the court generator before the match's first reseed
  (`new_point`), so those draws are lost either way. The trigger resets in npcs.rs draw after it, as the game's
  first-point resets do.

AI generator draws are **P3e5**: 72/21 at each new point, 1–72 through the rally.
