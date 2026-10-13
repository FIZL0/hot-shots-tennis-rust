# B177 — pause menu pills drawn from the wrong texture

**Cause**: B68 (a6c16bd0) pushed three more labels ("Leave the match?", "Yes", "No") in `play/menu.rs` `setup`
before `menu_art::pill_layers`, but kept `PILL = Own(14)`. So the pills and the dialog box were nine-sliced out of the
"Leave the match?" label, which gave the black bars with an "L" at the left end and part of a letter at the right.
The confirm rows were also one off (ASK showed "Yes", YES showed "No", NO showed the pill sheet: B178's white blob).
The source rects were right; only the texture index was wrong.

**Fix**: `ASK/YES/NO = Own(14..16)` and `PILL = Own(17)`, the labels in one `LABELS` const that `setup` uses. Test
`play::menu::tests::art_order` pins every index to `setup`'s order. New shot hook `HST_PAUSE_ASK=1` (with
`HST_PAUSE`) opens the confirm.

**Checked**: shots at 960×720 and 1680×720 of the pause dialog, the confirm and the pause Settings screen
(`context/b177/`, wrapper HST_BIN with the env, `--play --shot-at 8`). Pills are whole and the chosen one is bright
at both aspects. The pause Settings screen was already right: it takes the pill sheet as `art.last()`.

A peer session (s2) reached the same diagnosis independently. Its fix reordered `setup` instead; the result is the same.

## Not verified / not 1:1
- No shared popup helper with `mods_screen.rs ask_layout`. The pause screen draws through its own Quad pool
  (`menu.rs`), not the main menu's `Draw`, so sharing would mean porting the pause screen onto `Draw`. That is a
  bigger refactor in files other agents are editing. The pause dialog already uses the same style (box, pills 40 px
  apart, hand, hit rects), so it looks like the popup.
- Not compared against PCSX2: the pause dialog is our own art since B141 (front end: ours to design).
