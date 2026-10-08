# 2 — the custom page and the match (FINAL)

## What it does
- `play/main_menu.rs`: `Menu.mods` = `mods::list(<iso dir>/mods)` minus any mod whose TParam row can't be read
  (logged). Tab / pad Select toggles `Menu.custom` for everyone (refused with the back sound if no mods); every
  player not ready moves to the shown page (costume reset). A player readied on the other page keeps their pick;
  unreadying puts them on the shown page.
- Custom page: names in 2 columns × 12 rows a page (`LIST`, `LIST_ROWS`), ↑↓ one entry, ←→ a column (12),
  clamped to the list; page shown = the page of the first player not ready; a hand per picking player in their
  colour; "Custom characters k/n" under the list.
- A card on a mod: the 3D preview (`previews.rs` loads `mods::load` on the worker thread, hovered costume ±1; poses
  with the donor's timing), the name in `word.tm2` (shrunk to fit), the play style from its TParam row (base row's
  type, or a `タイプ` override), △: Serve/Stroke/Volley POW, SPE, STA numbers. Costumes = `mod.json`'s count.
  Lock rule: same mod + same costume readied by another player is blocked (`Who` identity).
- Confirm screen: a mod's slot shows its name and costume number on a dark card.
- Match: `--chars` gets the donor, `--outfits` the mod costume, plus `--slot-mod <match slot> <dir>` per mod.
  `main.rs` parses any number of `--slot-mod` (and still `--mod DIR --mod-slot N`) into `mods::MatchMod`, now one
  `Option<Mod>` per slot (`get(i)`), so every player can be a mod. `mods::load` takes `impl Store` like
  `load_disc` (for the worker thread).
- `HST_SELECT` accepts `mN` (custom entry N) for `--shot`s.

## Verified
- `play::main_menu::tests::custom_page_picks_a_mod`: page toggle, mod costume wrap at its count, lock rule,
  list ends, args (`--chars 5,5 --outfits 0,1`, two `--slot-mod`), back from confirm, no-mods refusal.
- `mods::tests::lists_the_custom_roster` with `HST_MODS=~/repos/HST-MODS/out/mods`: all 87 listed.
- `--shot` of the menu with `mods/` → HST-MODS `out/mods` (`context/b40a/custom_page.png`): 87 mods on 4 pages,
  four cards with Fore! Phoebe, Get a Grip Emi, Out of Bounds Jasmine, Open Tee Mika previews, names, styles.
- `--shot` of a bot match with the menu's flags, Phoebe (costume 2) vs Jasmine (`context/b40a/match_two_mods.png`):
  both on court mid-rally.
- `tools/check.sh -p hst`: 67 pass, 1 fails: `mods::tests::rerigged_mod_plays_forehand_and_run` (sway check:
  local `context/mods/test_pc00` has no `HST_NOISE` extras — test data predating M1d, not this change; my
  `mods::load` change is type-only).

## Not verified / not 1:1
- Remaster-only screen: no original to compare; layout, list, keys (Tab / Select) are my design.
- Didn't press through the menu to Launch in a live window (no input injection into the floating window); the
  launch path is the existing B40d one, checked by the args unit test plus running the match with those args.
- Page shown is shared; with two humans on different pages' entries only the shown page's hands are drawn.
- △ shows raw TParam numbers, not letter grades: the disc's grades are a fixed per-character table in the MENU
  overlay with no known formula from TParam.
- Confirm screen has no portrait for a mod (name card instead); the original's portrait sheets only cover disc
  characters.
- Mods load on hover only (no preload), so the first hover of a mod shows an empty card for a moment.
- AI costume level for a computer mod = its costume index (M1c's choice kept).
