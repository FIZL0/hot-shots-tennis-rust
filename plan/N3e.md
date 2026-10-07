# N3e

- [ ] **N3e — Music and jingles.** BGM banks have no sequences section; music is a `.mid`-style sequence with
  `0x90` note-on / `0xc0` program change (event table `0x1bbd00`, programs via header section `0x10`). Port the
  sequencer tick (tempo, deltas) and play court/menu BGM and jingles (slot 8). Verify note-on timing against a ring
  recording. j: 2026-10-07-n3-audio
