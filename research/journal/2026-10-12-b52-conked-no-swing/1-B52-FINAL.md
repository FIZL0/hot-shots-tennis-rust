# B52 — a conked player can't swing (FINAL)

## The original
The player rally update (`research/fn.sh 349410`) switches on the player's mode byte (+0x3fa5). Mode 3 is the
body-hit state the body test puts the player in (P25/P25a). In mode 3 the button argument is never read: the only
thing that branch does is, with fewer than 2 players (practice), go back to mode 0 once the motion's frame reaches
its end. In a match (≥ 2 players) the player stays in mode 3 until the point's reaction, so it can't swing, dive or
whiff at all after being hit. The press's auto-approach (mode 1's +0x3f24) ends with the switch to mode 3.

## Port
- `play.rs press`: a press by a `bodyhit::standing` player is ignored (humans, the singles bot and the doubles AI all
  press through it).
- `play/bodyhit.rs struck`: the hit drops a pending press and its approach run, so no contact search starts later.

## Check
`tools/check.sh -p hst`: all pass except `net::tests::mod_looks_peer_to_peer_and_the_ready_gate`, which timed out
under the parallel run's load and passes alone. Unrelated to this change.

## Not verified / not 1:1
- No PINE recording of a press after a body hit: the rule is read from the decompile (the mode-3 branch never reads the
  button). It's a single branch, and recording would mean forcing a hit on a human player during a live rally.
- Practice's return to mode 0 once 0x2b ends isn't ported: the port has no practice mode (as in P25a).
