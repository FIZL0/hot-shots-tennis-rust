# B97 — back to the menu from a match: black screen / app gone

## Cause
`play/menu.rs` writes `AppExit` on **every** frame once the pause menu's "Return to the Menu" fade reaches 60 ticks.
`mode.rs::back` turned the first one into the switch (taking the mode's `back`), but since R1/B63 the switch doesn't
happen the next frame: `leave` waits up to 10 frames for the zoom-out snapshot, then the loading cover waits ≥3 frames.
The match keeps running meanwhile, writes `AppExit` again, `back` now finds no way back and lets it through → winit
closes the window and the app exits. Seen in run r1 (unpatched): the log ends with
`Unknown window for screenshot, skipping` right after the quit, so the process was gone. The user saw a black window
instead; that's likely the same exit with the window outliving the frame loop for a moment (not reproduced here).

## Fix
`mode.rs`: `Running.leaving` is set when `back` takes the way back; while it's set, any further `AppExit` from the
leaving mode is swallowed (it's the same request). A new mode's `Running` starts with `leaving: false`, so the menu's
own Quit still quits. Closing the window still quits (the primary-window check is unchanged).
Test `mode::tests::switches_and_comes_back` now lets each mode settle (loading cover down) before switching, so the
`leave` wait really runs, and mode 2 asks for `AppExit` every frame while a `Quit` resource is in; `loading` asserts
nothing quits during a switch. Fails on the old `back`, passes on the new.

## Scripted check (HST_KEYS)
`main.rs` gained `HST_KEYS="secs:Key … secs:shot=path.png"` (real-time seconds; Enter, Escape, Arrow* pressed for one
frame; `shot=` saves the window without quitting, unlike `--shot`), for unattended runs across modes. Run as
`HST_BG=1 HST_MENU=confirm HST_KEYS='4:Enter 22:Escape 24:ArrowDown 25:Enter 32:shot=target/b97/menu.png' hst <iso>`
(shots under `target/`, not the scratchpad: Hyprland-launched commands can't see it).
- r2/r3: menu → match → pause → Return to the Menu → **main menu shown** (target/b97/r2-c.png, r3-3menu.png); r2 then
  walked Local → Game Selection → Character Selection, so the menu is live, not just drawn.
- r4: match → quit → **main menu shown again** (r4-6menu.png, after a fresh menu Startup: "92 custom characters" logged
  a second time).

## Not verified / not 1:1
- **No single run did two complete match → menu round trips.** In r4 and r5 the first scripted Escape didn't open the
  pause menu (the match stayed on the serve; r3's identical first round worked, r4's second round worked). Not
  diagnosed: a debug run (r6) was cut off when Hyprland crashed (SIGABRT in `CDefaultFloatingAlgorithm::newTarget`
  while mapping a window, 02:13:52, the second r6's floating window opened; an earlier Hyprland abort at 00:07 predates
  this task). Hyprland came back in safe mode with the session locked, so no more floating test windows were launched.
  Suspects: another agent's virtual pad (r1 logged "HST vpad 4" connected to this window, and Start on any pad toggles
  pause) or frame throttling of a hidden window. The fix itself is per mode switch (fresh `Running` each time), and
  the unit test covers back → menu → quit.
- The user's "black screen" symptom was seen here as the app exiting; not reproduced as a lingering black window.
- Floating `hyprctl exec` windows may crash this Hyprland build when mapped (see above); worth knowing for tools/shot.sh.
