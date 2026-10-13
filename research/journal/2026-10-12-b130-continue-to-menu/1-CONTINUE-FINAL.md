# B130 — Continue on the result screen goes back to the menu

Continue (✕ from tick 120, stats page seen) used to close the result screen, end the ceremony and let the match run
on. Now it plays the Continue sound and starts a 60-tick fade to black, then sends `AppExit`. In a match started
from the menu, `mode.rs` turns that into the switch back to the menu, the same as the pause menu's "Return to the
Menu". A match started with `--play` on its own quits, as the pause menu does. The fade is a full-window black node
(`match_stats.rs` `Fade`), so it covers any aspect ratio. While it fades the page takes no input. `ceremony::end`
(the rematch's confetti and BGM restart) had no caller left and was removed.

Checked: release build, `HST_MENU=confirm HST_STATS=14 HST_STATS_PAGE=1 HST_KEYS="3:Enter 24:Enter …"`. The log
shows Continue, then the menu mode loading about 1.7 s later. The screenshots (`context/b130/fade.png`,
`menu.png`) show the window mid-fade, then the Main Menu. `tools/check.sh -p hst` passes.

## Not verified / not 1:1
- How the original leaves its result screen (it goes to its own menus or a rematch). Per PLAN, the front end is
  ours to design, so nothing was compared.
- The fade darkens the result page more slowly than the court. The UI blends in linear light (`hud_gamma.rs`),
  so 60 % black over an opaque UI piece reads lighter. Both reach full black at tick 60. The pause menu's fade does
  the same. Checked by eye only.
