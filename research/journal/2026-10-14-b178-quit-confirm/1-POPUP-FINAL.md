# B178 — pause quit confirm as its own popup

B177 had already fixed the stray quads (wrong texture indices). What was left: the confirm drew "Leave the match?"
as bare text in the pause dialog's first row, Yes/No on the next pills, inside the 3- or 4-row pause box.

**Now** (`play/menu.rs` `layout`, `ask_y`): with the confirm up, the pause dialog's box and pills aren't drawn; the
confirm is a popup like `mods_screen.rs ask_layout` (Reload / Later): the screen dimmed (black, alpha 80), a box3
panel (160..480, 46 px above Yes to 16 px below No), the question as its 22-px title, Yes and No on pills 40 px apart
(the chosen one bright, the other dim), the hand by the chosen one. Clicks/taps/hover test the confirm's own two
pills (`step`'s `pill`). Test `confirm_return` pins the popup's geometry.

**Checked**: `--shot`s at 960×720 (4:3), 1280×720 (16:9), 1680×720 (21:9) with `HST_PAUSE=6 HST_PAUSE_ASK=1`
(wrapper `context/b178/hst-ask.sh` as HST_BIN, `--play --shot-at 8`) → `context/b178/ask_*.png`: popup centred,
title on top, both pills whole, No bright, no stray quads.

## Not verified / not 1:1
- No shared popup helper with `ask_layout`: the pause screen draws through its own Quad pool, the main menu through
  `Draw` (see B177's journal); sharing would mean porting the pause screen onto `Draw`. Same geometry and colours
  instead.
- Not compared with PCSX2: the confirm is ours (B68; the original quits without asking).
- Mouse/tap on the new pill positions only checked by reading the code (no pointer test drives `step`).
