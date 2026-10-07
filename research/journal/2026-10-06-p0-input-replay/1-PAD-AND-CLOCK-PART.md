# P0 input replay — pad buffer + frame clock (PART, paused for P0a at the user's request)

- Pad manager object: 0x2efb00 (passed to the pad update 0x187a40 by 0x1878b0).
  Decoded state per port/slot: base + port*0x48 + slot*0x12 + 0x30, 18 bytes:
  u16 buttons (active-high after ^0xffff; libpad bits: 0x1 Select, 0x8 Start, 0x10 Up, 0x20 Right, 0x40 Down,
  0x80 Left, 0x100 L2, 0x200 R2, 0x400 L1, 0x800 R1, 0x1000 Tri, 0x2000 Circle, 0x4000 Cross, 0x8000 Square),
  4 stick bytes (deadzone 0x51..0xae → 0x80), then pressure bytes.
  Raw libpad block: base + (slot + port*4)*0x140 + 0x116; mode at +0x110 (0x41 digital, 0x73/0x79 analog/DS2).
  Save states show port 0 mode 0x79, buttons 0x0801 (R1+Select = the user's PCSX2 save-state hotkey).
- Frame counters (+15 between slot 8 and 9, saved 0.25 s apart): 0x1d5780, 0x2eef90, 0x30d284 (same value),
  0x2f1804.. (vsync-ish), 0x427af4 (game-local). Pick one and confirm +1/frame live.
- PINE answers requests at vsync: do ONE batch per poll (pointers resolved once) to sample every frame.
- Next: player position offsets in player objects (*(gm+0xa8+4i)); recorder; pure-Rust match sim for replay.
