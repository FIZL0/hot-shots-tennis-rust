# M8f9 — court and umpire mods in the store

Done 2026-10-15. Court and umpire mods now upload, list, install and remove through the mod store and the Mods
pages the same way characters do. Each one is tagged with its kind.

## What changed
- `mods.rs`: `read_any(dir)` reads a mod of any kind this build plays, each checked as its kind (`read`,
  `mods_court::read`, `mods_umpire::read`). The result is the `Mod` the pages, Install and Upload use. A court or
  umpire becomes a header: folder, id, name and game, with no costumes. `list_kind(root, kind)` lists one kind.
- `modfs.rs`: the install checks (`check_all`, the staging check, `folder_for`, the result) go through `read_any`.
  Before this, `mods::read` refused a court or umpire zip ("not a character mod"), so none could install.
- `modstore.rs`: `package` takes `kind` from `mod.json` (`Kind::of`). For a court or umpire it leaves style,
  grades, hand, costumes and like empty. The preview is `previews/umpire.png` / `previews/court.png` (standard §2/§3),
  falling back to the first `previews/*.png`. B400: an umpire's `thumb.png` is the head square
  (`mods_umpire::head`, moved out of `umpire_mods.rs`, cut by `head_of`), so Browse tiles show the head the way the
  umpire strip's card does. The detail panel shows the whole picture.
- `mods_screen.rs` / `grid.rs`: `View` keeps `courts` and `umpires` next to the characters. `locals()` and
  `of_kind()` give each kind tab its rows. Installed / Upload / Remove all / browse Remove index those rows, `have`
  covers every kind (Browse's Installed/Update tags), the Installed page tab counts all kinds, and the kind tabs count
  their own. Browse's rail drops the play style and left-handed filters on the Courts and Umpires tabs, and the games
  it names are that tab's. B400 again: installed umpire cards and tiles show the head
  (`umpire_mods::face_of` → `Tex::Umpire`, clipped round in `face_frame`). The shelf shows a court's or umpire's
  own picture at its own aspect (`shot_aspect`) and never asks the select to bake one. The Add mod info text names
  the kind.
- `mods_kind.rs`: Courts and Umpires aren't "coming" any more. Only Texture packs (M5) still are.
- After an install, Reload restarts the game, which lists the new courts and umpires on Select Court / Select Umpire
  (`mods_court::list` / `mods_umpire::list` at menu setup, as since M8f3/M8f6).

## Tests
- `modstore::courts_and_umpires_go_up_and_come_down`: an umpire mod built in the test and the smallest court in
  `mods/hst-mods` (when linked in) each package with their kind and no character facts, carry their own picture as
  the preview, round-trip `meta.json`, install from the zip, list as their kind (`mods_umpire::list` /
  `mods_court::list`, as the selects read them), and stay off the character roster. The umpire tile's head crop is
  36×36, transparent above the head. About 1 s.
- `mods_screen::kind_tabs_filter_every_page` (rewritten): on every page Courts and Umpires list only their kind,
  installed or from the store, with per-tab counts, and Browse's rail has no style or hand filters there. Texture
  packs is now the tab that stays empty.
- `mods_kind::tabs_go_round_and_say_what_is_coming`: courts and umpires are carried.
- `tools/check.sh`: 700 tests passed.

## Shots (`context/M8f9/run2/`, 1280×720)
- `browse-1.png` / `browse-2.png`: `HST_MENU=browsemods HST_KIND=1|2`. The 16 Get a Grip courts under Courts and
  the 8 umpires under Umpires (the sample's store is made up from the installed mods, now of the chosen kind). The
  umpire tiles show their heads, and the court detail panel shows `court.png` at 16:9.
- `installed-1.png` / `installed-2.png`: `HST_MENU=addmods HST_KIND=1|2`. The installed courts and umpires (umpire
  heads on the cards).
- `court-select.png` / `umpire-select.png`: `HST_MENU=courts-mod|umpires-mod`. The installed ones after the disc
  courts and umpires.
- `match.png`: `--play --singles --court-mod gag_c01_yamate --umpire-mod gag_ump00_natasha --umpire 2`. A match
  on a Get a Grip court with a Get a Grip umpire in the chair.

## Parent's done-when (M8f)
- Browse lists the Get a Grip court and umpire under Courts and Umpires: yes (shots above).
- A `--play` match on a Get a Grip court with a Get a Grip umpire calling: yes (`match.png`; the calls in her own
  voice are M8f3's test).
- "with its NPCs": not yet. M8f7 (court NPCs) is still open, so the gallery is the layout court's disc crowd. M8f
  stays open until M8f7, M8f8 and M8f10 land.

## Not verified / not 1:1
- No real upload or download of a court or umpire against the live Hugging Face store. Publishing Get a Grip data to
  the public store is outward-facing and the run is unattended, so it was left to the user. The package → zip →
  install path is the one the live store uses (same `package`, same `modfs` install) and is tested offline. The
  `live_*` tests (`HST_LIVE=1`) weren't run.
- Browse tiles of courts and umpires in the `browsemods` sample show initials, because the sample is offline and has
  no store thumbs. Online, the tile is the store's `thumb.png` (an umpire's head, a court's picture shrunk).
- The Mods pages, store and menus are front end, so they have no original to match (PLAN.md). Nothing here touches
  gameplay, the sim RNG or match timing.
