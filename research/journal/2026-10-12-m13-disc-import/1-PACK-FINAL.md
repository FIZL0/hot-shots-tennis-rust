# M13 — disc import, no ISO needed afterwards

## What landed
- `hst-data/src/iso.rs`: `Iso::write_pack(out, progress)` checks the disc (SYSTEM.CNF boot = SCUS_976.10 and the
  US 1.00 GAME.BIN, via `exe::Game::new`), then copies every file except the FMV movies (`A/*.PSS`, 217 MB), the IOP
  driver modules (`MODULES/`, `IRX/`) and `ZZBIN/MOVIE.BIN` into one file: `HSTPACK1`, XXH3-64 hash of the rest,
  index (name, offset, size), data. Written to `.part`, renamed when complete. `Iso::open` recognises the magic
  and reads the index, so every existing `iso.read(...)` caller works on a pack unchanged. `Iso::pack_hash` is the
  hash (deterministic: sorted names; for M2e's peer agreement).
- An exclusion list rather than a list of the files read today: a later port that reads one more disc file still
  finds it in an old pack. Cost: ~3 MB of files nothing reads yet (SAITOU/, a few menus).
- `crates/hst/src/disc.rs` + 2 lines in `main.rs`: `hst [flags]` (first arg a flag or nothing) runs from
  `$XDG_DATA_HOME/hst/disc.pack` (`~/.local/share/hst`, `%APPDATA%\hst`); with no pack it asks for the ISO with
  zenity/kdialog and imports first. `hst --import <iso> [flags]` re-imports. `hst <iso|pack> …` unchanged, so all
  dev tools keep running from the ISO. The import copies settings.txt from beside the ISO and ends with a message
  (stderr + zenity) that the ISO is no longer needed.
- Test `crates/hst-data/tests/pack.rs`: every packed file reads byte for byte as from the ISO, re-import gives the
  same hash, pack < 1/5 of the image; a non-ISO file is rejected.

## Verified (Linux)
- Pack: 170 820 213 bytes (171 MB) vs 1 431 371 776 (1.43 GB). Import takes ~2 s from a warm page cache.
- With the ISO replaced by an empty file (`bwrap --ro-bind /dev/null <iso>`), `XDG_DATA_HOME=… hst --play --stage 1`
  starts and plays a doubles match identical to the ISO run (shots `context/m13/pack_stage1.png` vs
  `iso_stage1.png`), and plain `hst` opens the main menu (`context/m13/menu_pack.png`).

## Not verified / not 1:1
- **Browser (OPFS)**: no wasm build of `hst` exists yet (M3 open), so there is nothing to run it in; split to
  **M13a** (file input → OPFS pack, read back from OPFS).
- **M12's install.sh**: M12 isn't done (no install.sh yet). When it lands its `.desktop` Exec can be plain `hst`
  (the game finds its pack) instead of remembering the ISO path.
- **"Offer to forget the ISO"**: the game never stores the ISO path, so there is nothing to forget; the import's
  closing message says the ISO can be moved or deleted. Never deletes the user's file itself.
- **GAME.BIN / MENU.BIN / SCUS_976.10 / SG2IOPM1.IRX are packed whole** (2.4 MB), not just the ranges `exe` reads:
  `exe`'s disc checks look at their sizes and headers, and stripping would need every reader to log its ranges.
- mods/ beside the old ISO are not copied (can be large); the import message doesn't mention them.
- The zenity dialog path (first start with no pack and no argument) wasn't clicked through in this unattended run;
  `--import` (the same import code) was.
- Windows: no file dialog (zenity/kdialog are Linux); there `hst --import <iso>` or dropping the ISO on hst.exe
  (runs from the ISO directly). Not built/tested on Windows.
