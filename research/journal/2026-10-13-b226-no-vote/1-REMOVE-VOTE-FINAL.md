# B226: lobby voting removed

User 2026-10-13: the host alone picks court/mode/umpire. B201a's vote is deleted, not hidden (git history has it:
34365d51 and earlier B201a commits).

Removed:
- `net.rs`: `Room::vote`, `Topic`, `RANDOM`, `Vote` (tally/result), `all_voted`/`close_vote`/`ballot`,
  `Msg::Vote`, `Cmd::OpenVote/Vote/EndVote`, the host's countdown on the ping tick, the CLI's `--vote`, and the
  `votes_close_on_the_winner_ties_seeded` test.
- `play/main_menu/online.rs`: the host's Vote button (five host buttons now), the topic list, the vote panel with the
  live tally, `View::topics/ballot`, tap targets, the `lobby-vote`/`lobby-topics` sample menus (also in
  `main_menu.rs`), the `lobby_vote` test.

Kept and checked: the host's Court button (`Cmd::Venue`). `net::tests::three_peers_see_the_hosts_court_and_start_on_it`
(was the vote test) now has the host set court 7 with `Cmd::Venue`, both guests see it in their `Room`, and all three
start on court 7. `lobby_taps_hit_items` updated for five buttons.

## Not verified / not 1:1
- none (front-end only; no screenshot taken since the layout just loses a button at the same pitch).
