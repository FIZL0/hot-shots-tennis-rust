# 01 — flare drawn, compared by eye — FINAL

First port shot (`cmp_poke2`) was far too bright/wide: vertex colours were interpolated in linear light then encoded.
Splitting each kite into 6 bands with the gamma value per band edge fixed it; `cmp_up2_crop.png` (same view, same sun)
now shows the same white core, ray count/spread and ghost chain. The port's rays read a little longer and softer.

Read live (slot 5, court 10): 0x7f8 = 10, 0x7fc/0x800 = 500/400, 0x804 = 611.78, 0x808 = 133.33, el 0.878.

## Not verified / not 1:1
- Occlusion fade (the game's GS read-back of the sun's pixels) not ported: fade is 1 whenever the sun is in view, so the
  flare shows through players/props (in `up_2` the original hides it behind a head). Needs a depth read-back; render-only.
- Mode constants: the boost scale (0x808 = 133.33) and ray size (fr·(150 − 500) + 500) are slot 5's court-10 values; the
  per-mode table's 0x804 (611.78) doesn't fit any mode's (0x7fc, 0x800) pair exactly, so other courts may differ slightly.
- Ray y offsets are field lines in the game (224-line field); treated as circles in view tangent space — by eye only.
- GS Gouraud approximated by 6 bands per triangle (small steps, not visible in shots).
- `flare3.bmp` used as colour with alpha 1 (grey = intensity) — assumed from additive blending, sprites only checked by eye.
- Whiteout quad not compared against the original (no capture reached c·4096 > 3712).
- Comparisons used a poked sun direction (lighting unchanged), one view (up_2) plus poke_2; not every court/weather.
- `tools/shot.sh` (hyprctl) shot of the same view missed the flare once; the direct `--shot` run is the one compared.
