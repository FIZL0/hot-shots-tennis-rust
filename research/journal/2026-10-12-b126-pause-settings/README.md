# B126 — Settings in the in-match pause menu

## What landed
- Pause menu has three pills: Resume · **Settings** · Return to the Menu (`play/menu.rs`, `OPTIONS = [6, SETTINGS, 1]`).
  The "Settings" pill label is drawn from word.tm2 (white, 2-px dark outline, like i_dialog_01's rows).
- Settings opens the main menu's own Settings screen over the paused match (`play/main_menu/pause.rs`): same
  `Menu`/`step`/`draw` code, so the sections (Options, Graphics, Controls) and the settings.txt save are the same.
  Back on the top Settings screen returns to the pause menu with Settings still selected, not to the match.
- Live while paused: widescreen (global `WIDE`; `widescreen::bars` now also resizes the bars after spawn),
  Graphics (resource), match music (loads the court's `bgmg_NN` bank and plays it, or stops it). Controls save to
  settings.txt and apply to the next match.
- The main menu re-reads settings.txt when it comes back from a match (`Kept`), so changes made in the pause menu
  show there.
- Mouse/touch: a click or tap on a pause pill selects and confirms it; on a settings row it selects and confirms
  that row; right-click is Back. Window→640×448 mapping goes through widescreen `place`, so it works at any aspect.
- Sim: the pause keeps the fixed clock frozen while settings are open; nothing touches the RNG or the tick count.
- Script hook for runs: `HST_PAUSE_SETTINGS=settings|graphics|controls` opens the screen straight away.

## Checked
- Unit tests: `rows_under_clicks`, `window_to_screen`, `back_returns_to_pause`, the updated menu layout/input tests
  (28 pass).
- Shots (context/b126/): pause dialog with 3 pills; Settings over the match at 16:9, the same as the main menu's.
- `--play` scripted run: widescreen off live → Escape back to pause (Settings selected) → resume shows 4:3 with
  bars; settings.txt has `widescreen = off`.
- Menu-started run (menu_run.png, m1–m4): Match Music on mid-match → Return to the Menu → main menu → Settings
  shows Widescreen OFF, Match Music ON (so the next launch and the Kept reload both pick it up).

## Not verified / not 1:1
- Upscaled textures and Menu Music save but only take effect in the next match / back in the menu (textures are
  loaded at match setup; menu music isn't playing during a match).
- Mouse/touch was only checked by unit tests (hit-testing and the window mapping), not with a real click in a run.
- Live graphics changes inside a match were not checked by eye (the screen was shot; the effect on the court wasn't).
- The original pause dialog has no Settings row, so the label comes from word.tm2, not the dialog sheet. Front-end
  doesn't have to be 1:1 (PLAN).
- Return to the Menu was checked from a menu-started match only; a `--play` match has no menu to return to (B76).
