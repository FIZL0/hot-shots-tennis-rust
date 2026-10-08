# 22 — last bot-only recordings, P11 wrap-up

Two more hooked recordings with `HST_AI` row overrides, both bit-exact in `ai_rally.rs`:

- `HST_SINGLES=1 HST_AI=0:51:3,1:5:0 tools/pcsx2.sh python3 tools/record_ai_rally.py 8 6000
  context/fixtures/ai_rally_singles_base.bin 1 1,2,3` (`bots_singles`; rows 51 level 3 and 5 level 0, both BASE):
  6039 frames, 2678 draws, nothing dropped. Receive 2101 calls, BASE 6713. Reaches **BASE smash taken** (2) for the
  first time.
- `HST_AI=0:89:0,1:84:0,2:86:0,3:97:0 tools/pcsx2.sh python3 tools/record_ai_rally.py 5 4000
  context/fixtures/ai_rally_s05_weak.bin 1 1,2,3` (slot 5, level-0 doubles rows): 4036 frames, 3761 draws, 227
  records dropped (buffer full; each call is self-contained, so the rest still replay). Receive 1293, NET 3519,
  BASE 8267 calls.

Doubles coverage markers (temporary eprintln, all doubles fixtures): the give-way fires (20) but none of the
out-of-court return aims, the not-found short-landing walk, the not-found repick from the partner's record, the
net-zone hold (path mode 2) or the substate-3 wait countdown. Singles: still no out-of-court aims, plan-10 upgrade,
60-frame redraw, line-margin hold or chase arrival.

Why stop here: row/level overrides on the bot-only saves are exhausted (singles NET/BASE/ALL at levels 0–3, doubles
level 0 and the default rows). What's left needs a human side: balls hit wide or into the net on purpose, a human
partner (+0x264), so a slot 3/4 recording driven with vpad. P11k6/P11l6 (frame-exact replay through
`doubles_ai::step`) need the app's flight to be the game's path object (mode, line gap, unsettled hit counters),
so they wait on the ball path port. P11k4/k5/k6/l4/l5/l6 marked blocked; P11 with them.
