# P13 — umpire lines still to record (READY: needs the human)

P13's port matches the original on every tick of one recorded doubles bot match (`umpire_s05.bin`, court 10,
umpire 4, voice set 0, English). Some of her calls never came up in that match, so these code paths are untested:

| Line | Program/key | What makes it happen |
|---|---|---|
| Let | prog 1 key 3 (judge call 4) | a serve clips the net and lands in |
| Set announce | prog 2 key 2 (+ key 11/12) | a set ends without ending the match (needs a match of 2+ sets) |
| Tiebreak announce | prog 2 key 0 | a set reaches the tiebreak (6–6); she calls it at the next serve |
| Deuce again | prog 0 key 13 | a second deuce in one game (deuce → advantage → deuce), on any court but 5 |
| Match | prog 2 key 4, then match over | the match ends; keep recording until the result screen |
| Replay tiebreak | prog 2 key 0 again | an instant replay of the first tiebreak serve, if replays show serves |

Also unknown: how the game picks the umpire (UV00..04) and voice set (A..D) for each match. Two or three
matches on different courts or with different settings, each with the live ids, would settle it.

## How to record
1. Set up a match that will produce the lines above: a long one (several sets, no-ad off) gives deuce, set and
   tiebreak lines. A human player can force a let by serving into the net tape. Bots are fine for the rest.
2. Save state at the match's first frame, before the opening serve (the test expects the match entry: phase 0).
   Use scratch slot 8 or 9, not 3/4/5.
3. From the main checkout:
   `PYTHONPATH=tools tools/pcsx2.sh python3 tools/record_umpire.py 8 <frames> context/fixtures/umpire_s08.bin`
   (60 frames a second; a 3-set match is about 60000–100000 frames). It stops on its own if the game stalls
   for 10 s. Check for "missed frames" lines: a few are fine, long gaps aren't.
4. Note the court, the scenarios you saw and roughly when, and the match settings, in this journal.

## Then (agent)
Generalise `umpire_s05` in `crates/hst-sim/tests/umpire.rs` to run over every `context/fixtures/umpire_*.bin`.
Its court is currently hard-coded to 0, so pass the real one; this matters for deuce-again on court 5. Assert
that each new line appears at least once, and add a recording for match over (phase 5 →
`Umpire::call_match`/`match_over`).
