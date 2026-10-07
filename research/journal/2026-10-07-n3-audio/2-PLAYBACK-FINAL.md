# N3b — voice playback (FINAL)

Result: `snd::Voice` steps one SPU2 voice like PCSX2 2.9's mixer, one step per 48 kHz sample. Replaying 12 sounding voices of save-state slots 3/4/5 from key-on gives identical loop start, next address, envelope phase/level/counter, ADPCM history, sample position, the 32-sample decode queue, loop flags and the last enveloped output sample (OutX). `snd::Level` gives the driver's L/R volume registers bit-exact for 13 voices, and the tone part (tone level, tone pan, centre flag, gains) is derived from the bank tone bit-exact. The steps since key-on are `(read·4096 + sp) / pitch`; voices where that does not divide exactly (pitch changed) are skipped (1 voice). `hst/src/audio.rs` is the 48 kHz mixer (bevy `Decodable` stream, `Sound::play(bank, program, key, level)`, `--sound <xb> <hd> <program> <key>`).

## Fixture
- `research/tools/fixture_spu.py` now adds `spu` rows (PCSX2 2.9.52 `V_Voice` in `SPU2.bin`, size 0x108: voice base core 0 `0x2101f0 + 0x108·v`, core 1 `0x2121b0 + 0x108·(v−24)`; volume L Reg_VOL @0 / R @12; ADSR reg32 @24 (ADSR1 low), Counter @68, Value @72, Phase u8 @76; Pitch @80, LoopStartA @84, StartA @88, NextA @92, Prev1 @96, Prev2 @100, LoopFlags @107, SP @108, OutX @112, DecodeFifo s32[32] @128, DecPosWrite @256, DecPosRead @260; addresses in SPU halfwords) and `level` rows (EE voice table `0x30e1c0`, 0xec per voice).

## SPU2 voice step (per 48 kHz sample)
- Read the current block's flags (block header high byte); LOOP_START (4) sets loop start = NextA & ~7.
- If write − read ≤ 12: decode the block if not cached (Prev1/Prev2 carry over), push 4 samples `((NextA%8)−1)·4..` into the queue, write += 4, NextA++; at a block end (NextA%8 == 0): LOOP_END (1) → NextA = loop start, and stop unless LOOP (2); NextA++, drop the cache.
- Then out = Σ (gauss[(SP & 0xff0)>>4][i] · queue[read+i]) >> 15 (each tap shifted), ADSR step, out · level >> 15, L/R = out · (s16)(reg<<1) >> 15. SP += min(pitch, 0x3fff); read += SP >> 12; SP &= 0xfff.
- Key-on: phase 1, level 0, NextA = start | 1, everything else 0.

## ADPCM
- 16-byte block, header byte 0 = shift (low nibble) | filter (high nibble), filters {0,0},{60,0},{115,−52},{98,−55},{122,−60} (others 0,0); 28 nibbles low first: `(nib<<28 >> (shift+16)) + ((f1·prev1 + f2·prev2 + 32) >> 6)` clamped to s16.

## ADSR
- Attack (inc, exp = ADSR1 bit 15, shift bits 10–14, step 7 − bits 8–9, target 0x7fff), decay (dec, exp, shift bits 4–7, step −8, target (bits 0–3 + 1) << 11), sustain (dec if ADSR2 bit 14, exp bit 15, shift bits 8–12, step 7 − bits 6–7 inverted (`!`) when decreasing; ends when level hits 0), release (dec, exp bit 5, shift bits 0–4, step −8, target 0).
- Per step: counter += max(1, 0x8000 >> max(0, shift−11)) (÷4 if exp increasing above 0x6000); level_inc = step << max(0, 11−shift), exp decreasing → (s16)(level_inc · level >> 15); counter ≥ 0x8000 → counter 0, level += inc clamped 0..0x7fff.

## Gaussian table
- Generated (nocash/Near formula, as PCSX2): 512 points s·(t+u+1)/k, scaled to 0x7f80·128 total, per phase the four taps corrected to sum 0x7f80, rounded. Matches PCSX2's table; test asserts rows 0, 128, 255.

## Driver volume (EE flush `0x1971c8`)
- Pan index = 0x40 if the tone's centre flag, else clamp(play pan + channel pan + tone pan − 0x80, 1, 127); pan table `0x1bb880` (128 × u16, high byte L, low byte R, 0x78 each at 64).
- Per side: x = seq_vol · tone · velocity / 0x3f01 · bank / 127 · pan byte; centre tones × gain / 0x7fff; max 0x3fff.
- Tone level = ch_vol · expression · set volume · tone[0xb] / 127³ with the channel's reset values volume 100, expression 127 (channel reset loop in the sequence init, `piVar[5] = 100`, `piVar[8] = 0x7f`, stride 0x70).
- Tone pan = tone[0xc] clamped 1..127 (0x40 if centre); centre gains from the cos table `0x1bb988` (129 × u16, 32767·cos): L = cos[pan], R = cos[0x80 − pan] (23169 at 64).
- Both tables are read from `SCUS_976.10` by `exe::sound_tables`. Checked by hand: voice 47 → 7800, voice 25 → 1187/424.

## SE sequence player
- 8 ticks a frame (480 / 60); an event fires ⌈delta/8⌉ frames after the previous one, the first on the first frame. Running status.
- a0 note velocity set (velocity 0 = key-off of that note), `f0 00 20 x` marker (3 bytes), `f0 00 30` (4 bytes, effect unknown), other f0 to f7, b0 two bytes, ff 2f end; other status nibbles take no data.
- 8120 sequences on the disc parse; 14 failures are empty program/key slots that point at offset 0.
- Pitch word for a key-on: root<<24 | note<<16 | fine<<8 | 0x40 with scale word 0x01001000 (SE default seen in the ring).

## App
- One bevy audio stream; each 800-sample frame advances the playing sequences then mixes voices (sum, clamp s16). Test `audio::tests::plays_a_system_sound` renders sys_se00 (0,0) to a 0.43 peak and the voice ends.

## Left for N3c+
Stereo keys are two play calls (`0x1b0098`: two sequences with positional sequence volume L/R, e.g. 106/43 and 43/106); the driver's 48-voice allocation by priority; the velocity curve (EE `0x2f1920`, voices so far had velocity 127/100 raw); core/master volume and reverb; sequencer ADSR overrides at key-on (`0x18f950`).
