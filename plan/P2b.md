# P2b

- [ ] **P2b — Incoming-slice aims in the fixtures.** Split from P2a
  (journal 2026-10-08-p2a-sweet-timing/1-TIMING-FINAL.md, Gaps).
- `shot::aim` scales the aim when the ball being struck is a slice in a rally (`incoming`):
  - `Some(true)` (the slice was sweet) multiplies it by 0.45;
  - `Some(false)` handles an off-sweet slice.
- P2a's plain-play fixtures miss part of this:
  - `aim_doubles.bin` has 2 incoming slices, both off-sweet;
  - `aim_singles.bin` has none;
  - so `Some(true)` isn't checked at all, and `Some(false)` is checked in doubles only.
  - P2 checked these cases only with forced +0x3ee4 / +0x3fa0 writes, and P2a removed those.
- Options, laziest first:
  - Longer recordings, keeping the aims with an incoming slice.
  - A pad-driven P2, which slices on purpose with sweet timing. It needs a second pad in the same copy (vpad.py
    has one pad per copy) and a save with P2 human.
- Done when `human_aims` has at least one `Some(true)` aim in each mode and one `Some(false)` aim in singles, all of
  them bit for bit, with no memory writes besides the singles player count. Then make aim.rs assert that coverage.
