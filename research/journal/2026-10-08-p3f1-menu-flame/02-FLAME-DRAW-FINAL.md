# P3f1a — the menu flame drawn (2026-10-12)

## Ported (from the decompile of the effect and its config block)
- Motion: spawn speed 1.04 along the heading; per frame vel = (0.65·(1.04+s)·cos h, (1.04+s)·sin h), with the boost
  s = 1.3·(1 − d/180) (d = degrees off vertical); heading steered towards up each frame.
- Spin: angle starts at the heading, spin = %7 − 3 degrees/frame, ×0.96 per frame.
- Size: 64 px · scale, scale from 0.1 growing +0.13/frame to 1.56, ×0.96 once life < 14.
- Alpha: 128, fading over the last 14 frames (life·128/14). Life 18 + %24.
- Sprites: `MENU03.XB0` `f_s.tm2`, fire half (u 0..64), smoke half (u 64..128) for particles 20–23.
- Order per frame: draw (old state) → life−1 → update. The `rand()` draws are unchanged from P3f1, so `rng.rs`'s
  existing tests still hold.
- Placement: screen centre + the node's (−24, −136) + the effect's (32, 64) = (328, 152) on 640×448, i.e. behind the
  VS on the confirmation screen (`play/main_menu/flame.rs`, scaled like the rest of the menu).
- Run from P1's pick on the character select (made again on re-pick), stepped at 60 Hz while the select/confirmation
  is up; `main.rs` seeds the match from the `rand()` the menus left (`main_menu::flame::rand()`), falling back to the
  clock count only when a match starts without the menus.
- Checked: `--shot` of the confirmation screen (`HST_MENU=confirm`, `context/p3f1a/confirm_*.png`): fire and smoke
  rise and fade behind the VS. Test `flame::tests::rises_and_fades`.

## Not verified / not 1:1
- No side-by-side with the original: copy 3's PCSX2 window wasn't visible to hyprctl (`hyprctl clients` empty), so
  `tools/screenshot.sh` couldn't grab it; a PINE RAM search for the live effect object after the pick found nothing.
  Judged by eye against the decompile only.
- Colour registers of the particles not read; drawn untinted (white).
- Blend: plain alpha; the original may draw additively.
- `Blue.tm2` (the alternate sheet) is loaded by the original but when it's used wasn't found; unused here.
- Placement offsets read as relative to the screen centre (an interpretation of the node chain).
- 60-frame start delay is from P3f's logs ("~1 s"), not from code.
- Our menus' flow and timing differ from the original's (menus are ours to design), so the number of draws before
  the seed differs from a real run; it only changes which seed, not how it's used.
- Motion in plain f32, not `ps2` ops (rendering only has to look right); the draws are exact.
- Rotation direction (sign of the spin on screen) not checked against the original.
