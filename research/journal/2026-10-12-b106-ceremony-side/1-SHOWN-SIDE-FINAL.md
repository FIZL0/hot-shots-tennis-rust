# B106 — the ceremony's shown side by who is real on this machine

What `ceremony::plan` picked (P26c's captures, `2026-10-08-p26-match-stats/3-CEREMONY-FINAL.md`): the human side if
exactly one side has a human, else the winners; lost = shown ≠ winner (lost clip, odd banner row). Offline that is
the original's rule and stays. The gap was online: `g.humans` marks every taken seat (a peer's seat drives its
player through the lockstep), so an online 1v1 or 2+2 had humans on both sides and both machines showed the
winners — the loser's machine got the winner's VICTORY.

Fix: `ceremony::local(humans, mine)` — offline the humans, online only the players on this machine's seats
(`Netplay.lock.mine`, seat → player as `Pads::slot_of`: doubles seats 1P..4P = players 0, 2, 1, 3). `plan` takes
that. So: vs CPU → the local side, win or lose; 2/4 local on both sides → winners; online → this machine's side
(a 2+2 loss shows the local team's DEFEAT while the other machine shows its VICTORY); a machine with local players
on both teams → winners; a bot in an empty online seat doesn't count.

Test `ceremony::tests::shown_side` pins shown side, clip and banner row for vs CPU win/loss (singles, doubles with a
COM partner, two humans on one team), 2 and 4 local, online 1v1 and 2+2 from each machine, a split machine, and a
bot seat online.

Tests: `tools/check.sh -p hst` all pass except `net::tests::lan_lobby_by_mdns`, which found another worktree's LAN lobby
(parallel run) and passes alone.

## Not verified / not 1:1

- Online cases: the original has no online play; the rule is the user's (task line), checked by the unit test only,
  not in a live two-machine match.
- The match-end jingle (play.rs, at the verdict) still decides "the shown side won" from `g.humans`, so online the
  loser's machine hears the winner's jingle 3 over the DEFEAT banner. Left alone to keep play.rs untouched (other
  agents edit it); fixing it needs the same `local` there (the system that cues it would need `Netplay`).
