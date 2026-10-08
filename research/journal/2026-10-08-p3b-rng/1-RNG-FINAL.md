# P3b: the game's random sources (FINAL)

## Generators
- `0x19f5c0` is plain MT19937 (state words at +4, index at +0x9c4, mag01 at +0x9c8; standard tempering), seeded
  by `0x19f4f0` (init_genrand, index 624). `rand()` is newlib's LCG (×0x5851f42d4c957f2d + 1, bits 32..62), state
  u64 at `*(0x1b80f0)+0xa8`, 1 at boot, never reseeded.
- Four MT19937s in a match:
  - shared `*(gm+0x80)`: seeded at match setup (weather schedule, P17k), reseeded by message 0xe (new point,
    `0x324b50`) with `0x42303c = rand()` (a replay keeps the saved one). Players draw from it: mis-hit and wild aim,
    the launch's fresh +0x3b9c, shouts and shout keys (`0x3553d0`), voice bank (`0x345ad0`, b when r15 % 100 ≥ 70),
    aim nudges (`0x34c250`) and serve coins (`0x353000`), team reactions (`0x354940`), the ball launch's two
    uniforms (`0x379080`, stored at ball +0x258/+0x25c), placement, effects, and the AI reseeds.
  - AI `0x427130`: seeded 1 at overlay init; every AI object (`0x35edb0`, from the player constructor after the
    voice draw, for a CPU or a human beside a CPU partner) reseeds it with one shared draw. All AI rolls, and the
    doubles call-out (`0x3d5b20`: `0x3640b0` 25% then a bit, both AI).
  - court `*(*(*(gm+0x84)+0x154)+0x50)`: seeded 1 at construction (`0x39b280`), reseeded with `rand()` by message
    0xe unless gm+0x344. Gallery, walkers, emitters.
  - sound manager `*(0x43b1d0)+0x740`: seeded 1, reseeded with `rand()` by `0x3a4030` on message 0xc (change ends),
    6, and 0xe only when gm+0x54 ∉ {0,1}. Hit sound bit (`0x340860`), bounce sounds (`0x3af430`, `0x3b0570`).
- Draw forms: chance `(r>>16 & 0x7fff) % 100 < p`, uniform `utof(r) · 2.3283064e-10`, coin bit 16.

## Proof
`research/p3b_rng_rec.py 5 1500 context/p3b/rng_s05.bin` (lock-step, slot 5: rand state and all four generators
every frame). `crates/hst-sim/tests/rng.rs`: every frame of every generator is reached from the previous frame's
RAM state by `Mt::next` (draws shared 67, AI 1415, court 319, sound 354), or by the `Rngs` reseed: at vsync 7566
and 8656 shared ← rand #0, court ← rand #1 (`new_point`); at 8657 sound ← rand #0 of 49 that frame
(`change_ends`). The unit test checks MT19937's published outputs.

## App
`hst_sim::rng` (`Rand`, `Mt`, `Rngs`); main.rs hands the weather schedule's generator and `rand()` on to play
(`play::MatchRng`). `play.rs` draws every ported site from its generator: setup (emitters on court seeded 1, per
player voice then AI reseed, then change_ends and new_point), new points and change ends reseed, shared for
mis-hits/wild/fresh/shouts/aims/serve coins/team reactions/launch uniforms, AI for every AI roll and the call-out,
sound for the hit bit, court for spawn/gallery/emitters.

## Left (PLAN P3d–P3f)
- The original's whole draw order needs every caller: shared draws the app doesn't make (placement, effects and
  sparks — `effects.rs` has its own xorshift — framed-lob draws, the `0x37e950` launch path); court spawn drawn
  twice by the app (emitters and npcs.rs); sound bounce draws; the sound reseed on 0xe when gm+0x54 ∉ {0,1}.
- Other `rand()` callers (49 calls in the change-ends frame) and the menus' count before the match seed (main.rs
  picks a clock count).
- The voice bank's rules for two players of one character.
