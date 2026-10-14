# B304 — character select: per-game filter pills dropped (FINAL)

User 2026-10-14: the roster's "From <game> n" pills (one per mod source game, B296/B293) are gone; the filters
are All / Hot Shots Tennis / Mods only (just "Hot Shots Tennis" with no mods, as before).

- `main_menu.rs`: `Menu::sources` removed; `filters()` is 3 with mods, 1 without; `roster()` filter 2 = every mod;
  the pills row draws the three. □ / Tab cycles All → Hot Shots Tennis → Mods → All.
- Test `roster_filters` rewritten for three filters (mods from several games all under Mods; wrap after 3).
- Shots `HST_MENU=chars` at 960×720, 1280×720, 1680×720 (`context/b304/`): three pills, nothing else moved.

Seen in passing (pre-existing, not this task): at 1680×720 the bottom help line runs off both window edges.

## Not verified / not 1:1
- none (front-end, ours to design; no original to match).
