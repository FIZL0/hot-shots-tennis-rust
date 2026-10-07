# P14c2 ambient sound emitters — PART (held by the user mid-task)

## Where things stand
- `hst_data::exe::Game::emitter(ty)` gives a trigger type's sound (callback row +4), mean gap (param row short +0x54) and jitter (float +0x58).
- `hst_sim::npc::EMITTERS` lists the 19 types whose callback is the plain emitter (0x3f6990): 4 7 11 12 13 16 17 23 24 25 26 35 36 41 42 45 46 50 51. None of them draw RNG in the motion reset (row +0xc, +0x44 and +0x70 are all 0).
- `npc::Emitter` ports these steps:
  - the generic tick's countdown: on reaching 0 it plays program 7 / sound id with volume 0x40, positional; sweep = 240; direction = (MT >> 16) & 1;
  - the re-arm when timer < 1: `cvt.w.s(mul(itof(base), msub(1.0, jitter, mul(2⁻³², utof(r)))))` (asm 0x3f6a94: `adda.s` 0+1, then `msub.s`);
  - type 36's ±0.375 pan sweep, wrapped to [0, 359];
  - msg 4's replay save/restore and its sweep = 0.
- Test `emitters_count_down_like_the_game` (crates/hst-sim/tests/npc.rs) is **bit-exact on courts 10 (slot 5) and 1 (slot 7)**: 11884 + 5998 emitter ticks, 13 plays. All RNG draws are consecutive among the tick's MT outputs.

## Open
1. **The start pan** (+0xd4 = `(float)*(int*)(*(0x312540)+0xc)`) usually equals `sound::place(pos).0`, but not always:
   - court 10: 3 where the bearing is 4;
   - court 2: 229/335/333/182 where the bearing is 358/134.

   It is probably the sound library's last computed angle, updated elsewhere. Find out who writes `*(0x312540)+0xc`. Until then the test copies the recording's pan on a play. Only type 36 audibly uses the pan.
2. **Court 2 (trig_s10) fails at vsync 33654**: emitter 6's timer goes 528 → 526 in one frame, with no reset seen. It could be a double tick (game catch-up) or a mid-frame read the still-filter missed. Check the neighbouring samples and the other objects' counters at that vsync. If it is a double tick, skip that pair in the test the way stills are skipped.
3. **Court 4 (trig_s03)** has not been run in the test yet. The recording ended at about 2200+ samples (26.8 MB; timeout 300); the file is truncated to whole samples by `chunks_exact`. Check that it has emitters.
4. **The 0 resets counted** means no `+0xc0` change survived the still-filter. The save/restore path (seen at vsync 8656 and 9932 on court 10: saved = timer, flag clear) has not been exercised by the test yet.
5. Then restore `for slot in [5, 7, 10, 3]` and `>= 3` in the test, and wire `Emitter` into the app's audio (spawn from `npc::spawn` Trigger kinds in `EMITTERS`, using the shared RNG). After that: journal FINAL, tick P14c2.

## Resume
- Fixtures: `context/fixtures/trig_s{05,07,10,03}.bin`. Recorded with `tools/pcsx2.sh bash -c 'timeout 300 python3 tools/record_npc.py <slot> 3000 context/fixtures/trig_sNN.bin trig'`.
- Analyse with `cargo test -q -p hst-sim --test npc emitters -- --nocapture`. The scratchpad script trig.py (object timeline) is not kept; the test prints the failing states.
- Decompile: `research/fn.sh 3f6990` (emitter), `research/fn.sh 3f2d50` (generic tick, sound block at LAB_003f3af4).
