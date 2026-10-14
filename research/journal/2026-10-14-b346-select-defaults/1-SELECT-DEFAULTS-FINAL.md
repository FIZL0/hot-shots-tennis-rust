# B346 — select opens on Random; COM cards keep the last character

- Cause 1: every entry to the select reset players to `Player::default()` (grid index 0 = Ashley, `random: false`).
  `Player::fresh()` (on the Random card) now seeds `local_start`, `duo_start`, the assignment screen's ✕ and
  `swap_mode`'s newly opened seats. `Player::default()` itself is unchanged (online lobby picks, tests build on it).
- Cause 2: B269's `carry_pick` only moved COMs 1P had *added* (`bots[q]`). B331's local start leaves the other
  seats open, and their COM cards (B258) took `auto_com` — the first character nobody has — so the next pick jumped
  to the next character. `carry_pick` now covers any not-ready computer seat: it lands on the cell just readied
  (`skip_taken` → that character's next free costume; Random stays Random). ○ on an open COM card still hands it
  to the computer's automatic pick.
- Tests: new `select_opens_on_random_and_coms_keep_the_character` (doubles: 1P on Ashley → COMs Ashley costumes
  2/3/4). Updated `an_open_seat_is_a_com_card`, `locked_costumes_and_match_order`, `select_swaps_the_hand`,
  `costume_pack_extends_a_disc_character`, `one_roster` for the new defaults (a step right from Random to Ashley,
  carried COM picks).
- `mods_screen::upload_all` timed out once in the full main_menu run (packing threads under load) and passes alone;
  unrelated.
- This also covers B319 (same request, open in TODO).

## Not verified / not 1:1
- Front-end behaviour, ours to design (PLAN.md); not compared with the original.
- No `--shot` taken: only the cursor's starting cell changes (the Random card's existing look); checked by tests.
