# B40e1 — the menus' hand (FINAL)

## Finding
The front-end's pointing hand has no swing. It isn't a sprite: the menu overlay draws `MENU/MENU00B.XB0`'s
`menu/3d/cursor` (also in `MENU12A.XB0`), a skinned 3D left hand on the Biped rig (mesh bound to the L forearm,
hand and finger bones), lit, playing `cursor.ani` (1200 ticks, looped). The wrist stays put; the index finger and
thumb wag and curl now and then (screenshot bursts on the Options screen, `context/b40e1/run2/crops.png`). That's
why no RAM word or decomp branch for a hand x offset turned up: the pause menu's out-and-back `Hand` swing
(`play/menu.rs`, `hsm_yubi`) is the game overlay's and doesn't apply here. The Game Selection hand looked static
only because that part of `cursor.ani` is the still stretch.

The archive also has `cursor_ok.ani` (30 ticks; the hand jabs, probably on confirm) and `cursor_op.ani` (20 ticks;
the arm raised, probably on open).

## Port
`play/main_menu/cursor.rs`: builds the model with `effects::model`, turned so the fingertip points right with the
back of the hand toward the viewer. It's lit by its own directional and ambient light on render layer 8 and drawn
by an offscreen camera (orthographic, 0.3 × 0.15 box) into a 256×128 image. `cursor.ani` is ticked at 60 Hz and
restarted at its end. The layout's white-hand quads (rows, controls, mode, confirm; 64×32 at the old sprite's
spots) now use that image (`Tex::Cursor`), so the hand stays part of the 640×448 layout and goes through
`widescreen::place` like every quad. `draw`'s placeholder sine is gone (`hand_x = 0`). The coloured player hands on
the select stay sprites. Checked at 16:9 (Options) and 8:3 (main menu): `context/b40e1/run3/`.

## Not verified / not 1:1
- Lighting: the original's VU1 light direction and colours weren't read. The light is set by eye to give the
  same white-to-grey shading as the screenshots (rendering only has to look right).
- Camera and scale: the original's view matrix for the hand wasn't read. The box and its size are set by eye
  against the crops.
- `cursor_ok.ani` / `cursor_op.ani` aren't played: not checked in the game when they trigger. The idle
  `cursor.ani` loops all the time.
- The coloured player hands on the select screen are still sprites. Not checked whether the original draws
  them as sprites or as tinted copies of the 3D hand.
- Phase: the clip starts at tick 0 when the app starts, not when each screen opens. The original's restart
  point wasn't checked.
