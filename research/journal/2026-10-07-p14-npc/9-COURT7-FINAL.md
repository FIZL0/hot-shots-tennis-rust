# 9 — Types 27–29 against a recording, court 7 (P14c3c)

## Reaching court 7
- From the Confirm Settings screen (copy 2's scratch slot 8, "Select Court" highlighted; on copy 2 ✕ confirms, ○ goes
  back): ✕ opens the court carousel, which starts on the court labelled 11 and wraps 11 → 1.
- The carousel's labels are not the court index `*(0x422f90)`. Label → index: 1→1, 2→4, 3→6, 4→2, 5→8, 6→9, 7→11,
  8→10, 9→3, **10→7**, 11→(start).
- Court 7 is Grand Palais Indoor Court: right ×10, ✕, up, ✕. The match is in play about 25 s later. That state is
  copy 2's scratch slot 9.
- It has five trigger creatures: 29, 29, 28 and 27 at the corners (±7, ±24), plus a type 30.

## Recording
`prox_c07.bin` (context/fixtures, 600 samples) was recorded from slot 9 with
`HST_POKE="1:0:0:3:0;1:2:1:0:3.5;50:1:b:0.45:0;100:3:b:0:0"`:
- Sample 1: creatures 0 and 2 are 3.0 / 3.5 from a player. They startle, and the message stays 0xe.
- Sample 50: creature 1 is 0.45 off the ball, outside the ±0.2 box. It startles from distance only, and there is no 0x14.
- Sample 100: creature 3 is on the ball. It is struck, and every object's message becomes 0x14 in that tick.

## Findings
- Message 0x14 is broadcast synchronously to the whole object tree. The trigger's message handler only stores it in
  +0xc0, so a tick whose message turns to 0x14 is stepped like any other.
- 27–29 have an animation controller but no animation header (ctrl+0x24 = 0). The controller's set then only
  stores the next frame, and its advance adds the speed with no clamp: the frame stays, and `next` counts 0, 1, 2…
  The port now does this when `len == 0`. Before, it clamped `next` to 1. The other courts are unchanged.

## Test
`startled_creatures_match_the_game` now covers court 7 too: 2396 ticks, 4 startles, 1 struck, bit-exact. The new checks:
- A creature that is `struck` must show message 0x14 after its tick.
- Where 27–29 stand, a new 0x14 must come from a struck creature in that tick.
- Widening the box to ±0.5 fails at vsync 8439 (the 0.45 poke).
