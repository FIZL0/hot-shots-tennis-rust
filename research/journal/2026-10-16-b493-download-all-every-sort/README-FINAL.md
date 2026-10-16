# B493 — Download all in every sort

Before this, the footer's Download all only showed on one uploader's list (△ on a tile → "More by", `View::by`).
Now Browse's whole-store list has it too in every sort (Newest, A-Z, Uploader): footer = Refresh, Download all.

- `grid.rs` `View::words`: `(Page::Browse, None) => ["Refresh", "Download all"]`; the press queues `View::items()`
  (the tiles on view: search, Show and filters applied, the kind tab's) through the same `download_all`, which
  leaves installed-at-same-or-newer and queued mods out and starts up to Downloads at Once.
- `mods_screen.rs` `download_all(author: Option<&str>, …)`: status without "by X" for the whole list ("Downloading 2
  mods (1 installed or queued already).", "All the mods listed are installed or queued already.", "No mods listed to
  download."). Side panel line: "Install every mod listed here you don't have yet (the search and filters choose which)."
- An uploader's list unchanged (Download all, All mods; all their mods).
- Test `download_all_in_every_sort`; `browse_by_author` and the rest of `mods_screen` pass.
- Shots (`HST_MENU=browsemods`, 1280×720 and 540×960): `context/b493/` — the extra pill fits beside Refresh/Upload.

## Not verified / not 1:1
- none (front-end: ours to design, no original to match).
