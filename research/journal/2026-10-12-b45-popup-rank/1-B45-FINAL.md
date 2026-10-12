# B45 — CPU rank on the score pop-up (FINAL)

- Bug: the point pop-up's plates (`play/popups.rs` draw) still had the P12b1 stand-in "COM rank row 0" ("Lv 5" for
  every CPU); B35 fixed only the serve panel.
- Original: the pop-up's plate draw (the per-player routine P12b1 lists: pill, face, slot label, rank) takes the rank
  label row from the same match-setup int array as the serve panel (setup block +0x88 + 4·i, i.e. 8 ints after the
  slot labels at +0x58) and skips the label when it is 0x11, then puts it at v = row · 24. B35 already proved that
  int is the COM's AIParam row rank / 17 for humans on slot 5 and both `1p3goodcpus` dumps
  (`hst-sim/tests/ai.rs bots_get_their_rows`).
- Port: the pop-up uses `g.players[i].ai.rank` for COMs and no label for humans, exactly like `play/panel.rs`.

## Not verified / not 1:1

- No side-by-side screenshot of the pop-up in the remaster vs PCSX2: the source int and the draw's v = row·24 lookup
  were checked in the decompile and the int's values against RAM (B35's test), and the pop-up uses the same rank
  texture and tint as the already-checked panel, so a shot would only repeat B35's check.
- Where the menus write the int in challenge mode (B35's open point) is still not looked at.
