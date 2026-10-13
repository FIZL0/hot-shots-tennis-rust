# B76 — "Return to the Menu" closed the game

## Cause
`--play` (a match started from the command line) added `play::plugin` straight to the base app, with no
`mode::plugin` behind it. So the pause menu's quit (and the result screen's) was a bare `AppExit` and the app closed.
Matches started from the menu were already modes with `back` set and went back fine.

## Fix
- `main.rs`: `--play` now starts the match as a mode (`mode::plugin`), with the main menu as its way back. The menu
  gets the same `Args` with `play: false` so its `load` loads the menu and not a match.
- `mode::Quit`: a marker put in next to an `AppExit` that should really quit, even from a mode that has a way back.
  `auto_shot` sets it (`--shot` done) and so does `HST_MASH` when it finishes, so unattended `--play --shot` runs
  still end once the shot is saved.
- `HST_KEYS`: a scripted key is now held for at least 1/30 s. The pause menu steps at 60 Hz, so on a fast display a
  key held for a single frame could fall between two steps and never register.
- Test `mode::tests::a_real_quit_quits_a_match`. The existing `switches_and_comes_back` already covers "a match's
  quit goes back".

## Checked in the app (screenshots in `context/b76/`)
- run3: `--play`, then Escape → Down Down → Enter at 0-0. Paused, then the Main Menu. The app stayed open, and the
  `--shot` taken later still quit it.
- run4: `HST_MENU=confirm`, then Enter (match from the menu) → pause → Return. Back on the menu (the kept Game
  Selection screen), app still running.

## Not verified / not 1:1
- The test drives the mode switch with stand-in modes, not the real pause menu inside a full match. A full match needs
  the disc and a GPU, which unit tests don't have. The real pause menu → main menu path was checked only in the app
  runs above.
- There's no title screen in between: we go straight to our main menu (the front end is ours to design, PLAN.md).
- A `--play` match's result screen now goes back to the menu too instead of closing the app.
