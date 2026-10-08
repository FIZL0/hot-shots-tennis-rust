# B35 — serve panel's rank label for CPUs (FINAL)
- Bug: the panel drew rank row 0 ("Lv 5") for every COM player (a stand-in in `play/panel.rs`).
- Original: the panel's per-player draw (two variants, singles/doubles layouts) takes the rank label's row from a
  per-player int in the match setup globals (4 ints after the slot labels; the slot labels are 4 = COM) and skips the
  label when it is 17. In RAM that int is the player's AIParam row's rank byte (段位 % 17, record +0) for COMs and 17
  for humans: slot 5 bots rows 84,86,85,89 → 0,0,0,1; `1p3goodcpus` / `1p3goodcpus2` (B34 capture) Carol human → 17,
  Will/2/10 in outfits 9/4/9 → rows 137,128,136 → 15,12,14.
- Port: COM players show `g.players[i].ai.rank` (the row already chosen from character + outfit); humans none.
- Test: `hst-sim/tests/ai.rs bots_get_their_rows` now also checks the panel rank int against the table for both RAM
  dumps. Not looked at: where the menu writes the int (challenge mode may set it differently).
