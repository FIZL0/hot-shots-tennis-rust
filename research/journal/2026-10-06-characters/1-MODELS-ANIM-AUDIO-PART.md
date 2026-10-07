# Characters: models, motions (done), audio (PART)

## Models (`PC/PCnnCcc.XB`, `pcNN_tTT_cCC.MDL/MTL/MTI/NOI`, `Rkt/pcNN/racket_NN`)
- TT = body type (TParam モデルタイプ). Node names in the node header extra blob: 3ds Max Biped (`Bip01…`, 53),
  helpers (`s_body`, `face`, `Bone000n` hair/pony chains), `Racket` (racket attach).
- Node matrices: +0x00 bind (node → model, row vectors), +0x40 local, +0x80 inverse bind.
- Batch header +0 = bone count N (1 rigid … 6), +4.. = N node indices (palette).
- Blended packets: the "bone list" (+0x3c) is drawn vertex → first position entry; each entry is a bone-space
  position PRE-MULTIPLIED by its weight, w = weight; vertex = Σ [p, w]·bind(bone). Normal `w` bits 3..5 = palette
  slot, bit 13 = more entries, bit 15 = no-kick. Copies coincide in the bind pose (checked).
- Character texture alpha is NOT coverage (smooth 0..253 masks) → draw opaque. Depth: far camera (40 m) needs a
  far near plane or layered cloth z-fights.

## Motions (`PCANI/PCnnANI.XB`, `.ANI2`; also `.MOR` morph and `.UVA` UV anims, not used yet)
- Format in `hst_data::ani` (80 ticks/frame, rot/pos/scale keys per Biped track). Rotation keys are the conjugate
  of the local rotation (Bevy: (-x,-y,-z,w)).
- Motion table at 0x41db80 (program data) = motion numbers used by the code (0x10.. strokes, 0x20.. serve,
  0x27.. whiffs). Contact pose = motion frame 8 (serve 3522b0: speed 8/frames).

## Audio (PART)
- Banks: Sony-style `SShd` (.hd) + raw PS-ADPCM (.bd). Loader 199df0: header +0x10..+0x24 section offsets
  (-1 = none). Driver slots (DAT_00312540+0x128): 0 court SE (co_seNN), 1–4 player voices (dvv_vc/sgv_vc per
  character), 5 umpire (gag_vc), 6 gallery, 8 jingle, 9 sys_se00. Play call 19fa70(slot, program, key, pos, …);
  hit sound = (0, 6, 3..8) by grade/type (340860), others (9, 0, 4/7).
- +0x1c section = programs: u16 max index, u16 offsets; program = u16 max, u16 offsets → event sequences
  (`a0 note vel`, `00 80 00`, `ff 2f 00` end) per key.
- +0x24 = tones, 16 bytes from section+2: u16 sample address /8 (verified: 109 of 111 land on BD sample starts),
  ADSR/volume/pan/root note/fine tune after.
- UNRESOLVED: note → tone (section +0x14 key map + +0x20 regions; identity fits only 60%). The resolution is in the
  IOP sound driver (IRX) — decompile it (R3000) next. BD samples: end flag (byte 1 bit 0) then one silent block.
- RESOLVED in 2026-10-07-n3-audio/1-BANKS-PITCH-FINAL.md (EE-side, 4-byte a0 key-on with a tone-set index).
