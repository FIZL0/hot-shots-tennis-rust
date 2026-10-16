# B448 — Mods and Settings in the character select's look

Task: `plan/B448.md`. The concept artifact (claude.ai link) isn't reachable from an unattended run, so the
written spec in the plan file was followed.

## What changed
- `play/main_menu/look.rs` (new): the select's look (B422, `roster.rs`) as shared parts: navy backdrop (no grid
  paper), plain white "< Title" (tap = Back), ring-bordered tab pills between button art, 14 px / 3 px-ring columns,
  well pills (chosen filled with the ring colour), "< value >" step pills, section headings with count and rule,
  6 px scroll bar, tiles (selected = selected colour + 3 px inset ring), paragraphs. `Zoom` draws a portrait page
  on a ~420-unit-wide view and scales quads and hits back out so phones read like the mockup.
- Mods (`mods_screen/grid.rs` rewritten, `mods_screen.rs`, `mods_kind.rs`): kinds in seat colours (Characters red,
  Texture packs gold, Umpires blue, Courts green), L2/R2 kind tabs; header tag STORE / UPLOAD, big "Browse …",
  status right; show pills Everything · Installed · Updates · My uploads · Saved, "< Newest >" sort, one filter pill
  (Select cycles Game / Plays like / Hand, only when it has more than one choice); tiles under a heading per game
  (the first in the ring colour), badges Update / Installed / New and Uploaded / Uploading / Queued / Failed /
  Skipped, progress bar; footer line with the page's words (Add mod, Reload, Remove all, Download all, …).
  Right column: name with "by …", costumes left / hand right as plain words (no switcher), picture with the gold
  play-style label over the pink stats card, facts with the green verified line, ring primary + two well
  secondary buttons. Questions (Remove, Take down, Upload all, Sign in) sit inside the column in a selected-colour
  box sized to their words; the column then only names them. Upload: rights rule in a well box over the buttons.
- Settings (`options.rs`): navy, "< Settings", page tabs between L1/R1 (blue set), list column with page name and
  option count, rows as tiles with "< value >" pills (Off soft; picked for editing = filled pill, B326 halves still
  step), volume bars in seat colours, "Changes apply right away" at the list's foot; side column: name, one-line
  explanation, CHOICES rule, choice pills (current filled). Controls page the same (key / pad pills).
- Lobby and the confirm screen (they share `frame.rs`): the grid paper is now the select's navy (`navy_frame`);
  the labels that were dark green on paper are soft blue-grey.
- Removed what this left unused (the old mods card/side panel, `attribute_card`, `CELL_FILL`/`CELL_ON`, the old
  Settings tab row and paragraph helpers).

## Checked
- `tools/check.sh -p hst`: 452 passed (pointer taps on Settings rows/tabs and Controls rebind, mods grid moves,
  scrolls, taps, kind tabs, Upload, My uploads, remove-all words, by-author).
- Shots (in `context/b448/after/`, before in `context/b448/before/`): browse, by-author, upload (+ Upload all
  question), sign in, my uploads (+ Take down question), courts, umpires, Settings Display/Graphics/Sound
  (editing)/Controls, lobby, confirm, at 1280×720, 960×720, 1680×720 and 400×800.

## Not verified / not 1:1
- The concept artifact itself: not reachable here; built from `plan/B448.md`'s written spec, so spacing and sizes
  are the spec's px, not measured off the concept.
- "New" badge = a store mod not installed and not yet reviewed (my reading of the spec; the spec doesn't define it).
- The on-screen keyboard, the "no phone" and wipe popups, and the Renderer switch popup keep their old style and
  are drawn unzoomed over the page (out of the spec's scope).
- Portrait: pages are laid out ~420 units wide and scaled up (my choice to make phone text readable).
- After switching show pill the cursor stays on the pill (B383 moved it to the search box): the pill row is now the
  page's own control.
- Settings uses the blue seat colours; the spec names no colour for it.
- Lobby/confirm: only the background and the label colour changed; their cards and title band keep B407's look.
- Texture packs (gold) shown only through `seat_of`; no texture-pack store sample to shoot.
