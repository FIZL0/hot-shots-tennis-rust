# B176 — face icons framed on the rig

**State: FINAL.** Follows B160 (`../2026-10-13-b160-face-icons/`).

## Problem
B133/B160's head square was one fixed 100-px square (of the 384×480 preview) hung from the silhouette's top. Big
heads (Mifune, Miranda, Carol, Regis, the afro and hat costumes) didn't fit: chins, sides and hats were cut, and a
leaning head (Mifune) sat at the icon's edge. Contact sheet before: `context/b176/before.png`.

## Fix
- `inspect::head_px`: the rig's `Bip01Head` origin projected into the preview (`inspect::image_px`, the inverse of
  `sub_crop`, same frame maths as `sub_view`). In every disc and mod rig it sits at the jaw (overlay:
  `context/b176/marks0.png`).
- `inspect::face_square(img, Some(head))`: the head's top is where the opaque column over the bone (±12 px, gaps up
  to 3 rows, so a racket held above the head doesn't count) ends; side = `HEAD_FIT` (1.6) × (bone − top); top 7% of
  the side above the hair; centred on the mean middle of the opaque run through the bone over the head's upper 70%
  (pulls a leaning or turned head into the middle). No head bone or a mod's own preview PNG: B133's square as before.
- `previews.rs` passes the head pixel at the preview read-back; `RENDER` 6 → 7 so every cache bakes again.
- Tuned in Python on the logged bone pixels first (`context/b176/w/proto.py`, `proto2.png`). A crown-width term
  (widen for afros/box heads) changed almost nothing (`proto3.png`, `proto-mods2.png`), so it was dropped.
- Test `inspect::tests::face_square_fits_the_head`.

## Checked
- `HST_BAKE=all` at r7: all 140 disc costume close-ups (`context/b176/after-disc.png`) and 355 mod costume
  close-ups (`after-mods.png`): every head whole, hats included.
- `--shot`s: select grid (`context/b176/grid.png`, Carol/Miranda/Mifune row) and a doubles match panel with a mod
  (`--slot-mod 3 mods/hst-mods/fore_pc03_sam`, `match.png`).

## Not verified / not 1:1
- Framing is ours, tuned by eye against the disc's `face_CC_OO.tm2` faces (`context/b133/faces.png`): ours shows a
  little more margin round the head than the disc art. The front-end doesn't have to match the original.
- Very wide shapes still touch the round cut at the sides: Regis's cowboy-hat brims (costumes 5–9), the box-head
  mod (`m-hst…`) and a few long ponytails. The face and head are inside; only brim/ponytail tips are clipped.
- A mod whose rig has no `Bip01Head` falls back to B133's square (none in the mods folder do).
