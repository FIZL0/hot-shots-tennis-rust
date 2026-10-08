# P3e7 gallery manager messages

## Decomp (manager 0x39c440, walker 0x39efe0 / 0x39ece0, scoreboard 0x38c550)

- Match start (msg 6): the walkers' handler runs **before** the manager's (recorded, below). Each walker (0x39ece0)
  zeroes +0xd8..+0x310, faces the court centre, and with players 0x422fa4 > 1 sets anim 5 at a random frame in its
  slot's sixth of the anim (0x39f670: one court-generator draw), counter −1 (else counter 0); then it stores the
  manager's count in +0x304 and registers a mark at its home (0x39ca70, kind 0, delay slot·3). The manager then
  sets count 0 and (old phase ≠ 1) zeroes everything (0x39cb60, run flag kept); with players > 1: run, one
  0x39cdb0 draw, court 5 init (93 marks).
- First new point (msg 0xe, old phase 0): walkers mode 0, counter 0 if players > 1 (no 0x39ece0); manager count 0
  (no reset: only after old phase ∉ {0, 1}). msg 0xe after a point calls 0x39ece0 (counter 0 always).
- msg 0x18 (point-over exit): count 0. Reset vs count-0 is never visible: a cleared mark is only shown again after
  0x39ca70 or the court 5 init rewrites it; kind is rewritten by both.
- Point over (0x17) on court 5: init when call +0x426 == 0 and (0x4230b8 > 0 or +0x560 ∈ {1, 2}).
  +0x560 (0x38c550): 1 when the rally had 1 shot (or 2 on one called branch) and 0x38d6f0(server) — the point
  wasn't lost by the server's team; 2 when 2 shots and 0x38d6f0(last hitter); forced 0 when the ball state
  0x3165f0 == 4. With call 0 that is "≤ 2 shots and the last hitter's team won": an ace or a return winner.
  0x4230b8 is 1/2 for a game/set won but not set by the match's last set.
- Match over (0x1a), players > 1: 0x39dcd0 (favoured team won, = `sound::favoured`); unfavoured → reset, favoured
  on court 5 → init; always run = 1 (resume). The walkers react to it like a point (mode 1, counter −1).
- Type 44 msg 8: still no sender found (no static caller; the messages go through vtables).

## Recordings

- Court 4 doubles start (`research/p3e7_watch.py`, vsync 7029–7030): count 0 → 4 (walkers 0–3, delays 0, 3, 6, 9,
  +0x304 = 0..3) then 0 in the same tick: the walkers first. Walker counters −1, anim 5.
- Select Court: internal court 5 is the menu list's 11th entry (two left of the first cursor;
  `research/p3e7_stages.py`: −2 → 5, −1 → 1, +1 → 6, +2 → 2, +4 → 9, +5 → 11, +6 → 10; the 5th entry "Wild Green"
  is court 8). Court-5 match: `research/p3e7_court5.py`, log `context/p3e7/court5_match.txt`.

Court-5 doubles start (P1 pressing ✕): count 0 → 93 (kind 1) at vsync 6692 in the start phase (the walkers' 4 and
the reset fall in one sampled tick), 0 at the first point (6913). Over 47 points (`research/p3e7_tally.py`), the
93 marks went up exactly where `call 0 and (0x4230b8 > 0 or +0x560 ∈ {1, 2})` predicts:

| call | +0x560 | 0x4230b8 | points | marks |
|---|---|---|---|---|
| 0 | 0 | 0 | 12 | no |
| 0 | 1 (ace) | 0 | 3 | yes |
| 0 | 1 (ace) | 1 (game) | 3 | yes |
| 0 | 2 (return winner) | 2 | 2 | yes |
| 0 | 3 | 0 | 2 | no |
| 2 (fault) | 0 | −1 | 17 | no |
| 3 (double fault) | 0 | 0 | 7 | no |
| 3 (double fault) | 0 | 1 (game) | 1 | no — the port showed them before |

Match over (vsync 38594–38752): the last point (return winner) put the 93 marks up; phase 5 then count 0 (msg
0x18) and 61 vsyncs later kind 0 too: the reset of an unfavoured team's win (msg 0x1a); run stayed 1. Which team
was favoured wasn't logged. The next match (attract, court 2) started at 44211 with count 1, 4, 0 in one tick.

Not reached: a game won by a plain rally point (both 0x4230b8 > 0 points with marks also had +0x560 1/2), a
favoured win on court 5, and a deciding tiebreak (type 44's msgs 4/8).

## Port

- `npc::Walker::start` (match-start cheer and draw), called per walker at spawn with its mark; then
  `Cheers::clear` (now the full reset) and court 5's init. The match's first serve restarts the idle loops
  (`new_point(players > 1)`), resets the tick and clears the marks (`play/npcs.rs`, `started`).
- `npc::court5_again(call, game, shots, hitter_won)` at the decided point (call re-judged from the rally; game =
  game/set won short of the match end); `Cheers::match_over` + resume at the serve after the match's last point
  (the port has no match-over phase: the next match starts at once and its new point zeroes the marks again).
- Left: the walkers' cheer at match over and the next match's start cheer (no match-over phase in the port);
  the ball state that voids +0x560; type 44 msg 8's sender.
