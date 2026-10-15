# B414 — Lowest graphics preset (2026-10-15)

`Graphics::LOWEST` (`crates/hst/src/graphics.rs`): 50 % scale, Shadows Off, MSAA off, Mesh 25 %, Fewer Shadows and
Skip Off-Screen on, frame limit 30. It's in `PRESETS` right after Low (High, Default, Steam Deck, Low, **Lowest**,
Phone), so the Preset row steps Low → Lowest → Phone. Low is unchanged. The Preset help text names it. Tests:
graphics.rs (flags, preset name, `keep`, order) and options.rs `presets_and_switches` (Low → Lowest → Phone).
`tools/check.sh -p hst`: 424 passed.

## Measurements (load average 22–30 from other agents the whole time)

Court 3 (Wild West), 4 bots. Low = `--fps 60 --scale 67 --shadows low --msaa off`; Lowest = `--scale 50 --shadows off
--msaa off --mesh 25` plus the fps shown. Logs and runs are in `context/b414/`.

**GPU per pass** (native, `HST_PASSES=1`, 1280×800, sums of the logged passes at 30/35 s):

| | Low | Lowest |
|---|---|---|
| iGPU (Raphael, weaker than a Deck) | ~4.5 ms/frame (opaque 1.5–1.8, transparent 1.1–1.3) | ~2.4 ms/frame (opaque 0.7, transparent 0.5) |
| dGPU | ~0.85 | ~0.50 |

So Lowest is about half the GPU time of Low.

**Native, iGPU + `HST_SLOW=1` (2 cores at 120 %)**, `tools/bench.sh`, 45 s, two runs each:
- Low: 60 fps both runs. CPU 5.5 and 11.3 ms/frame; the second run landed in a load spike.
- Lowest at 60: 60 fps, CPU 9.1 / 9.2 ms/frame.
- Lowest at 30: 30 fps, CPU 8.5 / 8.5 ms/frame. That is about half the CPU per second of the 60 fps cap.

These per-frame CPU figures are dominated by the load. This machine holds 60 fps on every config, so it can't show
where the cap matters.

**Browser**: `tools/web_check.mjs`, MOBILE=1 (844×390), THROTTLE=4, WebGPU with Vulkan ANGLE, `WEB_TRACE=30:25`,
existing `web/pkg` (7785321, which has Mesh) with the settings passed as URL flags. Three runs each:

| run | Low: frames in 25 s · page M instr/frame · GPU busy ms | Lowest: same |
|---|---|---|
| 1 | 336 · 237 · 0.59 | 442 · 209 · 0.54 |
| 2 | 535 · 202 · 0.57 | 290 · 221 · 0.55 |
| 3 | 340 · 228 · 0.58 | 594 · 182 · 0.50 |
| mean | 404 (16 fps) · 222 · 0.58 | 442 (18 fps) · 204 · 0.53 |

The throttled browser is CPU-bound in wasm, where render settings don't help much: about −8 % page instructions
and −9 % GPU busy, but the spread between runs is larger than that. Lowest's gain is on the GPU (the table above).
That is what limits a weak phone or iGPU, not throttled Chromium on a desktop GPU.

**Why 30**: a device that has to choose Lowest is one that can't hold 60. An even 30 judders less than a frame rate
swinging between 40 and 50, and capping at 30 roughly halves CPU and GPU work per second (heat, battery; native 8.5
ms × 30 vs 9.2 ms × 60). The sim stays at 60 Hz. Anyone who wants 60 can step the Frame Limit row, which makes the
preset Custom.

**Look**: `context/b414/shots/low.png` vs `lowest.png` (court 3, HST_WEATHER=0). Same scene and HUD. Lowest has
no shadows at all (players included) and a softer 3D picture. It still reads as the game.

## Not verified / not 1:1
- No real phone: no iPhone or Android device is attached here. The phone stand-ins were the 4× throttled desktop
  Chromium and the iGPU with 2 cores. Both were measured with the machine at load 22–30, so the fps figures are
  noise. The GPU-pass times are the solid numbers.
- The 30 fps cap does nothing in the browser: `graphics::limit` returns on wasm32 because the browser paces frames.
  On the web, Lowest runs uncapped. Capping the web build would need a rAF-skip, which is outside this task.
- Sway LOD (a Settings switch, not part of `Graphics`) isn't set by any preset. B411 decides whether it stays.
  If it does, it should join Lowest.
- I didn't look at the Preset help text's longer wording in the menu's side panel (it wraps like B412's text).
- Rendering only has to look right (PLAN). No gameplay code was touched.
