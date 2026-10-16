# P34 — iOS: Palace at the Phone preset, 8-bit main texture

Task: confirm P32 on the iPhone (Grand Indoor Palace, Phone preset, `HST_FPS` gpu ms). If it's still under 60, try an
Rgba8 main texture.

## Device check: needs the human
There's no iPhone in this environment (`idevice_id -l` finds none), so the on-device half is in `plan/HUMAN.md`.
Because the device result couldn't be waited for, the Rgba8 lever is built now and is on by default in the iOS build.

## Rgba8 main texture (`graphics.rs` `main8`)
The scene cameras carried `Hdr` (hud_gamma.rs `retarget`), so their main texture was Rgba16Float, 8 bytes per sample ×
2 MSAA samples. The task suggested `CompositingSpace::Srgb`, but that's the wrong switch here. Bevy's upscale blit then
gets `SRGB_TO_LINEAR` and decodes the values, which our shaders already gamma-encode, on their way into the Rgba8Unorm
scene image. Its clear colour would also be encoded a second time.

The plain approach works: drop `Hdr` and set no compositing space. Bevy then gives the main texture the target's
format, which is the scene image's Rgba8Unorm. The blit copies it unconverted, and the clear colour goes through as
`LinearRgba`, the encoded values `hud_gamma::clear` already set.
- An LDR view puts `DEBAND_DITHER` into Bevy's PBR shader, which the HDR path skipped (its tonemapping node returns
  early with `Tonemapping::None`). So `main8` also sets `DebandDither::Disabled`.
- `Hdr` stays while HDR (G5) or bloom is on (bloom needs it; the HDR scene image is Rgba16Float anyway).
- `HST_MAIN8=1` turns it on elsewhere, `HST_MAIN8=0` turns it off on iOS.

## Checks (desktop, iGPU stand-in as in P32)
Pixels: `tools/shot.sh … --shot-at 2 --msaa 2x --scale 100 --shadows low`, `HST_WEATHER_SEED=1234`, A = off,
B = `HST_MAIN8=1` (screens in `context/p34/run1`). A vs a second A: 0 px on court 7, 7 px on court 2.
- A vs B: mostly ±1–2 levels (rounding to 8 bits after each blend). The largest differences are on the palace's
  stacked light glows (max 119 on 539 px of 921600; court 2: 2454 px > 2). Those are the sums Rgba16Float kept above 1
  between blends, and 8 bits clamp them, as the GS does. Side by side in a zoomed crop (`c7r0-zoom.png`) they look the
  same.
- With `--hdr` or `--bloom low` and `HST_MAIN8=1`: 0 px difference (Hdr kept).

GPU, `HST_PASSES=1 HST_GPU=igpu`, 1920×1080, palace, 100 %/Low/2×, two runs each (logs in `context/p34/bench`):

| | opaque | transparent | msaa_writeback | fps |
|---|---|---|---|---|
| off | 2.36–2.49 ms | 3.07–3.34 ms | 0.98–1.11 ms | 72–93 |
| on  | 2.04–2.15 ms | 2.69–2.92 ms | 0.67–0.72 ms | 94–111 |

That's about 0.7–1 ms less GPU per frame on the iGPU. A tile GPU with MSAA should gain more, because there the
bandwidth is the store/resolve of the tile.

Builds: `cargo build --release -p hst`, `tools/ios-build.sh check -p hst`.

## Not verified / not 1:1
- On the iPhone: not run (no device). Whether the palace reaches 60 at the Phone preset, with P32 and this change, is
  for the human (HUMAN.md). The gain on Apple's GPU is unmeasured.
- The Phone preset is still 67 % / Low / 2× without testing it on the device.
- 8-bit blending rounds/clamps between translucent draws where Rgba16Float didn't: small pixel differences, largest
  on stacked glows (described above). This is rendering, which only has to look right, and it's closer to the GS's
  8-bit frame buffer.
- Gameplay exactness: not affected (render target format only; no sim, RNG or timing touched).
