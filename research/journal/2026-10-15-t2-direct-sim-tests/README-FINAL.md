# T2 — direct tests for contact.rs, quat.rs, rally.rs

## What landed

- `crates/hst-oracle/tests/rotation.rs` has five oracle tests. Each compares the port with the game's own function
  bit for bit (`to_bits`). Entry points and argument layouts are in `research/oracle.txt` (T2 block).
  - `plane_sweep` (contact.rs): 40k sweeps. Branch counts: miss 29815, hit at t>0 6311, starting inside the skin
    3874. It compares the whole hit record: t, penetration, centre, contact point, normal.
  - `contact_points` (contact.rs, the contact point taken from a hit record): 40k, every branch reached
    (15921/4686/1649/2014/15730).
  - `matrix_to_quat`: 30k, all four trace/diagonal branches (17143/5748/3461/3648).
  - `quat_to_matrix`: 30k.
  - `quat_slerp`: 30k. Cases t=0 6366, t=1 5045, flipped 11188, as-is 7401. The inputs include opposite quats,
    equal quats, t = −0, a denormal t and 1−ε.
- Port fix the oracle found: `quat::slerp` now tests t for 0 and 1 the way the EE FPU reads it, so a denormal t
  counts as zero (`ps2_float::fpu::operand`). The game took the t=0 shortcut for t=1e-45 and the port didn't. The
  FPU never produces denormals in play, so replays don't change: all of hst-sim passes, including the pose and
  motion tests.
- `crates/hst-sim/tests/ai_rally.rs`:
  - The body of `replay` is now `call()`, which the new test shares.
  - New `bytes_all` writes back every field of the 0x280-byte AI block that `Rally` decodes.
  - New test `rally_state_tick_by_tick_on_match_s05` replays 10124 calls of all four CPUs in the slot-5 doubles match,
    vsync 7592..10612: receive 917, NET 3975, BASE 5232.
  - The state carries over: each call starts from the port's own block left by that CPU's previous call. Only bytes
    the game changed between the two calls (dispatcher, point flow) come from the recording. After every call the
    whole modelled block must equal the game's.
  - Each call is tied to `match_s05.bin`: its game frame is in the recording, and its score/rotation globals, its
    phase and all four player positions equal a sample of that tick bit for bit.
  - Result: all 10124 pass. The test runs in under 1 s.

## Fixtures

- `ai_rally_s05_all.bin` (2026-10-08 02:44) is the same match as `match_s05.bin`: the tie passes for all 20975 of its
  calls.
- Its NET/BASE calls (2869 of them) fail even when seeded fresh: draw counts, sub/follow/voice fields, and the exit
  record slot 0 zeroed. It was recorded before the recorder read gm-relative fields (P11k2) and before the
  partner-voice and claim-slot regions and the stack-scratch fix (B34/P11k7/P11k8). The existing tests use it for the
  receive only for the same reason.
- So the chain takes receive calls from `_all` and NET/BASE calls from `ai_net_s05.bin` (05:28, recorded after the
  fix, same match). The chain stops at the last call in `ai_net_s05.bin`.

## Not verified / not 1:1

- Slerp's VU0 micro program is hooked with the port's `quat::micro_slerp` instead of being run, because ps2-oracle
  has no VU0 micro mode. The pose sampler's PCSX2 recordings still cover it. Follow-up: T3.
- Plane sweep penetration: the original only writes hit record [1] when the sweep starts inside the skin. Its callers
  clear it first, and the test does the same (initialised to 0), so leftover values from an earlier call aren't
  tested.
- Sweep inputs use realistic w components (positions w=1, the normal w=0). Other w values aren't exercised.
- The position tie allows ±1 sample: match_s05 samples at vsync, and the AI runs mid-frame.
- The rally chain covers vsync 7592..10612 only, not the whole match. A full-match chain needs `ai_rally_s05_all.bin`
  re-recorded with the current recorder.
- `bytes_all` writes the singles-only fields as doubles leaves them (match_s05 is doubles). Mind fields other than
  net_left/net_rate/net live outside the block and aren't compared here.
