# P3f5 — lens flare drawing

The flare's 24 rays used to be only stepped (their `rand()` draws kept the sim RNG in step); now they are kept and drawn.

- `hst_sim::rng::Flare` (in `Rngs`): setup draws `bright = (rand()%100)/100`, `angle = π·((i+1)·4096/24 + rand()%85 − 42)/2048`;
  each tick `bright += (rand()%11 − 5)/100`, clamped to 0.25..=1. Draw count unchanged (replay tests pass).
- `crates/hst/src/play/flare.rs`: two meshes (rays untextured, sprites with `flare3.bmp` from CMN/GAME.XB) parented to the
  match camera at depth 0.11 in tangent space (follows FOV, any aspect). Additive, unlit, no fog.
  - visibility c = cos between view and (sun at 32768 − look point at 8192); sprite/ray intensities b1/b2 = min(128, c·192[·strength]);
    whiteout quad when c·4096 > 3712.
  - rays per the draw function: sun offset r = |d|/16 ≤ 400 boosts the rays facing away (scale 0x808 = 133.33 live),
    length L = fade·(p5>>2)·bright·boost (capped), width min(p5>>3, L/(0x7f8·(0.2 + 0.8·el/(π/2)))), 0x7f8 = 10 live.
  - Gouraud centre → black drawn in 6 bands so the falloff is shaded on gamma values as the GS does.
- Checked with `research/p3f5_flare_hunt.py` (pokes the flare's sun direction over PINE so slot 5's cameras look into it;
  screenshots via pad L3+R3 — copy 2 binds no F8) against `HST_EYE` + `HST_FLARE_SUN` port shots. Captures: `context/p3f5/`
  (`up_2.png` original, `port_up2.png`, `cmp_up2_crop.png`).

Iterations: `01-DRAW-FINAL.md`.
