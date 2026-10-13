# 1 — hover selects only when the mouse moves onto a new row (FINAL)

## Cause
Neither grid's ←→↑↓ table points back (`pill_step`, the Code screen's column/row walk) and no online code re-sets
`Menu::sel` per frame (`heard`, `settle`, `align`, `poll` checked). The one thing that sets it from outside the pad
is B221's pointer hover (`pointer.rs` `pointer`): whenever `Window::cursor_position()` differed from last frame by any
amount it sent a hover act for whatever row was under the mouse, and `Menu::point` moved the cursor there. A mouse
left resting over Copy Link (or over E) whose reported position jitters — sub-pixel moves, the compositor or browser
re-sending a position, a pad's touchpad seen as a pointer — re-selects that row right after every pad move: the
"snaps back" / "stuck on E" symptom. It isn't specific to the online screens: every main-menu screen uses the same
hover. Copy Link (host lobby singles) and E are both row 4, which made it look index-based; it's position-based.

No earlier "fixed" commit for this was found in `git log` (searched cursor/hover/snap/stuck); B221 (hover on every
screen) is where it came in.

## Fix
`pointer::State::hovered`: the mouse's act is remembered and a hover goes out only when it changes (the mouse moved
onto another row/face, or off everything), so a resting or jittering mouse never overrides the pad or keyboard.
Clicks, taps, drags and the wheel are unchanged.

## Test
`play::main_menu::pointer::tests::resting_mouse_keeps_the_pads_cursor` runs the real `pointer` system on a window
with the mouse jittering 0.25 px every frame: on the host lobby, hover selects Copy Link, → goes to Leave and it
stays there for 29 frames (the lobby code filling in on frame 10); on the name entry with the mouse on E, the pad walks
all 40 keys and the 4 buttons. Without the fix it fails on frame 1 (sel 7 → 6), the reported symptom.

## Not verified / not 1:1
- Not reproduced with the user's own controller/mouse: the jitter source on their setup (compositor, browser,
  touchpad) is inferred; the test simulates it. Menus are ours (PLAN), nothing to compare with the original.
- After a screen change, if the mouse rests on the same kind of act (e.g. `Row(3)`) on both screens, small moves
  within it don't select it until it moves onto another row — accepted.
- The in-match pause menu (`play/menu.rs`, its own pill hover) still hovers on any position change; not a main-menu
  screen and shared with other agents' edits, left as is.
