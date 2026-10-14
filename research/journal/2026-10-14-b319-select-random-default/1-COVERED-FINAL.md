# B319 — select opens on Random; COM cursor starts on the last pick

Already done by B346 (commit c318f182, journal `2026-10-14-b346-select-defaults`): `Player::fresh()` puts every
seat on the Random card when the select opens, and `Menu::carry_pick` starts each next not-ready computer seat
(open COM cards and added COMs) on the cell just readied, moved to that character's next free costume
(`skip_taken`). No code change here.

- Checked: `select_opens_on_random_and_coms_keep_the_character` (crates/hst/src/play/main_menu.rs) passes on
  this branch — all four cursors on Random at entry; after 1P readies Ashley, COMs 2–4 start on Ashley costumes
  2/3/4.

## Not verified / not 1:1
- Front-end behaviour, ours to design (PLAN.md); not compared with the original.
- No `--shot`: only the starting cell changes; covered by the test.
