# P3e2: the sound generator's draws

The sound manager's MT19937 is at `*(0x43b1d0)+0x740`.

## Draw sites in a match

- **Stroke bit**, 0x340860 (already ported, play.rs): one bit on the hit, unless the hitter's stroke is clean.
  - It reads the effects object (0x423f80): +0xd0 shot kind, and per player p +0xd8+8p offset, +0xdc+8p grade, +0xdd+8p branch.
  - The bit is drawn when branch != 4, kind != 4 and grade ∉ {1, 2}.
  - +0xd4 holds one byte per player: the stroke flag, set for one frame when that player's record is written, a few frames before contact. It is set even when the values don't change.
- **Hit-spark reroll**, 0x343200(obj, mode). It draws from the sound MT (gm+0x80 only when 0x43b1d0 is null).
  - Mode 0, when a burst has died out: one draw, start = (r>>16)&3, then slots start, start+4, … < 25. That is 29 or 25 draws.
  - Mode 1: all 25 slots, 100 draws.
  - Each slot takes four uniforms: angle = unit·6.2831855, turned by 0x125e18 (sceVu0RotMatrixZ) applied to identity 0x1cc360, then the rolls +0x40/+0x44/+0x48.
  - The game's sincos keeps sin ≥ 0 for θ ≥ 0, so the turn is in [0, π) (P9c's observation).
- **Spark message handler**, 0x342090:
  - Msgs 0xc and 6 do a mode-1 reroll, then 342970(obj, 0, 0), which clears +0x50 (the burst stops).
  - Msg 0xe does the same, but only when gm+0x54 (the old phase) ∉ {0, 1}.
- **Sound manager**, 0x3a4030: reseeds from rand() on msgs 0xc and 6, and on 0xe with the same phase gate.
  - The phase machine is 0x323990: gm+0x54 old, +0x55 new.
  - Entering phase 2 sends 0xe. Phase 2 is entered from 4 (after a point: reseed) or from 1 (after the change of ends: none).
- **The 20 draws at 7885 were a dive ring.** Poke-and-diff showed the run object (vtable 0x1d1cf0, 0x3993f0) draws 10 puffs × 2 uniforms from `*(0x43b1d0+0x740)`, the sound MT.
  - P17p called it the shared MT. Its recorder had always read the sound one, so the foot test passed either way.
  - The app's foot_fx now draws from `rng.sound`, and the doc comments, record_foot.py and the P17p journal are corrected.
- **Result/award bits**, 0x3af430 (match end 0x3ac090) and 0x3b0570 (result object 0x3a9f60): one sound bit each, which picks the crowd sound 0x1a02a0(1, lose, bit, 0x80, 1) along with the umpire's call 0x39da50.
  - Both belong to the winners' ceremony, which the port skips (P26c). The next match reseeds the generator anyway.
  - Recorded as a gap on P26c.

## Order at a new point after a point

In the reseed frame the sparks reroll all 25 slots from the old generator, and those draws are lost. rand() then gives shared, court, then sound.

- rng_s05's 8655 rand outputs are #0, #1 and #2 = 0xf0b2f18. #2 is the sound seed, mt[0] at 8657.
- The 8656 sample is torn between the reseeds, which is why P3b read the sound seed as "rand #0".

## Port

- hst-sim effect.rs:
  - `Roll::new` uses `shot::rot_z` through `world::mat_mul(IDENTITY, ·)`.
  - `Roll::draw` and `Roll::reroll(rolls, all, mt)` are the reroll.
  - `Sparks::tick(&mut Mt)` rerolls a quarter of the table when the burst dies out; `Sparks::restart(&mut Mt)` rerolls all of it and stops the burst.
- effects.rs HitSparks: the xorshift is gone. `frame(hit, &mut rng.sound, …)` and `restart`.
- play.rs:
  - `reseed_sound(g)` stashes the old generator in `Game::respark`, then calls `rng.change_ends()`.
  - It is called at setup, at Next::ChangeEnds, and at the end of `next_point(fresh)`, after `new_point`.
  - `start_effects` restarts the sparks from `respark`.
  - MatchOver goes through `next_point(true)` (msg 6 in the game).

## Proof

`research/p3e2_sound_rec.py 5 1600 context/p3e2/sound_s05.bin`, run under `HST_LOCKSTEP=1 tools/pcsx2.sh`. It records, per frame: vsync, the effects object 0x100, the sparks object 0x70, 25 particles ×0x40, 25 rolls ×0x50, and the sound MT 0x9d0, covering vsync 7533–9133.

`crates/hst-sim/tests/rng.rs` `sound_draws_like_the_game` replays the 1599 frames:
- The stroke flags name the hitter, and a dive throws a 20-draw ring.
- Burst starts (from the particles) are the hits: the stroke bit is drawn if the hit is unclean.
- A burst dying out rerolls.
- The new point after the played point (the shared reseed in rng_s05, 8656) restarts the sparks and reseeds the sound generator. The first new point (7566) follows the change of ends the save state sits in, so it touches neither.

Every frame, the generator equals the game's word for word, and the 25 rolls (matrix and uniforms) and the burst flag are bit-exact. The run covers 1 ring, 6 stroke bits, 14 burst ends and 1 reseed.

Mutations both fail at 8656: dropping `new_point` before the sound reseed, and dropping the sparks' restart.

`sparks_s05` (no sound MT in its fixture) still takes the rolls from the recording after each tick.

`research/p3e2_poke.py <v> <n> <slot> [poke]` is the lock-step poke-and-diff used to find the ring's consumer.
