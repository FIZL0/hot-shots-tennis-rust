# B428 — the COM's roster column opens in the section 1P readied in

Cause: once 1P readies, the next computer player's card is reset (`auto_com` / `Player::default`) and put on the
last pick's cell (`carry_pick`, B269), but its `Player::tab` stayed at the default, so `section_at` fell back to
the character's home section — a favorite readied from Favorites reopened in its own game's section.

Fix: `Menu::last_tab` remembers the section the last pick was readied in (beside `Menu::last`); `carry_pick`
(local) and the online lobby's `edit_bot` / `bot_added` give it to the COM's card. `section_at` still falls back to
the home section if the tile isn't in that section (e.g. Random readied, or the favorite was dropped).

Test: `tabs::tests::the_com_opens_in_the_section_1p_readied_in` (fails without the fix: Golf, 1 instead of
Favorites, 0).

## Not verified / not 1:1
- Not compared with the original: the roster's sections are our own front-end (no Favorites in the original).
- Not checked on screen with `--shot`; covered by the unit test of the menu state only.
