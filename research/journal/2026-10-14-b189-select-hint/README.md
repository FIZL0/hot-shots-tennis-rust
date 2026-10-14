# B189 — select-screen hint pop-up (FINAL: superseded)

B189 (filed 2026-10-13 09:37, f3fdf4b8) asked to redraw the character select's plain hint line
"Tab / Square: N custom characters   Select / C: left or right hand" as a per-device pop-up.

The user removed that line at 22:05 the same day (9bc3e880, "drop the char-select hint line (legend covers it)").
Its job is now done by M14's legend (`play/main_menu/legend.rs`): on `Screen::Chars` it lists Character / Costume /
Hand (Select, View/Create, or the keyboard's bound key) and Filter (□ / Tab, only with own mods), in the glyphs of
the device last used (keyboard caps from `Bindings`, Xbox / PlayStation / Deck pads), switching live, hidden after
a touch (the select's hand pill and grid are tappable), laid out on the 4:3 screen's bottom strip spanning any
window (`Anchor::Span`). `legend_lists_what_each_screen_handles` checks □ and Select are listed exactly when handled.

So nothing to add: re-adding a hint pop-up would undo the user's later decision. No code changes.

## Not verified / not 1:1
- The original's `KeyAssign_CharacterSelect2` pop-up art is not used: the user replaced the hint with our own legend
  (menus are ours to design, PLAN 2026-10-12). Reopen as a new task if the original-style pop-up is still wanted.
- Live device switching with real pads / a touch screen: as M14 (none attached to unattended runs).
