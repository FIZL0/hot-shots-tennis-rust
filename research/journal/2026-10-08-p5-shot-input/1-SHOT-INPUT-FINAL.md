# P5: shot selection by input

## The decode (retail)

- The pad driver (0x188880) keeps each button as a 2-bit field ((now<<1)|before) − 2. Its values are −2 idle, −1 released, 0 just pressed and 1 held.
- The human decode 348ff0 reads only `== 0`, the press edge. It picks a shot code from ✕ (1), then ○ (2), then △ (4), and stops at the first button pressed. Holding a button does nothing, so the game has **no hold or charge**.
  - Gameplay reads the pad's buttons only here. The one other +0x40 read is at 174309.
- The stick decides the direction. The analog stick counts unless it sits in its dead square; then the d-pad does. The direction is scaled ×1.2, clipped to length 1 and turned by the camera.
- Two-button "combos" are just that priority order: ✕+○ gives ✕, ○+△ gives ○, all three give ✕.
- Double tap: the action machine (349410) keeps a re-press window, +0x3e54 frames long (30 for smash branch 4, otherwise 2), counted against +0x3f00. A press inside it sets +0x3f04. P8a ported this as WHIFF_REPRESS.
  - During the auto-approach (+0x3f24/+0x3f28) only the first press is kept, in +0x3f2c. The port's press() already does the same.
- Serve (351dd0): an underhand toss needs △; strong and weak tosses need ✕ or ○. The port already does this.
- The task names two things that are not input:
  - "Special shots (0x408e60 list)" is the per-character up1/dw1/dw2/dw3 variant-record list (37c590). P3 ported it (record_spin, timing::load).
  - "Charged blend" is the +0x3ed8 mode/blend from swing setup 35b030, which depends on contact height, grade and stats, not on hold time. P3 ported it too.
- Stick → kind (flat/drop turn) was P5a's work (`shot::stick_kind`).

## Recording

`research/p5_input_rec.py` drives the virtual pad on a frame-counted schedule in slot 4, with P1 human. It wrote 1800 frames with no gaps to `context/p5/presses.bin`, which is also copied to `context/fixtures/p5_presses_s04.bin`. Findings:

- ✕ held for 180 frames gives one swing, at the edge. No new swing comes while it is held, even after the action returns to 0.
- ○+△ gives code 2. All three buttons give code 1.
- △ pressed twice 20 frames apart re-presses (+0x3f04 = 1).
- Presses arriving while +0x3ec1 is 5 are ignored. A swing starts in the press's own sample: the countdown +0x3ec4 leaves −1.

## Port

- `hst_sim::shot::press_kind(before, now)` is the edge decode with the ✕ > ○ > △ order. play.rs's read_input already uses just_pressed in that same order.
  - Test: `shot_tables.rs` `presses_start_swings_like_the_game`. All 12 of the game's swings start on a `press_kind` press with the game's shot code. There are 583 held-button frames, and none starts a swing.
- Fixed the d-pad. play.rs merged the d-pad and keys into the stick with clamp_length_max(1), so diagonals ran and aimed at about 64% of full length.
  - `SlotPad::dpad` now carries the d-pad bits to `loco::pad_dir`, as the original does. Test: `player.rs` human_pad_replay on this recording steps 147 d-pad-diagonal running frames exactly.
