# B248 — mods get a unique id

Before: an install went to `mods/<mod.json id>/`, replacing whatever was there, and the store listing dropped any
entry whose id was already listed, so two different mods sharing an id (or a name) hid or overwrote each other.

Now:
- **Installed mods**: the unique id is the install folder (`Mod::folder()`). `modfs::folder_for` picks the first of
  `<id>`, `<id>-2`, `<id>-3`, … that's free or holds *this same mod*, which is then replaced (an update):
  - store download: same id and same store author (`Stamp.author`, new in `store.json`; a pre-B248 stamp has none
    and still counts as the same, so old installs update in place as before);
  - local zip (Add mod, browser import): no store stamp and a byte-identical `mod.json`.
  `mods::list` lists every folder, so the select roster and the Mods page show both. Removal (✕ twice) accepts the
  game's own `<id>-<n>` folders (`modfs::installed_by_game`); hand-made folders are still left alone. The browser
  stores OPFS folders under the folder name (`mod_save`/`mod_drop`/`import_mod`'s `id`), the page JS unchanged.
- **Store**: `merge` keeps an entry unless one with the same id *and* same author (or same zip) is listed, so two
  authors' same-id mods both show. Store entries are told apart by `Entry::key()` = `<author>/<id>` (Browse's
  Installed/Update tag, preview cache); installed ones by folder (the select's stats, the Mods page's pictures).
- Tests: `modfs::same_id_mods_both_install` (local + store cases, pre-B248 stamp, removal rule),
  `modstore::main_then_newest_pr` (same id, other author listed; same zip under another author not).

## Not verified / not 1:1
- Front-end only (not in the original): nothing to compare against the original.
- No live run of the store with two real same-id uploads (the store has none); covered by the merge unit test.
- Not looked at in a `--shot`: the change is in keys and folders, the page draws as before (unit tests cover the
  install/list path the select reads).
- Same uploader, different author strings: a main entry carries `mod.json`'s `author`, a PR entry the HF account
  name, so a store mod installed from `main` and its update PR by the same person (if the two differ) install as two
  mods instead of an update. Upload still commits to `mods/<id>/`, so two authors' same id collide on the store's
  `main` when both PRs are merged (a maintainer's call at merge time).
- Online lobbies still name a mod by its `mod.json` id (`online.rs`); the match itself already picks the right one
  by its looks hash (`mods_online.rs`).
