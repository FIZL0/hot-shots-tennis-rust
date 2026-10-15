# B387 — mods' Reload / Later popup: no Back, and a Reload after Later

**Cause.** Two separate things. The popup's `ask_step` treated ○ like Later, and a tap off the popup counted as Later too (B221). On top of that, Mods opens on **Browse**. Reload was only a pill on **Installed**, so after Later there was no Reload where the player ends up.

**Fix** (`play/main_menu/mods_screen.rs`, `mods_screen/grid.rs`, `legend.rs`, `pointer.rs` test):
- ○ (and a right click, which is ○) does nothing while the popup is up. A tap off the popup does nothing either (`d.popup(None)`). Only Later, or Reload, closes it. The legend no longer offers "○ Later".
- While mods have changed since the start (`View::changed`, the same flag behind the main menu's note), Browse's heading gets a **Reload** word after Refresh (or after Download all / All mods on an author's list). It runs the same `reload()` as the popup and Installed's pill, and it has its own description line. Installed's Reload pill is unchanged.
- `HST_MENU=browsemods HST_RELOAD=1` shows Browse with the Reload word for `--shot`s.

**Tests.** `removes_from_the_child` now checks that ○ leaves the popup up, and that Browse lists Reload after Later and drops it once nothing has changed. `taps_outside_popups` checks that a tap off the popup has no action and that Later still closes it. The legend test passes with the new legend. `tools/check.sh -p hst --bin hst main_menu`: 113 passed.

**Looked at.** `context/b387/browse-reload.png` shows Reload in Browse's heading at 16:9, and it fits beside the sorts and Refresh. `context/b387/popup.png` shows the popup with the legend down to Move and Choose.

## Not verified / not 1:1
- Pressing Reload itself (restarting the process) wasn't run in a test, because it would spawn the game. It is the same `reload()` that Installed's pill and the popup use.
- On a kind tab whose mods can't be installed yet (Courts, Umpires, Texture packs: `mods_kind::coming`), no page shows pills. Reload is hidden there too, as it already was on Installed. Switching back to Characters shows it again.
- There is no original to compare against: this is a front-end screen of our own.
