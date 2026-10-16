# B424 — Roster footer states the whole cast

Each roster column's footer now reads `8 of 16 in Hot Shots Tennis · 95 characters` (`tabs::foot`): the tile's
place in its section, then the total cast (disc characters + own mods, the Random card not counted, a favorite
counted once). When the whole line is wider than the column (4 players in a portrait window) the section name is
dropped (`8 of 16 · 95 characters`; the rail above names the section) instead of squeezing the text unreadably.

Test: `tabs::tests::the_footer_states_the_whole_cast`.
Shots (context/B424/run1, run2): roster1/roster4/roster4-favs at 16:9, roster4 at 4:3 and 540×960 portrait,
roster2-favs at 21:9: the long line fits every 16:9/4:3 column, portrait 4P shows the short one.

## Not verified / not 1:1
- Menu screen, ours to design (PLAN): no comparison with the original needed.
- Section count still includes the Random card in Hot Shots Tennis (same as the rail chip, "16"), while the total
  doesn't: so the two numbers agree with what's on screen (tiles vs characters).
