# N3a

- [x] **N3a — Banks, note → tone, SPU pitch.** `hst_data::snd::Bank`: `(program, key)` → sequence (offsets from the
  sequences section start, `0x19a008`), key-ons `a0|ch note vel set` + varlen delta, tone = set + 8 +
  (note − lowest) × 16 (`0x19a0d0`). The resolution is EE-side (SDPR2 sequencer, event table `0x1bbd00`); the IOP
  driver `SG2IOPM1.IRX` is only an SPU2 register remote fed by the EE command ring (`0x305000`). `snd::pitch` ports
  the driver's pitch command (root/note/fine/bend → VP via its 608-entry table, `exe::pitch_table` from the disc).
  t: hst-data sound.rs (63 key-ons from 3 save states resolve to the exact tone: ADSR, SPU sample address, note,
  root, fine; 105 voice pitch registers bit-exact). j: 2026-10-07-n3-audio/1-BANKS-PITCH-FINAL.md
