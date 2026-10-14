# B263 — framed face icons

The "lobby player icons" are the character select's round face icons (the lobby's picks go through the same select,
B195); the lobby's own seat rows have no pictures. They sat on flat beige squares.

- `main_menu.rs` `face_frame`: each face (disc grid and custom page) in our round box (`box3` with corner = half
  the size, i.e. a circle): warm fill under the face, ink rim and white ring over its edge, the ring dark like the
  face where every costume is taken. The face is drawn at 0.8 of the frame.
- The Random card gets the same ink/white/fill rounded box (8-px corners) instead of a flat rectangle.
- Hit areas unchanged. Drawn in the 640×448 layout, so it scales with the select at any aspect.
- Shots: `context/b263/chars0.png` (before), `chars1.png`, `mods1.png` (after), `HST_MENU=chars|mods` via an
  `HST_BIN` wrapper.

## Not verified / not 1:1
- Our own design (front-end, PLAN: menus are ours); the original's select has square disc faces, no frames.
- A disc face not baked yet (the `FACES` sheet square) shows its corners past the round frame until it's baked.
