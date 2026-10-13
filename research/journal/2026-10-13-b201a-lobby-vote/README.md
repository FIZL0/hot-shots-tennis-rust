# B201a — lobby voting

The host opens a vote from the lobby's **Vote** button: Court (the confirm screen's strip order + Random Court),
Umpire (five + Random) or Singles / Doubles. Every peer gets a panel of options with live ballot counts and the
seconds left; OK votes (and can change the vote), the host also has End Vote.

- `net.rs`: `Room.vote: Option<Vote>` (topic, options, ballots by peer, seconds left), `Msg::Vote`, `Cmd::OpenVote /
  Vote / EndVote`. The host counts down on its 1 s ping tick and re-sends the room each second; the vote closes when
  every peer in `names` has voted, at 0 s, on End Vote, or at Start. `Vote::result`: most ballots; a tie (and
  Random) is broken by an FNV seed over the ballots, so any peer computing it gets the same pick (the host applies it
  and broadcasts the room). Singles only applies if the people seated fit in two seats.
- `online.rs`: Vote button, topic panel, vote panel (taps too), `HST_MENU=lobby-vote|lobby-topics` sample shots.
- `hst net … --vote C`: the host opens a court vote once every seat is taken; every peer votes court C.

## Verified
- `net::tests::three_peers_vote_a_court_and_start_on_it` (host + 2 guests over iroh endpoints, ballots 7, 7, 2):
  every peer's Start room has court 7. `votes_close_on_the_winner_ties_seeded`: majority, seeded tie, random, mode.
- Three processes: `hst net host doubles 2 --relay off --vote 7`, `hst net join <code> --relay off --vote 7`,
  `… --vote 2`: all three print `Start(Room { … court: 7`.
- Shots: `context/b201a/lobby-vote.png`, `lobby-topics.png`.

## Not verified / not 1:1
- No original to match: online lobbies are ours (front-end, PLAN.md).
- Not run in the browser build or across two machines (only native, one machine).
- B201 (the shared live-pick lobby) wasn't merged yet; this sits on today's lobby screen. When B201 reworks the
  lobby, the Vote button/panels need carrying over (they're self-contained: `TOPICS`, `vote_options`, the two
  `Screen::Lobby if …` arms and the two panel blocks in `layout`).
- "Host decides which ones are votable" is the host choosing the topic when opening the vote; no per-option
  allow-list.
- `tools/check.sh -p hst`: 225 pass; `mods::tests::mod_eyes_follow_the_donors_uva` and `modfs::tests::real_mods_install`
  fail on a mod in the external `../HST-MODS/out/mods/fore_pc00_phoebe` (its mod.json's `costumes` path is outside the
  mod folder): state of that repo, not this change (no mods code touched).
