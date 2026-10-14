# B350 — confirm screen's umpire: name only, one icon

The confirm screen's side card (`confirm_screen` in `play/main_menu.rs`) drew "Umpire Lily" and passed the disc's
umpire face (`Confirmation_00`, already round with its own rim) through `face_frame`, which adds our ink/white ring
and warm fill round it: a ring inside a ring.

Now the face is drawn on its own (26 units tall, its 46×44 aspect kept) and the label is just the name ("Lily").
Random draws the "?" tile's middle square, cropped 4 texels inside its top/bottom edge lines (else they showed as
stray strokes), labelled "Random Umpire" (like the band's "Random Court": a bare "Random" said nothing).

Checked with `HST_MENU=confirm-court` / `confirm` shots (`tools/shot.sh` through an HST_BIN wrapper), before/after in
`context/b350/`. The card is in the menu's 640×448 units, so it scales with the rest at any aspect.

## Not verified / not 1:1
- Menus are ours to design (PLAN.md); no comparison with the original's confirm screen needed.
- "Random Umpire" wording for the random pick is our choice (the task only named the real-umpire case).
