# B443 — "Wish list" → "Saved mods"

User-facing text on the Mods page (`play/main_menu/mods_screen/grid.rs`):

- Rail Show filter: "Wish list" → "Saved mods".
- Detail-panel □ button: "Wish list" / "Off wish list" → "Save" / "Unsave"; button legend □ "Save".
- Detail pill: "On wish list" → "Saved".
- Status line: "X is on/off your wish list." → "X added to/removed from your saved mods."
- Doc comments and the legend test in `mods_screen.rs` follow.

Kept: the file `mods-wishlist.txt` (and localStorage key) so lists people already saved carry over; internal
names (`Show::Wish`, `wish`) aren't shown. `mods_kind.rs` had no mention.

Tests: `tools/check.sh -p hst --bin hst mods_screen` — 16 passed.

## Not verified / not 1:1
- No screenshot taken: text-only change; the shorter labels fit wherever the old ones did.
- Front-end is ours to design (PLAN), so nothing to compare against the original.
