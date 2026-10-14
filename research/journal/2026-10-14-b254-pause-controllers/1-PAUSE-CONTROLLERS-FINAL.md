# B254 — Reassign controllers from the pause menu

A local match's pause dialog gets a fourth pill, "Controllers" (between Settings and Return to the Menu; online
matches, detected by the `Netplay` resource, keep the three). It opens the main menu's own controller assignment
(B238's screen: seat cards, device names, red ✕, legend, pointer hits) in the pause overlay that already hosts the
Settings screens (`main_menu/pause.rs`), filled from the match:

- each human seat's device from `controls::PadSlots` (pad entity, else the keyboard if it is that seat's; no `--pads`:
  1P's pad shown if it has one), a human seat whose pad is out is empty ("Open");
- the computer's seats are `bots`, fixed: new `Menu::paused` turns off △ (add/remove COM), 1P's ←→ swaps only devices
  and skips the computer's seats, ✕ / Tab / □ may take any device off (1P too, no rotating the others up over a COM
  seat).
- Others' devices (a newly plugged pad) join a free human seat with ✕, move with ←→, leave with ○, as in the menu.
- 1P's ✕ ("Done" in the legend) hands the seats to `PadSlots` (entities + `Seat`s against the sorted pad list) if
  anything changed; Back (○/Esc/the title tab) leaves them. Either returns to the pause dialog.

Who is human (`Pads::seats`) never changes and the match stays paused (fixed clock held), so the sim, its RNG and the
match timing only see which device presses for a seat. Legend fix while there: "Join" is not offered when only COM
seats are free (step never let a device take one).

Tests: `pause::reassign_in_a_match` (swap past COM, △ inert, leave/join, apply, `PadSlots::update` keeps it),
`menu::controllers_row` (4-pill layout, ✕ on it). Shots: `HST_PAUSE=4 HST_PAUSE_SETTINGS=assign` →
`context/b254/{dialog,assign,assign_wide}.png` (16:9 doubles, 8:3 singles), checked by eye.

## Not verified / not 1:1
- The original has no such screen (remaster-only, menus are ours); nothing to compare.
- Not driven with a real second gamepad being plugged in mid-match: the join path is the menu's (unit-tested) and
  `PadSlots::update` (unit-tested); no physical pads on this machine run.
- The screen keeps the assignment screen's title "Game Selection" (its `TITLES` entry), not "Controllers".
- A human seat left without a device stays empty until reassigned (a pad plugged in later doesn't take it by itself).
