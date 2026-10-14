# B361 — Singles/Doubles follow the players on the local roster; non-scrolling roster mockup

## What changed
- `play/main_menu.rs`: the local roster (B331) no longer swaps Singles ↔ Doubles by hand: □ / Tab and the tap on
  the mode bar are gone (the bar only shows the mode), and so is the legend's "Singles / Doubles".
  - A local match always opens in singles (`local_start` resets `doubles`, so a rematch's doubles doesn't carry).
  - One or two devices: singles. A third device's ✕ (`can_join`: no free seat, local, singles) turns it into
    doubles via `swap_mode(keep = true)`: 1P stays, the singles 2P goes across the net to 3P, the new device takes
    1P's partner (2P), 4P is the computer. Humans' ready picks stay ready.
  - A device leaving (`drop_seat`) with two or fewer left goes back to singles (`swap_mode(false)`: 1P and the
    first other human, opponents first; every card picks again, as B331's swap did).
  - The legend's "Join" for an unseated device shows in that case too (`can_join`).
- `play/main_menu/home.rs`: the Local card's facts say Doubles "3-4 players" (was 1-4).
- Online (host's Mode screen), the phone duo and the paused-match assignment are unchanged.
- Tests updated: `one_roster` (□ no longer swaps), `a_dropped_device_frees_its_seat`, `four_humans_any_device_any_seat`,
  `locked_costumes_and_match_order` (join → doubles, leave → singles), `select_opens_on_random_and_coms_keep_the_character`
  (sets doubles directly to keep testing three COM cards), `pointer::taps_from_main_menu_to_match`.
  `tools/check.sh -p hst --bin hst main_menu`: 86 passed.

## Smash-style cursor TODOs
None open: the only one was B331 (already in `plan/DONE.md`, implemented); nothing in `plan/TODO.md` asks for
Smash-style per-player cursors, so nothing was removed.

## Mockup: roster that never scrolls, every seat shown
`context/b361/mockup/{singles,doubles}.png` (+ `.svg`, generator `gen.py`), against today's select with the
user's 92 mods (`context/b361/now-chars*.png`: 5 rows of 19 on view, a scroll bar).
- Disc row on top (Random + the 14) at full tile size, one row.
- Every mod below in a denser grid, grouped by source game under a thin header ("Hot Shots Golf · 30"); the tile
  size is the biggest that fits all of them in the space left (no scroll bar ever; name strips drop below a
  legibility size and the card shows the name instead).
- Mode pill top right is a status, not a button, with the rule beside it ("A 3rd player joining makes it Doubles" /
  "Down to 2 players: back to Singles").
- Seats: singles shows 1P vs 2P plus a dashed 3P · 4P panel ("press ✕ / Enter on another pad to join — the match
  becomes Doubles"); doubles shows all four cards, the CPU's greyed.
Needs the user's go-ahead before it's built (plan/HUMAN.md).

## Not verified / not 1:1
- Not checked against the original: the local roster is a remaster-only screen (PLAN: front-end is ours to design).
- No `--shot` of the new behaviour (a third device joining needs live pads); covered by the unit tests only.
- The mockup is a hand-drawn SVG (placeholder faces/models), not rendered by the app.
