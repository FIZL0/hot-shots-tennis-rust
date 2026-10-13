# B135 — the menu's 3D hand drew wrong (FINAL)

## What was wrong
`effects::model` builds every effect material as `AlphaMode::Blend` (the match's effects need it). Blended
materials don't write depth, so the hand's palm, curled fingers and back faces were drawn over its front in
triangle order: a white hand covered in grey speckles, with a striped index finger and holes in the fist. On top
of that it was lit too brightly (4000 lx + 250 ambient, no tonemapping), so the lit side came out flat white.

## Fix (`play/main_menu/cursor.rs`)
- The hand's materials are made opaque (depth written, no depth bias), so it draws as one solid model.
- Light: one directional light from the front and above (800 lx) with ambient 50, which gives the original's
  mid-grey hand with darker edges and a lighter back (it was flat white before).
- Size: the camera box goes from 0.30 to 0.35 wide, so the hand is about 38 px of the 640 layout like the
  original's (about 37 px on the Options screen; it was 44). The box is moved so the fingertip still touches the row.

Original reference: Options screen of PCSX2 copy 4 (slot 2 → ○×3 → main menu → Options), 7 screenshots across
the clip in `context/b135/orig/e*.png`. Remaster before/after: `context/b135/run1/menu.png`,
`context/b135/run2/t0.png`; side by side (original | before | after): `context/b135/compare.png`.

## Not verified / not 1:1
- Lighting and colour are matched by eye (rendering rule). The original's VU1 light vectors and colours weren't read.
- View angle: the original shows the hand from the side, as the port already did. Tilts of ±20–75° were tried and
  matched worse, so the turn wasn't changed. The original's view matrix wasn't read.
- The original's hand looks half-height and aliased (field rendering in PCSX2), with dark pixels at its edges.
  The port draws it smooth, at full resolution.
- Only the main menu was checked after the fix. Every other `Tex::Cursor` quad uses the same image, so they get the same change.
