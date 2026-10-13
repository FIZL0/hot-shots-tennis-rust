# 1 — the p2m2 holds only P1's pad (READY: needs the human)

## What was found
- `context/recordings/4playergame.p2m2`: 35175 frames, 36 bytes each = two 18-byte pads (PCSX2's input recording
  format stores ports 1A and 2A only; it has no multitap slots). Port 2A is all zeros (nothing connected); only
  port 1A has input (18274 non-idle frames).
- The save state's `PAD.bin` shows the setup: DualShock 2 pads on 1A, 1B, 1C, 1D (a multitap on port 1), port 2
  empty. Its RAM: all four players' AI pointers (player +0x80) are 0, so all four are humans; slots
  `06/09, 07/05, 02/04, 08/09` (char/outfit), vsync 12580, score 0-0.
- So players 2–4's inputs (multitap 1B–1D) were never written to the file. Replaying it (PCSX2 playback or
  `tools/play_p2m2.py`) drives P1 only; the other three stand idle and the match is a different one from the first
  point on. The user's match can't be reproduced from it, so neither part of P27 can be done with it.
- Where the game reads its pads: the per-player input update copies 18 bytes from
  `0x2efb30 + port·0x48 + slot·0x12` (decoded state of each port/multitap slot, 8 entries), plus the pad's word
  `0x2efb10[port·4 + slot]`; raw packets are 0x140-byte blocks at `0x2efc00 + (port·4 + slot)·0x140`.
  `record_p2m2.py`'s PAD region (0x2efb00, 0x90) already covers port 1's four slots (0x2efb30..0x2efb77), so a
  capture made with it while four people play holds all four players' inputs every frame.

## What unblocks it (human)
Play a new 4-human doubles match (multitap on port 1, as before) while `tools/record_p2m2.py` captures it:
save a state at the first serve to a free slot N, then `tools/record_p2m2.py N context/fixtures/4playergame.bin 60000`
(it loads slot N itself and records until the match object is gone = match over), and play. At 1x some frames may be
skipped (logged as `missed frame`); if many are, slow PCSX2 to NominalScalar 0.5. Keep the state as
`context/recordings/4playergame2_SaveState.p2s`. With that capture an agent can build the replay test
(all four pads from the samples) and the side-by-side checks.

## Not verified / not 1:1
- Everything in P27: no fixture can be made from this recording (reason above). Nothing ported, no fixture retired.
- The multitap slot → decoded entry mapping is from the decompile (the input update above) and the save state's
  idle entries; not yet seen with live multitap input.
