# B385 — Back doesn't unselect P1's character in Local

(The id B385 was also used for the WebGL2 panic, journal `2026-10-14-b385-webgl2-unbatchable-panic`; this is the
roster back-button line.)

## Finding
Couldn't reproduce on current main. B355 (commit c7c9d292, merged 2026-10-14 20:55) already made ○ on the COM card
after 1P's pick undo 1P's pick (`Menu::step`, `Screen::Chars`, the `seats[p].is_none()` branch). Before that it readied
the computer on its automatic pick and went on to the confirm screen, which is what this report describes. The report
most likely came from a build made before that merge.

Checked in the real app (release build, `HST_MENU=main`, `HST_TRACE=1`, screenshots in `context/b385/`):
- keyboard (`HST_KEYS`): Enter (Local) → → → Enter (Ashley, Ready!) → Esc: 1P not ready any more, stays on the
  roster, 2P back to its "Press Enter to join" COM card.
- pad (`tools/vpad.py`, uinput, which gilrs sees as a controller): ✕ (Local) → → → ✕ → ○: same result, 1P shown as
  "Controller 1".

## Change
A regression test, `pad_back_after_1p_picks_undoes_it` (main_menu.rs tests): a pad that chose Local picks, ○
undoes the pick and stays on the roster with 1P picking, and a second ○ goes back to the main menu. Keyboard and touch were
already covered (`an_open_seat_is_a_com_card`, `touch_back_after_1p_picks_undoes_it`). No code change was needed.

## Not verified / not 1:1
- The web build with a real browser gamepad (Chromium's standard mapping): no browser or deploy here. The menu reads
  ○ as `GamepadButton::East` on every platform, so it should behave the same as desktop.
- The menu doesn't have to match the original (PLAN.md), so there's nothing to compare against the original game.
