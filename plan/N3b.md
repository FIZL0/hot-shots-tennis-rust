# N3b

- [ ] **N3b — Voice playback in the app.** Play a bank `(program, key)` like the game's key-on path: PS-ADPCM decode
  of the tone's `.bd` sample (loop flags), SPU ADSR envelope from ADSR1/2, pitch from `snd::pitch`, volume
  `ch_vol × expression × set[1] × tone[0xb] / 127³` and L/R from the 128-entry pan table (EE `0x1bb880`, flush
  `0x1971c8`), two voices per stereo key (`0x1b0098`). Mix at 48 kHz in the app (bevy audio or a custom source).
  Verify: decoded/enveloped output vs the SPU2 voice state in a save state (`SPU2.bin`: voice regs at
  `0x210240 + 0x108 × v`, core 1 at `0x212200`; SPU RAM at `0x10004`), volume L/R vs ring cmd 1 words.
  j: 2026-10-07-n3-audio
