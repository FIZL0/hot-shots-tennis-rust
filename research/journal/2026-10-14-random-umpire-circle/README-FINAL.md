# B356 — random umpire's circle on the confirm card

The only place the umpire is drawn as a round face is the confirm screen's side card. Since B350, a disc umpire there
is the disc's own round face with its built-in dark rim and white ring. "Random Umpire" was the square "?" tile with
no circle.

Fix (`main_menu.rs` `random_face`): for Random, draw an ink disc and a white ring inset 0.7 units, both stacked pill
discs like `face_frame` uses. On top goes the tile's middle square, clipped round with `Quad::round`, so it fills
the circle inside the ring the way a disc face does. The sizes come from measuring Chika's face in a `--shot`
(`HST_MENU=confirm`, with `umpire = 0` in a settings.txt next to a symlinked ISO in `context/B356/iso/`). Looked at
side by side in `context/B356/cmp.png`, and they match.

Checked: the umpire picker strip draws every umpire, Random included, as the same square card, so it needed no
change. Online's tile there is the court's, not the umpire's.

## Not verified / not 1:1
- The rim and ring widths were matched by eye against the disc face, not measured exactly. Menus are ours (PLAN.md).
