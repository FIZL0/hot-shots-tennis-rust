# B339 — parallel mod store downloads

Download all (and several installs pressed in a row) now download side by side instead of one after another.

## What changed
- `modstore::Event::Installed` / `Progress` carry the install's `Entry::key` (Add mod's pick: ""), so each outcome
  ends its own install; the Mods page keeps `doing: Vec<Entry>` and starts queued ones while fewer than
  Downloads at Once run (`View::next` / `View::finish`). A progress line also updates that mod's own tag text.
- Shared paths made per mod: the whole zip is `mods/.download-<author>-<id>.zip`; a store mod unpacks into
  `mods/.import-<author>-<id>/` (a local zip keeps `.import`). Picking the final folder and moving in
  (`folder_for` + rename) holds a mutex, so two same-id mods can't both take `<id>-2`.
- `mods::list` skips dot folders: with several installs, a refresh after one finishes would otherwise list another's
  half-unpacked stage (it could before too, with `.import`, if anything refreshed mid-install).
- Settings → Online & Mods → **Downloads at Once**: Auto, 1..max (`settings.txt` `downloads_at_once`). Auto = 3 on
  desktop, 2 in a browser, 1 on a phone; max 8 / 4 / 2 (a browser holds each ranged install in memory until the page
  has written it to OPFS, B336).
- Pressing Install on a mod already queued or downloading says "… is on its way already." instead of queueing it twice.
- `web/index.html`: no change needed — `hstModSave` is per id and parallel saves of different ids don't share files.

## Verified
- `downloads_side_by_side` (mods_screen): 5 queued at 3 → 3 run, outcomes by key start the next, Add mod's pick ends
  none, at 1 the rest wait; settings round trip + option stepping tests.
- Live `live_parallel` (`HST_LIVE_PARALLEL=4`): the store's four smallest mods (umpires), two whole and two ranged,
  installed at once into one folder: all Ok, four folders, no dot files left.
- `--shot` of the Online & Mods page at 1280×720 and 700×1000 (`context/B339/`): the new row lays out at both.

## Not verified / not 1:1
- Not 1:1: front-end feature of ours (no original counterpart).
- The browser build wasn't run (only `cargo check --target wasm32-unknown-unknown`): parallel ranged installs and
  OPFS saves in a real browser/phone, and whether 2/4 (phone 1/2) are the right limits for memory, are untested.
- Live test used tiny mods only (umpires, <1 MB); several 500 MB mods at once weren't downloaded.
- The side panel's 9 choice chips for Downloads at Once weren't screenshotted with the row selected.
- Note: `context/notes/check.log` is a symlink shared by all worktrees, so another slot's check.sh can overwrite it
  mid-read; the live run's output was taken from a scratch log.
