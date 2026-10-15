# B444 — long mod titles slide (marquee) on the Mods page

User 2026-10-15: a mod's title too long for its tile should scroll sideways so it can be read, not be cut off.

## What changed
- `mods_screen/grid.rs`: `marquee()` draws a line in its room; too long, the **chosen** tile's name and its game /
  uploader line rest 1.2 s at the start, slide left at 30 px/s until the end shows, rest 1.2 s, start over. The glyphs
  are cut at the room's edges (`rolled`: a glyph across an edge keeps only its inside part, src cropped to match).
  Tiles not chosen show the start ending in "..." instead of the old squeeze (squeezed text was unreadable and would
  jump when chosen). The chosen mod's detail panel: the name shrinks to fit down to 13 px, then slides; the
  "game - by - version - size" line slides too.
- Timing: `Browse::roll` (seconds on the current `Menu::sel`, reset on every cursor move), counted by
  `grid::roll_time` from Bevy `Time`, so it's the same speed at any frame rate.
- `HST_MENU=browsemods HST_LONG=<secs>`: the first two sample mods get long titles/games, the chosen one slid `secs`.
  Shots: `context/b444/long-{0,2.5,9}.png` (16:9): it slides, the others end in "...", nothing goes outside its tile.
- Test: `a_long_title_rests_slides_rests_and_starts_over` (`roll_offset` timing).

## Not verified / not 1:1
- No original to compare against: the Mods page is ours (front end isn't 1:1 per PLAN). Speed/rest times are my choice.
- Only the chosen tile slides (as console stores do); others are cut with "...". Read "game title" as the mod's name
  and its game line both.
- The Upload page's list cards (`mods_screen.rs` `card`) still shrink long names; they get no `Menu` for timing and
  weren't asked about. The rail's game names (Game group) still squeeze.
- Glyphs are cut hard at the edges, with no fade.
