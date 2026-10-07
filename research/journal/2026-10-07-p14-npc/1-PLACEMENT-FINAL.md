# P14a — background figures: roster and placement (FINAL)

## Records
- Category 21 (gallery) is in no court's plant file. Category 23 (creature) records, 11..20 per court; entry list
  `creature/npc00..npc15`, the plant idx k names npcK.
- Loader `0x332bf0`: a category-23 record with i16 at file +0x2c < 0 is linked: anchor = record (+0x2c & 0x7fff) of
  category (+0x2e + 0x11, always 0xfd → 14) in that category's array (layout + 0x490 + 4·cat, 0x90 each, file
  order); position = anchor +0x40 − file pos, x/y/z by sub.s. Stored by `0x333370`: +0x40 pos, +0x54 yaw,
  +0x58 scale, +0x5d link category, +0x5e link short. Category 14's own +0x2c is junk (0xcccd etc.), never used.
- Spawn (`0x32d4e0` case 0x17): M = rotY(+0x54) of identity (`0x125f68`), linked → × record +0x10 (identity for
  creatures, `0x125a80`); row 3 = +0x40..+0x4c. Anchor object = `0x334c70(layout, idx)` iff link category 14, else 0.
  Then `0x39b3c0(entry name, M, record, anchor)`.

## Roster (`0x39b3c0`)
Count at 0x4127e0 + 4·court (court = 0x422f90, 1-based), rows at 0x412820 + 0xc0·court, 12 bytes {name ptr, type,
constructor}; first row whose name equals the entry's (strcmp) is called with (type, M, record, anchor).
- type 54..59 (npc00..05) → `0x39ddf0` walker (0x310 bytes, vtable 0x1d1de0): made if players (0x422fa4) < 3 or
  byte 0x4139c2 + 0x12·court + 3·(type−54) ≠ 0; that row's bytes 0/1 = model `g-%02d` / animation set. Slot =
  manager (0x43b1c0) +0x800 counter (≤ 6), matrix also at manager +0x680 + 0x40·slot.
- type 0..53 → `0x3f1d50` trigger creature (0x290 bytes, vtable 0x1d2180), only with an anchor.
- 60 → `0x3a1170` (umpire chair, players ≥ 2) — P13. 61..66 → `0x3a26f0`, court 5 only (npc06..11).
- Both objects: +0x50 type byte, +0x54 record, +0x58 anchor, +0x70 matrix, +0xb0 scale, +0xb4 code.

Rosters (court: names → type): 1 trig npc06..09/11 → 0..4; 2 npc06/07/11/12 → 5..8; 3 → 10..13; 4 → 14..17;
5 wim 61..66 + npc12/14 → 18/19; 6 → 21..26; 7 → 27..30; 8 → 31..36; 9 → 37..40, 42; 10 → 43..47; 11 → 48..52, 47;
12 walkers only. Every court 1..12 has npc15 → 60.

## Proof
`hst-sim/tests/npc.rs`: RAM images `context/ram/npc_c01.bin` (PCSX2 state slot 07, court 1), `npc_c02.bin`
(slot 10, court 2), `s03.bin` (court 4), `s05.bin` (court 10), all doubles. Every creature record's position and
yaw bit-exact; the set of walker + trigger objects (record, type) equal to `npc::spawn`; every trigger matrix and
every walker matrix still at its spawn spot bit-exact (all 16 walkers in these states). Court 1: 12 figures (4
walkers, npc04/05 off in doubles), court 2: 17, court 4: 11, court 10: 17.

## App
`--stage N` places stand-in capsules (walkers grey, trigger creatures green, court 5 blue); `--singles` brings out
all six walkers. Models/animation: P14b–d.
