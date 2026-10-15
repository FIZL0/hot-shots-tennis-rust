# B168: bright rectangle near the player on the indoor court (court 7)

## What it was
A lighter band across the near court behind P1 on court 7, about 14 % towards white, with edges that follow the
perspective. It is a world-space draw, not the HUD.

## How I found it
- `HST_SKIP` filters, passed to `tools/shot.sh` through an `HST_BIN` wrapper because `hyprctl` drops the env.
- Skipping the hole model `extra_h01_s1111` removes it.
- A temporary per-material skip narrowed it to material 7, the black box under the see-through floor: 4 walls and a
  bottom, y 3.95..13.2, the "void" the floor reflection sits in.
- `--shadows off` and turning off B161's mips left it unchanged.
- Turning off the guard for material 7 alone removes it.

## Cause
B157's guard band (`gs.wgsl` `GS_GUARD`) took the GS band 2048 ± 1280 as ±4 half-widths across, assuming 320 GS
pixels per 4:3 half-width. It dropped the box's bottom triangles, so near the camera the floor blended over the
blue clear colour instead of black.

The original's stage-7 GS dump (`../opentennis/context/b157/c07_draws.txt`, prim 279) draws the box as a black
`tstripx8`: 6 of its 10 triangles, bb x −813..1453, y 60..992. Projecting the box with the same frame's camera
(`c07.ram`, eye 0x1e7d20, forward/down rows, tan 0x1e7d64 = tan 10°):
- Bottom near corners (±14, 13.2, −26.5): 4.068 half-widths, 7.855 half-heights. That puts them at GS x ±1133,
  y +880, so 278.5 px per half-width and 112.03 lines per half-height.
- Top near corners (±14, 3.95, −26.5): 4.72 half-widths = 1315 px, outside the band. That drops the near wall and
  one triangle of each side wall: 4 triangles, leaving the 6 drawn.

## Fix
The horizontal limit is now 1280/278.5 ≈ 4.60 half-widths (`gs.wgsl`). The vertical limit, 1280/112, was already
right.

## Checked
- `context/b168/fix_07*.png`: the band is gone. The original (B157's `orig_s07s.png`, same view) has none.
- `fix_08*.png`: Wild Green's near foliage is still dropped, as in B157's comparison.
- B157's Grand Palais wall projects to ±1929 px, so it stays dropped.

## Not verified / not 1:1
- 278.5 px per half-width comes from one vertex pair in one dump: y fits 112.03 lines exactly, and x is symmetric
  about 320. It isn't read from the microcode's projection constant, so other courts weren't re-dumped.
- Not re-compared with a fresh PCSX2 screenshot at the same instant. B157's original shot of the same serve view was
  used.
- Courts 1–6 and 9–11 weren't re-shot. The looser band only keeps triangles that the old one dropped, so they can
  only gain triangles the original also draws.
