# N3a: sound banks, note → tone, SPU pitch (FINAL)

Corrects 2026-10-06-characters/1-MODELS-ANIM-AUDIO-PART.md (Audio): the key-on event is 4 bytes and note → tone is resolved on the EE, not in the IOP driver.

## The IOP driver is a register remote
- `MODULES2/SG2IOPM1.IRX` ("sg2iop_driver", R3000, no symbols). `research/mkirx_elf.py` turns it into a plain MIPS ELF with function-start symbols; Ghidra headless (`-processor MIPS:LE:32:default`) decompiles 86 functions.
- The EE library (SDPR2) writes commands into a ring at EE 0x305000: 1024 × {u32 cmd, u32 voice, u32 word8, u32 wordC}, write index at 0x304fc0 (written by 1915c0). It keeps history, so a save state shows the last ~1000 commands. cmd 1 volume L/R, 2 ADSR (adsr1, adsr2), 3 sample SPU address (bytes), 4 pitch, 7/8/9 key masks, 11/13–16 per-core setup, 19 SPU DMA upload (addr, size ≤ 0x10000).

## Sequencer (EE)
- Event handlers: table 0x1bbd00 of (fn, length) by status nibble: 80 3, 90 3 (18ed00), a0 4 (18f950, key-on with tone set), b0 3, c0 2 (190d98, program change, header section +0x10), d0 2, e0 3, f0 variable (190ed0). Each event is followed by a varlen delta (`80 00` = 0). End `ff 2f 00`.
- (program, key) → sequence (19a008): section = hd + [0x1c]; program p = section + u16[section+2+2p]; sequence = section + u16[p+2+2key] — offset from the *section* start. Bounds: program ≤ u16[section], key ≤ u16[p].
- Tone (19a0d0): set = [0x24] section + u16[sec+2+2i] (i ≤ u16[sec]); 8-byte header (+1 volume, +4 default pan?, +5 alternate, +6 lowest note, +7 highest, unchecked); tone = set + 8 + (note − lowest) × 16.
- Tone 16 bytes: +0 exclusive group (196db0 if nonzero), +1 priority, +2 root, +3 fine (i8), +4 u16 sample address/8, +6 ADSR1, +8 ADSR2, +a centre-pan flag (pan 0x40), +b volume, +c pan, +d/+e alternates, +f flags (1, 2 no auto key-off?, 0x10..0x80).
- Volume ch_vol × expr × set[1] × tone[0xb] / 127³ (1161d8, 116f78). Voice flush 1971c8: pan table 128 × u16 at 0x1bb880 (hi L, lo R).
- Pitch command word8 = root<<24 | (note+transpose+bend semis)<<16 | (fine+fine bend)<<8 | bend (0x40); wordC = range<<24 | scale (0x1000 normal).
- Validated: every tone of every SE/voice bank reached by a sequence lands on a BD sample start. Gallery banks' program 9 offset table is garbage the game never requests.

## Pitch (IRX)
- note = b2, root = b3, fine = (i8) b1, bend = b0 of word8; range = b3, scale = low 16 of wordC; B = ((bend−0x40)·range)>>2.
- root > note: d = root−note, p = T[(12 − d%12)·16 + B + 0xd0 + fine] >> (d/12 + 1); else d = note−root, p = T[(d%12)·16 + B + 0xd0 + fine] << (d/12).
- VP = (scale · (p·441/480)) >> 12. Root note at scale 0x1000 → 3763 (44.1 kHz).
- T: 608 u16 in the IRX .rodata at file offset 0x4880, 192 steps per octave, T[0xd0] = 4096. It has anomalies (T[64] = 2360, T[172] = 3340) and no formula reproduces it (floor off in 22 places), so `exe::pitch_table` reads it from the disc (checked: size 28093, "sg2iop_driver").

## Proof
- `research/tools/fixture_spu.py <state.p2s> context/fixtures/spu_s0N.csv` (slots 3, 4, 5): loaded banks (EE "SShd" copies matched to the disc, .bd found in SPU RAM → base), complete cmd 2→3→4 key-ons after the last upload, and each voice's last pitch command next to its SPU2 pitch register. PCSX2 `SPU2.bin`: SPU RAM at +0x10004, voice pitch u16 at 0x210240 + 0x108·v (core 0) and 0x212200 + 0x108·(v−24) (core 1).
- Slot-5 banks: sys_se00 @0x1e0000, co_se10 @0x7f1c0, galsg10a @0x105640, umpire gag_vc04a, jingle jig_00, bgmg_10, four dvv_vc voice banks. Key-ons before the last upload belong to menu voice banks no longer loaded (mvc_cs0), hence the window.
- Test: hst-data `sound.rs` — 63 key-ons resolve through `Bank::key_ons` + `Bank::tone` to the exact ADSR, SPU sample address, note, root, fine; 105 pitch registers equal `snd::pitch` bit-exact.
- Key-ons with cmd 3 before cmd 2 come from another path (BGM sequencer) and are left to N3e.

## Next
N3b playback (ADPCM decode, ADSR, volume/pan), then N3c triggers.
