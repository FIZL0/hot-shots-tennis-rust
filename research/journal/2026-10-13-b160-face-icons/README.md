# B160 — hi-res face icons for every character

**State: FINAL.**

## What changed
- `previews.rs`: each bake job now has a second phase. Once the 384×480 preview has been read back, the baker
  finds its head square (`inspect::face_square`, B133's framing, unchanged), points the same camera and rig at
  that square (`inspect::sub_crop` turns the preview pixels back into a crop of the original's frame), draws it
  at `FACE` = 320×320, and saves it as `<preview>-face.png` next to the preview (disk cache, or OPFS in the
  browser through the same `read`/`write`). So disc and mod characters get the same framing.
  `RENDER` 5 → 6, so every cache bakes again. If a preview has no `-face.png` (the run quit between the two
  phases), that key gets baked again.
- Icons: `Icons` is keyed by `Who`, and the disc grid draws `Tex::Icon(Who::Disc(c))` (costume 0). Until a
  close-up is baked it falls back to the disc's `FACES` square. The mod page works as before.
  `previews::face(iso, mod, c, k)` builds the match panel's 40/64 face for disc and mod players. The panel draws
  a face as its whole image (`rect` None), so the 320-px canvas works as well as INPANE's 64×64.
- A mod that ships its own `previews/<costume>.png` has no close-up, so it still gets B133's crop of that PNG
  (`inspect::face_icon`).

## Checked
- `HST_BAKE=all`: 450 previews and 450 close-ups baked (one bloom costume was cut off by the `--shot` exit; the
  new missing-face check bakes it again). Contact sheet: `context/b160/faces0.png`.
- `--shot`s at 1920×1080 and 3840×2160 of the disc grid (`context/b160/grid-*.png`, `grid-4k-crop.png`), the
  custom page (`mods-*.png`, `mods-4k-crop.png`) and a doubles match panel with three disc players and a mod
  (`--slot-mod 3 fore_pc03_sam`: `match-*.png`, `match-4k-crop.png`). All faces are sharp at 4K and sit where the
  disc ones did.
- Test `inspect::sub_crop_is_the_pixels`. `tools/check.sh -p hst` passes, and so does the wasm32 `cargo check`.

## Not verified / not 1:1
- The icons are our own render, not the disc's `face_CC_OO.tm2` art. This is the task's intent: the front-end
  doesn't have to match the original. Framing is B133's, measured against the disc faces.
- `FACE` is a fixed 320 px: about 1.1× the grid face's pixels on a 2160-line window. On an 8K window the icons
  are drawn larger than the bake and look a little soft (marked `ponytail:`).
- No mipmaps: at 1080p the 320-px icon is shrunk about 2.2× with linear filtering. It looked clean in the
  shots.
- Browser OPFS was not run live; it uses the same `read`/`write`/`cached` paths as the previews, and the wasm
  build compiles.
