# P19a — the pause screen (FINAL)

## What the original does
- Start toggles pause (system sound bank `sys_se00`, program 0 key 2). Only one in-match dialog exists: the pause
  dialog with two rows, "Resume the Match" / "Return to the Menu"; there is no confirm step. ✕ chooses (key 2),
  ○ does nothing, Start closes it again (key 2). Up/down: key 0, wrap-around, repeat after 32 ticks held then
  every 8, one direction at a time. Input is ignored for 5 ticks after opening.
- "Return to the Menu": screen fade to black over 60 ticks, then back to the menu (the port quits).
- `KeyAssign_inpane3` is "Skip / Replay / Change Camera / End Demo" (replay/demo help), not part of pause.
- Draw (HUD object's pause branch replaces every other HUD mode): `i_pause_result_15` full screen at alpha 64;
  top bar `inpane_pause03` (3 pieces: 352 wide stretch, native 32, 256 stretch); `inpane_pause02` "Paused" at
  (24,16), best-of sets digit (u = (2·sets−2)·16, tint 127,59,59) at (176,24), "sets", games digit (tint 127,97,24)
  at (248,24), "games". The score board is the result board's routine in its pause mode (frame, stripes, plates,
  each set's games) with the points from `inpane_pause00` (tiebreak `01`), 64×48 cells at x 184 / 390, y 352
  (320 for best of 5); advantage: leader A, other side alpha 64.
- Dialog (`i_dialog_00` frame/pills, `i_dialog_01` text rows of 32 px, `hsm_yubi` hand): y lift −32 (−72 for best of
  5); frame 160..480 from y = lift − 16(n−1) + 184; option rows from lift − 16(n−1) + 204, 40 apart; pill tint
  123,105,79 chosen / 94,83,70 other, then a white sheen (v 88); text white alpha 128 chosen, 94,83,70 alpha 64
  other; hand at (128 + swing, row + 8).
- Hand swing: x out to −16 decelerating (step ((30−t)/3)²/100 + 0.35), held 11 ticks, back to 0 accelerating
  ((t/3)²/100 + 0.35), 57 ticks a cycle; restarts at each move. A pulsing value (60..100, ±0.889 a tick) is
  computed in the dialog but not drawn by this dialog (not ported).
- Alpha: measured on the screenshots (paused vs resumed, linear fit per region): dim ×0.5, top bar and board centre
  ×0.5 over that — the INPANE sheets' raw alpha/255 × sprite alpha/128, as `panel::image` already decodes.

## Port (`play/menu.rs`)
- Esc / Start opens it; the fixed 60 Hz clock is held (huge timestep, overstep dropped on closing) so the match
  freezes; the other UI roots (panel, pop-ups, the port's text) are hidden while it is up. Presses made in the menu
  are cleared from the pads. Start no longer serves (it never did in the original).
- `HST_PAUSE=<s>` opens it at that time for `--shot`. Doubles: `context/shots/p19a/port_pause.png` against
  `pause_0.png` (slot 3): positions match; singles `port_pause_singles.png`.
- Tests: `play::menu::tests::{slot3_pause, points, input}`.
- Fixed on the way: pop-ups read player 0's face from the wrong panel texture since P19 added `i_gameinfo_00`
  (`panel::FACES`).

## Not done / unchecked
- Whether the original pauses the BGM; the 3D call models of a running pop-up stay visible.
- Scripts: `research/pause_shots.py <out> [buttons…]` (slot 3, Start, then each button, screenshots + RAM).
