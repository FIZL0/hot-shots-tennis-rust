# G3 — bloom (Settings → Graphics → Bloom)

## What it does
- `Graphics::bloom` (0 off, 1 low, 2 high): `--bloom low|high`, settings.txt `bloom = off|low|high`, a row on
  Settings → Graphics after Ambient Occlusion. Set by presets (off in all of them), forced off in Original Mode.
  Not locked on WebGL2: Bevy's bloom is render passes only (it has its own WebGL path).
- `graphics::bloom` puts Bevy's `Bloom` on the scene cameras that clear (the court's; not an overlay such as the
  call camera, which would bloom the court a second time), and removes it when off.
- The court cameras are already `Hdr` with tonemapping off and hold gamma-encoded values (`hud_gamma.rs`), so the
  prefilter threshold works on encoded brightness (Bevy uses the brightest channel): threshold 0.85, softness 0.2
  (knee 0.68–1.02), `Additive` composite (Energy-conserving would dim everything else), intensity 0.06 / 0.12,
  the rest Bevy's `NATURAL`. No shader change was needed: `gs.wgsl` already writes encoded values into the HDR
  texture, and additive draws above 1.0 simply bloom more.

## Tuning (`context/g3/`)
- run1: threshold 0.8, softness 0.5, intensity 0.16 — the knee starts at 0.4, so clay mid-tones glowed and the
  whole court washed out. run2 showed `--stage N --play` shots differ run to run anyway (off vs off ±7 mean), so
  the numbers below are from static `--stage N` views, which are deterministic.
- run3, off vs high, mean change by the off pixel's brightest channel (0–255):
  dull courts 1, 7, 9: ≤0.04 overall, highlights +0.5–2; bright courts 2, 3, 5: +0.5/+1.1/+2.9 overall,
  ≥230 pixels +3/+5/+8, 150–200 pixels +0.3–0.5. Stage 5's sky glows over the stadium roof's edge.
- run4: `--original --bloom high` is pixel-identical to `--original`; low on stage 5 is +1.2 mean (high +2.9);
  stage 3 `--play` with high: the court lines and sunlit sand glow softly, the HUD is untouched (it composites
  after the scene).

## Checks
- `Graphics` text/flags round trip covers `bloom`; a test checks it is preset-set and passes as `--bloom low`.

## Not verified / not 1:1
- Bloom is ours, not the original's: nothing to match. Judged by eye on stages 1, 2, 3, 5, 7, 9 at 1280×720;
  night/indoor courts and cut-away/replay cameras not looked at specifically.
- Browser builds (WebGL2/WebGPU) not run here.
- Frame cost not measured (presets keep it off).
- Interaction with the in-match frame persistence / sun glare (`hud_gamma.rs` `persist`) only seen in still shots:
  bloom happens in the court camera before the persistence mix, so it carries the 25 % of the last frame like any
  other pixel.
