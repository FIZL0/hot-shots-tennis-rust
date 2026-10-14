# B263 — framed face icons

The "lobby player icons" are the character select's round face icons (the lobby's picks go through the same select,
B195); the lobby's own seat rows have no pictures. They sat on flat beige squares.

- `main_menu.rs` `face_frame`: each face (disc grid and custom page) in a round frame: stacked discs of our pill's
  outer stadium, a thin ink rim and white ring (1.5 units each, user: thinner than the first `box3` try) round a warm
  fill, the ring dark like the face where every costume is taken. The face fills the frame less 3.5 units.
- The Random card gets the same ink/white/fill rounded box (8-px corners) instead of a flat rectangle.
- Hit areas unchanged. Drawn in the 640×448 layout, so it scales with the select at any aspect.
- Shots: `context/b263/chars0.png` (before), `chars1.png`, `mods1.png` (first, thick), `chars2.png` (thin), `HST_MENU=chars|mods` via an
  `HST_BIN` wrapper.

## Not verified / not 1:1
- Our own design (front-end, PLAN: menus are ours); the original's select has square disc faces, no frames.
- A disc face not baked yet (the `FACES` sheet square) shows its corners past the round frame until it's baked.
