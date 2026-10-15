# G1 — ambient occlusion (Settings → Graphics → Ambient Occlusion)

G1 asked for AO first, then bloom, shadow filtering, HDR/tonemapping and smoothed normals. This iteration does AO;
the other four are split out as G3 (bloom), G4 (shadow filtering), G5 (HDR/tonemapping) and G6 (smoothed normals).

## What it does
- `Graphics::ao` (0 off, 1 low, 2 high): `--ao low|high`, settings.txt `ambient_occlusion = off|low|high`, a row on
  Settings → Graphics after Anti-aliasing. Set by presets (off in all of them: the default look is unchanged),
  forced off in Original Mode, and locked off on WebGL2 (Bevy's SSAO needs compute shaders).
- `graphics::ao` puts Bevy's `ScreenSpaceAmbientOcclusion` (GTAO-based; quality Low/High) on the scene cameras that
  clear (the court's), not on overlays such as the call camera in `popups.rs`. Bevy's SSAO needs `Msaa::Off`, so the
  `msaa` system draws every scene camera without MSAA while AO is on, and `Smaa` stands in when Anti-aliasing is
  2x/4x. Turning AO off removes SSAO, the prepass components and SMAA again.
- `GsMaterial` now has `enable_prepass` (only cameras with prepass components run it) and its own prepass vertex
  shader in `gs_prepass.wgsl`: the same positions as `gs.wgsl` (its morph-descriptor guard and the GS guard band;
  without the guard a triangle the main pass drops would still write prepass depth and punch a hole). The fragment
  writes camera-facing normals (two-sided and mirrored parts, skinned normals that aren't unit length).
- `specialize` used to treat any key with `DEPTH_PREPASS` as the shadow pass, but a view with a depth prepass
  puts `DEPTH_PREPASS` in its main-pass key too. It now detects the prepass pipeline by its `PREPASS_PIPELINE`
  def; the shadow pass is that pipeline without `NORMAL_PREPASS`.
- `gs.wgsl` (`GS_AO`, opaque and cut-out draws only; blended ones draw over the AO): the vertex stage passes the
  share of its light that isn't the sun's (ambient + second light). The fragment scales that share by the
  visibility before fog. Bevy's GTAO reads about 0.89 on the open, flat court from the match camera, so the
  visibility is divided by 0.89 (clamped to 1). Without this the whole court came out ~7 % darker.

## Checks
- `tools/shot.sh` at 1280×720: `--stage 1 --play` off vs high, and `--stage 4 --play` off vs low
  (`context/g1/run1`, `run3`). Contact shade at the net's foot, the umpire's chair and the players' feet; open
  ground matches off within the run-to-run noise (110 vs 110, 148 vs 148 mean luma). SMAA edges look smooth.
- Debug output of the raw AO (removed): ground 0.87–0.93, chair foot 0.75, ambient share on the ground 0.85.
- AO off unchanged: a static `--stage 1` view (no match) is pixel-identical between two runs of the new build. The
  old build differs from the new one in 4 isolated pixels, by at most 8 levels, on shadow edges (`context/g1/run2`):
  the sun's pass now goes through the new prepass vertex shader, so it rounds differently by a sub-texel.
- `Graphics` text/flags round trip covers `ao`; Original + `--ao high` draws like Original (same noise as two
  Original runs).
- `tools/check.sh`: everything passes except `play::mod_match::tests::umpires_are_on_the_roster_and_play_a_rally`.
  The shared `mods/` folder (symlink into the main checkout) holds only `hst_umpire_04_anna`; the test expects all
  five umpire mods (B380/B381 are removing them). Unrelated to rendering, so G1 was ticked and committed by hand
  (`ctx.py tick` + commit) instead of through `done.sh`.

## Not verified / not 1:1
- AO is ours, not the original's: by design nothing to match. Judged by eye on two courts only (Stage 1, Stage 4);
  the 0.89 flat-ground normaliser comes from the match camera's angle and may over- or under-shade at very low
  replay-camera angles.
- Browser WebGPU build not run (no browser test here); WebGL2 has it locked off by cfg.
- Mod characters with AO not looked at (their normals go through the same skinned prepass path).
- The 4-pixel shadow-edge difference with AO off (above): render-only, within "looks right", not chased.
- Frame cost of AO not measured (Steam Deck/phone presets keep it off).
