# N3d

- [ ] **N3d — Player voices, umpire, gallery.** Per-player voice banks (slots 1–4, `snd/VOICE/PC/PcNNvceK.xb`),
  umpire calls (slot 5, `snd/UMP/uvNNx.xb`), gallery/crowd (slot 6, court bank). Find which program/key each event
  plays (scores, shouts, applause) and when; verify against a ring recording. j: 2026-10-07-n3-audio

From N3c6: `0x3554c0` plays a character voice (bank slot = player +0x12b8 + 1, at +0x3d70, 0x80); `0x3553d0` picks a
random key in a range other than the last one (player +0x3b64 slots) — a dive shouts program 3 keys 0..2 (0..1 in
doubles) at its start; `0x3552c0` (2 players, +0x3b98 == 0) shouts per stroke: serve/smash program 0 key +0x3b9c % 3,
other strokes program 1 key % 6, none for dives. Other callers: `0x3467b0`, `0x355350`.

Split: [x] N3d1 stroke and dive shouts · [ ] N3d2 whiff and reaction voices · [ ] N3d3 umpire · [ ] N3d4 gallery.
