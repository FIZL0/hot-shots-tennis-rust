# B490 — online lobby: no NEAR SIDE / FAR SIDE in singles

User 2026-10-16: each client keeps its own player on the near side, so in an online singles lobby the side labels
mean nothing.

- `frame::stage` skips a label that is empty (no pill drawn), so both screens can drop them.
- Lobby (`online/lobby.rs`): singles passes `["", ""]`; doubles keeps TEAM 1 / TEAM 2.
- Confirm (`confirm.rs`): the same in online singles (`m.online()`), where the same reason holds; local singles
  keeps NEAR SIDE / FAR SIDE (there the sides are real).
- New shot `HST_MENU=lobby-singles` (the sample lobby as singles, two seats) for checking it.

Checked: `tools/check.sh -p hst lobby` (8 passed, incl. any-aspect layout); `--shot`s
`context/B490/lobby-singles.png` (two plates, VS, no pills) and `context/B490/lobby.png` (doubles, TEAM 1/2 still).

## Not verified / not 1:1
- The online confirm screen in singles wasn't shot (no `HST_MENU` sample puts confirm online); the change there is
  the same one-line label choice as the lobby's.
- Front-end is ours (PLAN.md), so nothing to compare against the original.
