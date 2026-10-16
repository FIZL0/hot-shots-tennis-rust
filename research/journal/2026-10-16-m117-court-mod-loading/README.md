# M117 — a court mod's own loading screen picture

- New optional file in a court mod: `previews/loading.png` (`mods_court::LOADING`, `Court::loading_picture`), read
  through `modfs` so it works from a zip too. Any size; 16:9 (e.g. 1920×1080) suits most windows.
- `loading::cover(iso, court, picture)`: with a picture that reads and decodes, it replaces the layout court's
  `L_bg` BMP behind the logo panel, hint pane and "Now Loading..."; otherwise (none shipped, unreadable, not a PNG)
  the layout (donor) court's picture as before. `match_mode` passes the match's court mod's picture.
- Any aspect ratio: the picture *covers* the window at its own aspect (`width: 100vw, min-width: 100vh × aspect,
  aspect_ratio`), centred and clipped by the cover root; never stretched. The pane etc. stay the centred 4:3 screen.
- Not `previews/court.png` (the court select's flyover screenshot): the user (2026-10-16, mid-task) said that one looks
  bad as a loading picture, so it is not used as a fallback either.
- Test: `mods_court::tests::reads_and_lists_court_mods` (picture found / absent).

## Check
Temporary (not committed) Startup hook drawing the cover under `--shot` with a 1920×1080 test pattern (circle,
border, diagonals): `context/m117/run1/` `wide.png` (16:9: exact fit), `tall.png` (600×1000: full height, sides
cropped, circle round), `ultra.png` (3:1: full width, top/bottom cropped, round), `donor.png` (no picture: court 6's
disc picture as B43).

## Follow-up
M119: put `previews/loading.png` in the mods repo's standard (§2) and export Get a Grip's own court loading art for
its court packs (this run may not commit to ../opentennis-mods).

## Not verified / not 1:1
- No original to match: the original has no court mods; the layout is ours (front-end, PLAN: not 1:1).
- Checked through a temporary `--shot` hook, not by loading a real menu → match with an installed court mod (none
  is installed here, and none ships a `loading.png` yet: M119).
- The standard doc in ../opentennis-mods isn't updated yet (M119), so mod authors don't know about the file.
