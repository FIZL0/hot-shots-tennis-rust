# B220 — Settings: ↓ stops at Graphics

## Finding
Not reproducible on main (681c43dd). `Menu::step`'s Settings arm has stepped over `CONTROLS_ROW + 2` rows (every
switch, Graphics, Sound, Controls, Back) with wrap since P23, and keyboard (bound `down` = S plus ArrowDown), pad
(D-pad / stick) and touch (drag → the same `Press`, rows hit-tested per row) all feed that one `step`; the pause
menu's Settings overlay (`pause.rs`) runs the same `step` and draw.

Real app, `HST_MENU=settings`, seven ↓ presses (`HST_KEYS`): the hand sits on Controls with its description
(`context/b220/down7.png`), every row drawn above the description bar.

The likeliest cause of what the user saw was the layout before B227/B235's merges (top 90, 44 px pitch): the
lower rows ran into the description bar; now rows sit at `SETTINGS_TOP` 46 / 39.5 px, Back ends at y 400 < 404.

## Change
Test `settings_down_reaches_every_row` (main_menu.rs): ↓ walks 1..=Back then wraps to 0, ↑ wraps back to Back,
no setting changes, and Back's row ends above the description bar.

## Side fix (blocked the suite on main)
`rollback_desync_is_saved_and_play_goes_on` failed on main 681c43dd: the guest took the host's state (fresh 338)
before its own ggrs check reported the tick-330 desync, so `netplay::desync` dropped it as stale and the guest never
saved its inputs or showed the note. Now a stale report is still saved and noted once (no second resync asked, not
marked desynced).

## Not verified / not 1:1
- The exact build the user hit wasn't identified (no repro on main); the test pins the behaviour and layout.
- Pad and touch checked by code path (same `step`), not driven in the real app.
- Original game: n/a (Settings is remaster-only).
