# P3e4 — gallery manager leftovers (port: `npc.rs` `Cheers`/type 44, `play/npcs.rs`)

## Manager (*0x43b1c0)
- +0x1b61 run flag, +0x8c4 kind (clamp 0..1), +0x8c8 count; slots from +0x8d0 stride 0x30 (pos, base, delay, n);
  room for 6 + 93 (to +0x1b60). Court 5 reads its marks from +0x9f0 = slot 6.
- Step 0x39b7a0: nothing when +0x1b61 == 0 or court weather (+0x135) ∈ {2, 3}; then +0x85c/+0x860 = −1
  (event dropped). Else 0x39dbd0 (`sound::Gallery::step`), the marks' jumps (kind < 2: always), tick +0x1b74++.
- Register 0x39ca70: writes slot[count] even when full, kind = clamp(param) (walkers pass 0), count++ only < 6.
- Court 5 init 0x39cc50: 93 points from GAME.BIN 0x413270 (16 bytes, w 1; first (15.5, −1.3, 3.6)) to slots
  6..98, delay 3, n 0; kind 1, count 0x5d.
- Messages 0x39c440: 6 count 0, players > 1 → run + court 5 init; 0xc/0xe count 0; 0x17 cheerers (0x39c8c0),
  court 5 init if call +0x426 == 0 and (0x4230b8 > 0 or game/set or point kind +0x560 ∈ {1, 2}); 0x18 count 0;
  0x1a (players > 1) court 5 init if the favoured team won, run.

## Type 44 (0x3fad20)
- msg 4: world (+0x1b0) = home (+0x70). Deciding (tiebreak and sets played + 1 == 2·sets − 1, not replay +0x344):
  idle countdown, manager +0x1b61 = 0, world = identity (RAM 0x1cc360, checked in context/ram/s05.bin) with
  row 2 = normalize(−home.x, 0, −home.z), row 0 = row 1 × row 2, home position — the same ops in the same order as
  `npc::walker_facing(home, 0)` (mula z·z, madd x·x, sqrt, div; mula/msub cross). Else +0x1b61 = 1, voiced → call
  countdown.
- msg 8: pause if deciding, else resume.

## Sprites 0x39ba30 (not ported, P3e6)
Runs, not raining, (court 5 or 0x2eefb0 == 0); billboard per mark, texture *0x43b1d0 + 0x6a4 (+0x678[kind]),
sizes 0x4131e0 stride 0x14, occlusion alpha via 0x369530, fov 0x1e7d50.

## Gaps
P3e6 sprites; P3e7 msg 6/0x1a/0x17 conditions and msg 8. Nothing recorded: no court-5 or deciding-tiebreak
recording exists.
