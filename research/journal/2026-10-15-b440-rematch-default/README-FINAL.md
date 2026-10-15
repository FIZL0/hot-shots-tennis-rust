# B440 — after a match the main menu lands on Local, not Rematch

User 2026-10-15: back from a match, the highlighted row should be Local, not the Rematch card.

- The menu `setup` that runs when a match ends (`!fresh`, kept `Menu`) set `sel = 0`. Once a local match has been
  played, row 0 is the Rematch card, so it ended up selected.
- New `Menu::after_match` (`play/main_menu/home.rs`) sets the main screen with `sel = main_at(Row::Local)`.
  `setup` calls it in place of the `(Screen::Main, 0)`. The online case (screen `Lobby`) is unchanged.
- Test `a_match_comes_back_on_local`: Local is highlighted and Rematch is still the top row. `rematch_replays_the_setup`
  still passes.

## Not verified / not 1:1
- The front-end is ours (PLAN.md), and the original has no Rematch card, so there is nothing to compare.
- No `--shot`: `HST_MENU=rematch` opens a fresh menu, not the return path from a match. The unit test covers that path.

## Also
- `update::tests::versions_in_semver_order` was already failing on main: 17d2c0d1 renamed alpha to beta everywhere, and
  the test's order ended up with `-beta.10` before `-beta`, which isn't semver order. Its last pre-release is now `-rc`
  (test data only).
