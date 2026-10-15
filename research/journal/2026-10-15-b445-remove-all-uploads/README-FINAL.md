# B445 — Remove all uploads (with a warning popup)

- My uploads (Show → My uploads) has a new heading word, **Remove all uploads** (after Refresh), in
  `mods_screen/grid.rs` `words()`; its description is "Take every mod you shared off the store (asks first)."
- It never removes anything straight away: if the player is signed out, busy, or has no uploads, the status line
  says why. Otherwise a **warning popup** opens over the dimmed page in B205's popup style (`mods_screen.rs`
  `wipe_popup`). It shows "Warning", "Remove all N of your uploads?", that they leave the store (merged ones only
  once a maintainer agrees), and "This can't be undone." Below that are the **Remove all** (red while chosen) and
  **Cancel** pills. **Cancel starts chosen.** The popup takes every press: ↑↓←→ toggle, ✕ chooses, ○ or a tap off it cancels,
  and the kind tabs are held while it's up. It follows the pointer through `mods_cursor`.
- Confirming takes the uploads down one at a time with `modstore::remove`, starting the next when the last one's
  `Event::Removed` comes back (`View::wipe_next`). Open pull requests are closed, and merged mods get a delete
  request. The status reads "Removing your uploads: X (k/n)...", then "Removed all N...", or reports how many
  failed or that the player was signed out partway.
- `confirm.rs` (named on the task line) is the match's confirm screen and isn't involved, so it wasn't touched.
- Test `remove_all_uploads_asks_first`. It never presses Remove all while a real token is stored, which would take
  the machine's own uploads down.
- Shots: `HST_MENU=mymods HST_CHILD=wipe tools/shot.sh …` at 1280×720 and 720×1280, in `context/B445/wipe2-*.png`.
  The popup scales and centres at both.

## Not verified / not 1:1
- The live take-down run against the Hugging Face store wasn't exercised: it would delete real uploads. The path
  reuses M19's `modstore::remove`, which already ships, and the sequencing and summary are unit-tested.
- Menus are ours (PLAN), so there's no original to match.
