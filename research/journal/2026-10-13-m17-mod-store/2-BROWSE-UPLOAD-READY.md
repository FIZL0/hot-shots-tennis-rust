# M17 — Browse, install, upload; desktop Add mod picker

M16 is on main, so the M17 client and screens are built on it.

## What landed
- `crates/hst/src/modstore.rs`, the store client for `FIZL0/opentennis-mods`:
  - Listing: the tree of `main` plus every open PR (`refs/pr/<n>`); `main` wins, then the newest PR. Each
    `meta.json` is checked (id = folder, 64-hex sha256, size cap, text capped and stripped of control chars).
    Cached for an hour in `mods-store.json`.
  - Install: download `mod.zip`, sha256 against meta, `modfs::install` (M16's checks), id must match. The game
    writes `mods/<id>/store.json` (sha, rev, verified) beside the mod, saved to disk on desktop (temp dir, then
    rename) or to OPFS (`hstModSave`) in the browser.
  - Upload: `check_all`, deflate zip, meta, first `previews/*.png`. Desktop writes them to `mods-upload/<id>/`; the
    browser downloads them. Then it opens the HF upload page at `mods/<id>`.
  - Desktop Add mod (user 2026-10-13: "adding mods should also show for desktop"): `pick()` opens the system file
    picker (`rfd`, the xdg portal on Linux) for a `.zip` and installs it through the same checks into `mods/<id>/`.
- Mods screen pages:
  - Installed: Add mod, Browse mods, Upload my mod, the installed mods, Reload, Back.
  - Browse: entries tagged Verified / Installed / Update, with the hovered entry's preview.
  - Upload: shows the rule, then needs a second press.
- `HST_MENU=addmods|browsemods|uploadmods` opens these pages for screenshots.
- `web/index.html`: `hstModSave` (shared with the picker) and `hstDownload`.

## Proof
- Tests (`tools/check.sh`, all pass):
  - `modstore`: tree/PR parsing against live-shaped replies, meta checks, main-then-newest-PR merge,
    package→install round trip on the real Phoebe mod (5.4 MB, 29 files) including the disk write.
  - `mods_screen`: page flow, scroll and taps.
  - Live listing (`HST_LIVE=1`): "store: 0 mods" (the repo is empty).
- Screenshots `context/m17/{addmods,browsemods,uploadmods}.png` at 16:9: rows, tags and the ticker lay out fine.
- Desktop picker: `HST_MENU=addmods`, Return sent to the window with hyprctl → the "Add mod" dialog
  (`xdg-desktop-portal-gtk`) opened; Escape → "no file picked".
- The pause test's app needed `Shelf` initialised (the shared `draw` reads it).

## Not verified
- The real cross-machine check (upload from one machine, Browse on another, install and play) needs a Hugging
  Face account to open the upload PR: in PLAN *Needs the human*.
- The preview display hasn't been seen with a real image; the store has no mods yet.
- A full desktop pick→install wasn't clicked through. The same install path (`modfs::install` + `to_disk`) is
  covered by `package_then_install`.
- macOS: `rfd` wants dialogs on the main thread and `pick` runs on a worker (marked `ponytail:`).
- M16's `real_mods_install` skips silently: it looks in `context/mods`, but the mods are in `mods/hst-mods`.
- Report button and the "Verified only" filter (M17.md) aren't built.
