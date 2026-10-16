# M113: Browse mods' detail panel shows the stats card again

## What took the room
Room wasn't the main problem on a landscape window. At 1280×720 the panel had about 205 mockup px between the grey
line and the facts. The card was missing because the **store listing has no grades**: every live upload
(PRs 99–148, `hotshotsgaming`, 2026-10-16) has a `meta.json` with no `style` or `grades`, only `hand`, `costumes` and `like`.
`package()` writes them only when the upload's `View::stats` has the mod. That map is filled once, when the Mods screen opens, by zipping
the startup roster (`menu.mods` × `Art.2.mods`). A mod added or installed after startup, or one missing from the roster,
goes up without its card. The desktop Upload page here does show the stats for the installed ones (shot
`context/M113/upload.png`), so the bad uploads came from a session where the roster lacked them.
On a squarer portrait view (768×1024) room *was* the problem. There the detail panel is about 265 px, the strip's least height of 96
plus the buttons and facts left about 25 px, and the card's 70 px test hid it.

## Changes
- `hst-base::mods::tparam_row` (split out of `tparam`) reads a mod's TParam row from a csv already read.
  `main_menu.rs` keeps the disc's TParam.csv as a `TParam` resource, and `mod_card` grades a mod as the select does.
  On the Mods screen, every listed mod not in `stats` is graded once from it. So uploads carry their card, and the
  Installed/Upload panels show it, for mods added since startup.
- `grid::card`: a store character with no grades in its listing shows its installed copy's card (the select's),
  else the card with "?" letters (`roster::stats(None)`). The style goes on the grey line only when it's known.
- `grid::card_room`: the card has a fixed spot under the grey line. The strip shrinks first (down to 56), until the card
  gets its comfortable 120. Then the card thins down to 80, with rows and letters scaled by `roster::stats`. Only then does it hide. Its most is 150.
  To place it, the foot (buttons) is drawn first to measure, and its quads are moved over the panel.
- Portrait `lay`: the grid's panel leaves the detail at least `DETAIL_MIN` (least strip + least card + line,
  three facts, buttons), keeping its own 220 floor.
- Tests: `the_strip_gives_way_before_the_card`, `a_listing_without_grades_takes_the_installed_copys`.

Shots (`context/M113/`): land2 (1280×720), phone4 (540×1170), tab5 (768×1024), wide4 (1600×600),
land43up (1024×768 Upload). The card shows on all of them.

## Not verified / not 1:1
- The concept artifact (card 1) couldn't be opened from an unattended run, so I matched the task's words: the select's
  pink card, ringed, under the uploader line.
- The uploads already on the store stay without grades until they're re-uploaded. Browse shows "?" letters for them
  unless the mod is installed. I didn't find out which session uploaded them without stats (not reproducible here).
- I didn't take a shot of the "?" card: the sample screen can't put the cursor on a non-installed, gradeless entry.
  It's covered by the unit test, and `roster::stats(None)` is the select's existing draw.
- On the 768×1024 portrait the card is at its least (80 px). Its words are small but readable.
