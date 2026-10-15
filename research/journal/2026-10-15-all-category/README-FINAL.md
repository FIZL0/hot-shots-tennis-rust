# B390 — character select opens on "All"

User request (2026-10-15): the character select opens on a new category, All, holding every character from every
category.

## What changed
- `play/main_menu/tabs.rs`: new tab id `ALL = 0` (the `Player::tab` default, so a fresh select opens on it); ids
  renumbered (Favourites 1, Hot Shots 2, mod games from 3). Tab order: All (the whole roster, Random first), then
  Favourites (while any), Hot Shots, one per mod game. Hot Shots only shows when there's at least one mod game tab;
  without mods it would be a copy of All, so All is the lone tab and ←→ walk it like before (B306).
- A tab gone (Favourites emptied, a game removed) falls back to All, not Hot Shots; emptying Favourites from its tab
  leaves the cursor on All at the same character.
- Navigation unchanged otherwise: ←→ past a row's end overflow to the next / previous tab, so All ↔ Mods wrap.
- Tests updated for the new order (`tabs.rs` tests, two `main_menu.rs` tests that drive the mods' own tab now set
  it explicitly). `tools/check.sh -p hst --bin hst main_menu`: 112 passed.

## Checked
`HST_MENU=chars` and `roster2-deep` shots (context/B390/): "All 96" is the first, filled pill; the footer reads
`« Hot Shots Tennis: Get a Grip   All   8 of 96   Hot Shots »`.

## Not verified / not 1:1
- None against the original: the front-end is ours to design (PLAN.md, user 2026-10-12) and the original has no All
  category. In the 4-column doubles layout the tab pills are squeezed small (already so before B390, one more pill now).
