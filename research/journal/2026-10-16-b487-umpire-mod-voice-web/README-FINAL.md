# B487: a downloaded umpire mod's "Hear voice" played the donor on web

## Cause
In the browser, a mod's big files (models, textures, voices; anything not `modfs::small`) stay in OPFS and are read
in only when `modfs::ready(dir)` asks the page (B336). A store download is no exception: `web::mod_saved` unloads
them once the page has stored the mod. Nothing on the umpire mod path ever called `ready`:

- Preview Voice (`main_menu.rs` `Effect::Hear` → `umpire_mods::preview_bank` → `mods_umpire::bank` →
  `SoundBank::with_takes`) read each `voice/<p>_<k>.wav`, got `WouldBlock`, logged "not a PCM16 wav, skipped" and
  built a bank with no takes: the donor's disc bank alone (Gloria's donor is Lily). That bank was then cached for
  the session.
- The match launch waited for `--slot-mod` folders only, and `play/umpire_mod.rs` built slot 5's bank once in
  PostStartup, so the match would have had the donor's calls too.

`has_voice` (`umpire_mods.rs`) counts lazy files, so the △ prompt showed correctly. The download did store her bank.

## Fix
- `umpire_mods::voice_ready`: a mod's Hear asks `modfs::ready` first; while the files are out, the request is kept
  (third field of the `voice` Local) and retried each frame while that umpire is still picked. Nothing is cached
  until the bank can be built.
- `umpire_mods::launch_ready`: Launch waits for the `--umpire-mod` folder like it does for `--slot-mod`.
- `play/umpire_mod.rs` `over_donor` also runs in Update until it has done its job (`Over` resource): an online
  peer's copy that is still in storage is heard as the donor until it's read in, then as the mod. This only changes
  what's heard; the timing still comes from the donor's word lengths.
- `with_takes` now logs a failed read with its real error rather than "not a PCM16 wav".
- `modfs::ready` natively: reports files a test marked lazy (`add_lazy`, now also under `testing`). Natively
  nothing else is ever lazy, so the game behaves as before.

Tests: `a_mods_voice_is_read_in_before_its_heard` (menu: preview and launch wait, disc umpires don't) and
`an_umpire_mod_still_in_the_browsers_storage_is_heard_once_read_in` (match: donor bank alone while lazy, the mod's
2 calls on top once read in, applied once). `cargo check -p hst --target wasm32-unknown-unknown` builds.

## Not verified / not 1:1
- Not run in a real browser (store download of Gloria, then △ on her card). The test covers the lazy-file path
  natively through the same `modfs` code the browser uses, apart from the page's OPFS read (`hstModLoad`). That
  read is the same one character mods already use for their models.
- Nothing about the original applies: umpire mods are an addition, and only sound changes.
