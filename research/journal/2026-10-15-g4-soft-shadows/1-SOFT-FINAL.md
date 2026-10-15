# G4: softer shadow filtering (Settings → Graphics → Soft Shadows)

## What it does
- `Graphics::soft_shadows` (`--soft-shadows`, settings.txt `soft_shadows = on|off`, a Graphics row after Ambient
  Occlusion). It's preset-set (off in every preset, so the default look doesn't change). `Graphics::soft_on()` is
  false in Original Mode (G2 gives that its own look), with Shadows Off, and on WebGL2 (locked row, like AO).
- Switch: `graphics::soft_shadows` puts Bevy's `ShadowFilteringMethod::Temporal` on every `hud_gamma::Scene` camera
  while it's on, and removes it when it's off (Bevy's default, Gaussian, as before). That is the only free per-view
  state that reaches material specialisation without a new uniform or a material rebuild. `gs.rs::specialize`
  turns the Temporal key bit into `GS_SOFT` (main pass only).
- `gs.wgsl` under `GS_SOFT`: `sun_visibility` runs its own filter instead of Bevy's (so Bevy's temporal noise
  never shows):
  - a 6×6 blocker search (`textureLoad` of the shadow map's texels) over the widest penumbra
  - the mean blocker height above the ground along the sun (depth difference ÷ the cascade's z-row length:
    orthographic, so depth per metre is constant)
  - penumbra radius = 0.015 m per metre of height, at least 1 texel and at most 0.25 m
  - k×k bilinear-compare PCF over that radius (k = 3…9, taps ≤ ~2 texels apart)
  - Cascade choice and blending are as in Bevy's `fetch_directional_shadow`, with the same depth and normal biases.
  The result: a player's feet stay sharp, a tall pole's or a tree's shadow blurs.
- PCSS (`experimental_pbr_pcss`) wasn't used: it changes the view bind-group layout for every build, including
  WebGL2, and Bevy's own comment calls its non-temporal path noisy. Temporal on its own is noisy without TAA.

## Checks (`tools/shot.sh`, 1280×720, `--stage N --play`, `context/g4/`)
- Stage 4: a cross-shaped shadow of a tall lamp-like caster. First version (0.025 m/m, max 0.4 m, 5×5 search) washed it
  out with wavy streaks (`run2/z4-on.png`). With 0.015 m/m, max 0.25 m and a 6×6 search the shape stays readable
  and the edges are soft (`run4/z4-on.png`). A player's contact shadow is still sharp at the shoes
  (`run4/feet.png`).
- Stage 1: the open court is 7.5 % brighter with Soft Shadows than without (121 vs 112 mean luma). With shadows
  off it's 121 too. Bisected: with Bevy's Temporal filter, or a single centre tap through my lookup, it's also 121, so
  only Bevy's Gaussian (the current default) darkens it. Some caster near the ground self-shadows inside that
  kernel. This isn't G4's problem; filed as **B404**.
- Off unchanged: all new shader code is behind `GS_SOFT`. Two `--play` "off" shots differ only by the usual
  animation noise (4–7k pixels on the players, same luma on the ground).
- `tools/check.sh`: all pass. `Graphics` text/flags round trip covers `soft_shadows`. Test: preset becomes Custom, the
  flag passes only when on, off in Original Mode and with Shadows Off. Anti-aliasing stays row 3 (pointer test).

## Not verified / not 1:1
- Soft shadows are ours, not the original's: nothing to match. Judged by eye on Stage 1 and Stage 4 only. A Stage 1
  `--soft-shadows` run rolled rain (blob shadows), so tree shadows under the sun weren't looked at closely on it.
- Frame cost not measured: up to 36 + 81 texel reads per ground pixel in a wide penumbra, 36 + 9 at contact.
  It's off in every preset.
- Browser WebGPU build not run. The shader's texel loads assume the depth array (`NO_ARRAY_TEXTURES_SUPPORT` is
  WebGL2-only, and that build has the option locked off by cfg).
- Some faint banding stays inside the widest penumbrae (the blocker estimate still changes from texel to texel).
  It's render-only and within "looks right"; not chased.
- Mod characters and the replay/cut-away cameras with Soft Shadows on weren't looked at (same `Scene` cameras, same path).

Later (B404): the Stage 1 brightness gap was the weather roll (clock-picked seed; cloudy vs clear), not the
Gaussian filter. With `HST_WEATHER_SEED` pinned, every filter gives the same open court.
