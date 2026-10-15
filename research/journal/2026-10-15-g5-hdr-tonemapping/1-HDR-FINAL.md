# G5 — HDR / tonemapping (Settings → Graphics → HDR)

## What the tonemapper acts on (the decision the task asked for)
Every 3D shader writes gamma-encoded PS2 colour (`hud_gamma.rs`); the game lights in those values (VU1 vertex light
× texel, MODULATE up to 2×, + the highlight) and the GS clips the result to 8 bits. There is no linear light to
tonemap, and a filmic curve over the whole range would change every mid-tone of the original look. So:

- **Input**: the same encoded values, un-clamped at the GS stage only. `gs.wgsl` under `GS_HDR` skips the final
  `clamp(rgb, 0, 1)` (and the untextured path's 0xff cap), so texel × light and the highlight keep their overshoot.
  The scene cameras already draw into Bevy's Rgba16Float intermediate (`Hdr`); the scene image and the two frame-
  persistence images become Rgba16Float too, so blends and the 25 % trail keep it until the composite.
- **Not** the vertex light sum: I tried un-clamping VU1's 0xff too (`context/g5/run1`). On Stage 1 the sun's light
  sum exceeds 0xff over the whole clay, so the court's mid-tones came out ~8 % brighter (mean 134 → 145 on a patch
  far below 1). That is the art's tuning, not a clip of highlights; the vertex clamp stays.
- **Curve** (`hud_gamma.wgsl` `tonemap`, `HDR` def): identity up to 0.8 on the brightest channel, then
  k + (1 − k)·x/(1 + x) (slope 1 at the knee, white only at infinity), the other channels scaled alike (keeps hue
  where per-channel clipping turns sunlit orange yellow), then towards white by the square of the share taken off.
- **Switch**: `Graphics::hdr` (`--hdr`, settings.txt `hdr = on|off`, row after Ambient Occlusion, preset-set: off
  in all presets), `hdr_on()` false in Original Mode. The defs ride on the Shader assets' `shader_defs`
  (`hud_gamma::hdr`), so turning it on/off recompiles every gs/composite pipeline once; `follow_window` swaps the
  images' format. Off: nothing changes (same shader text paths, same Rgba8Unorm images).

## Checks
- `tools/shot.sh` 1280×720 `--stage 1 --play` and `--stage 4 --play`, off vs `--hdr` (`context/g5/run2`): 1.3 % /
  2.3 % of pixels differ by > 4 levels, all on bright parts (court lines, white clothes, the umpire's cap); the
  skirt's folds and the cap's shading show where off clips them flat white. Lines come out slightly softer (0.8+ is
  compressed). Mid-tones unchanged (court patches equal within 1 level).
- `--hdr` at launch exercises the runtime path: images are created Rgba8Unorm, swapped to Rgba16Float, defs added on
  the shaders' Added event.
- Settings → Graphics with the new row at 1280×720 and 800×900 (`HST_MENU=graphics`): 12 rows fit, both layouts.
- `Graphics` text/flags round trip covers `hdr`; `tools/check.sh` passes (671).

## Not verified / not 1:1
- HDR is ours, not the original's: nothing to match. Judged by eye on Stage 1 and Stage 4 only, in daylight.
- Turning HDR **off** live (menu toggle back) not exercised in a run; same code path as on (def removed, format
  swapped back), only on was run.
- No real HDR display output (the window stays SDR 8-bit); this is tonemapping of overbright values into SDR.
- Browser builds (WebGL2/WebGPU) not run: Rgba16Float render targets and filtering are core in WebGL2, not tested.
- Bevy PBR (`StandardMaterial`) draws also skip their clamp in the float scene; none looked at specifically.
- Frame cost not measured (one extra branch in the composite; float images double the scene's bandwidth).
