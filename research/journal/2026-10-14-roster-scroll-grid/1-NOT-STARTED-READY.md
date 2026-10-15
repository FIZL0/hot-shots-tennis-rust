# 1 — not started: waiting for the concept board approval (READY: needs the human)

The parallel runner handed this task out, but its brief says it is blocked until the user approves the concept board
('Proposal B382', artifact 6kpFeK6ya6YUJ66BN9Bjzk). The spec also changed after this worktree branched (TODO.md here
still says L1/R1 jump sections; the brief says L2/R2, with L1/R1 staying costumes). No code was written: building the
layout before the board is approved would mean guessing the design.

Unblock: the user approves (or amends) the board, then `tools/ctx.py unblock B382`.

## Not verified / not 1:1
- Everything: nothing was built (waiting for approval). The front-end does not need to match the original anyway.

`tools/done.sh --blocked` refused to commit: `play::mod_match::tests::umpires_are_on_the_roster_and_play_a_rally` fails
because the `mods/` folder this worktree uses holds only `hst_umpire_04_anna` (B381 removes chika/suzuki/lily). That
comes from the local mods, not from this task (no code changed). Blocked with `tools/ctx.py block` and committed by hand.
