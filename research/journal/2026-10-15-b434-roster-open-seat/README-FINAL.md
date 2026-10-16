# B434 — Roster: an open seat draws empty, no Enter in the join hints

## What changed
- `roster.rs` `open_seat`: every column asking for a player takes the empty dashed card — a local roster's unjoined
  seat (as before), a seat held open for a device (`Dev::Open`, which counted as active and drew a full card with a
  preview figure and "Open"), and an online seat not yet picked by anyone (was a full card, "?" stats, the grid
  greyed under a "Waiting for a player" box; now just its number and "Waiting for a player" in the middle).
  The old grid overlay could no longer be reached and is gone.
- Dimmer: the dashes at 60 % of the seat's soft colour, the big number at alpha 45 (was 70), the prompt at 100.
- Join hints never show Enter (`legend::join_styles()` and `home::press_hint` lost their `keys` flag): roster and
  home device tiles read "Press ✕ to join". Enter still joins (input untouched). With no pads connected the hint
  falls back to "Connect a controller to join".

## Checked
Release `--shot`s in `context/b434/run2/`: `HST_MENU=chars` (2P–4P empty, 1280×720 and 1600×600),
`chars-online` (4P waiting, 1280×720 and 600×1000 portrait), `main` (home 2P tile "Press ✕ to join").
Note: `tools/shot.sh` runs `target/release/hst`; rebuild release first (run1 was a stale binary).

## Not verified / not 1:1
- Front-end design is ours (PLAN: menus needn't match the original); nothing to compare against the game.
- No shot of a live `Dev::Open` seat (a device dropped online with ✕): no `HST_MENU` sample has one; it takes the
  same `open_seat` path as the local roster's seats, shown above.
