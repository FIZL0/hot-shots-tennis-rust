# P11m: the AI's MT generator (final)

## The original

- The AI's own MT19937 lives at 0x427130: state at +4, index at +0x9c4, 0x9c8 bytes.
- Seeding (0x19f4f0) is the reference init: state[0] = seed, state[k] = k + (s ^ s>>30)·0x6c078965.
- The generator (0x19f5c0) is reference MT19937, with tempering.
- The static init (0x422390) seeds it with 1.
- Each AI object's constructor (0x35edb0) reseeds it with a draw from the main generator (gm+0x80).
- Only the AI draws from it. The 24 functions that call 0x19f5c0(0x427130) are all 0x36xxxx/0x3cxxxx/0x3dxxxx AI code (zone blur, chance, Mind, set-state/sub, aims, timing).

## Port

- `hst_sim::mt::Mt` (`new(seed)`, `next()`); `reference_outputs` checks MT19937's published outputs for seed 5489.
- The app's `Game::ai_mt` takes every AI draw: `ai_roll`, the timing/picks/guess, the right-guess draw, the run-round, formation and singles position rolls, the aims and kind locks, the Mind updates, and the dispatcher (`doubles_ai`).
- The NPC, gallery, sound, reaction-motion and AI call-out voice draws stay on the app's xorshift `rng`.

## Check

- Bot singles and doubles each ran 90 s (`HST_AUTOPLAY=1 … --play [--singles]`) with no panics: 8 and 5 points.

## Gaps → P11m1

- The seed: the game draws it from its main generator each time an AI object is made. The app seeds once with a fixed value, because the main generator isn't ported.
- Which generator the AI call-out voice (`sound::call_out`) draws from isn't checked yet.
