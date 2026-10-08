# B37 server's spot after a change of ends — PART (agent continues)

Decompile reading so far:
- The serve placement (0x3449f0, on messages 0xc change-ends enter / 0xe serve enter) puts the server at
  x = court · stance(+0x1408) · facing; +0x1408 is copied from +0x140c, which the serve state machine sets to |x| at the
  toss (0x3522b0 state 2). The port's `serve_placement` + `stance` at the toss match this.
- +0x140c is forced to 3.0 only when 0x423040 is set: message 7 (leaving phase 0, match intro) and 0x1b (rematch);
  cleared on 0x12 (rally enter). Message order is in 0x323b58 (phase exit 7/0xd/0xf/0x13/0x18/0x1b, enter 6/0xc/0xe/0x12/0x17/0x1a).
- Doubles: placement sets +0x13f5, cleared on 0xf; while set (players > 1) the next placement is skipped, so the serve
  entry after a change of ends keeps the change-ends placement.
- match_s05 / 1p3goodcpus never have a server serving again right after a change of ends, so they can't show it.

Live check in progress: `research/b37_stance_probe.py` (slot 5, pokes stances 1.5/2.0/2.5 on players 1..3 and team 0
to 40-0 once the first rally starts, prints x/stance at each phase entry until after the change). Its first run had
not reached the change of ends after ~20 min (the 40-0 poke may not end the game quickly); rerun, perhaps poking the
point that ends game 1 instead. If the stance survives the change, the port already matches the code read and the
user's report needs another cause (look at what the port shows during `Phase::ChangeEnds`: the original places
everyone on 0xc as the phase starts; the port leaves them where the point ended until `next_point`).
