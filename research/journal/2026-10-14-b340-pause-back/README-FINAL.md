# B340 — Pause menu: ○ (Back) resumes the match

User 2026-10-14: ○ didn't close the pause menu. `Menu::tick` (`play/menu.rs`) cleared `cancel` every tick, following
the original (its pause dialog ignores ○). Now ○ / Backspace / K is Back and closes the menu like Start, with the back sound (1)
instead of Start's 2. A right click, or a click or tap off the pills, already counted as Back (B221), so those also resume now.
The confirm ("Leave the match?") is unchanged: ○ there is still No. ○ on the Settings / Controllers screens over the
pause still goes back to the pause menu: `step` returns early while they're open, and `after` closes them later in
the same frame, so the press doesn't carry over. The legend shows Back as "Resume" next to Start.

Test: `menu::tests::input` (○ closes with sound 1, also inside the 5-tick input delay, as Start does).

## Not verified / not 1:1
- Not 1:1 by design: the original's ○ does nothing on its pause dialog. The user asked for this change, and the pause
  menu is front-end presentation, so it doesn't have to match.
- The back sound (system sound 1, not Start's 2) is my choice. It matches ○ on the confirm.
- Not checked live in a window. The unit test covers the menu logic, and the input wiring (`cancel` from
  East/Backspace/K) was already in place.
