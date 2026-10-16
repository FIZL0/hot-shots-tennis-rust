# P56 — Renderer auto-pick: why Chrome 152 on Linux (RX 9070 XT) got WebGL2

## Finding: WebGPU was never offered (no fallback)
This machine has the same card (Navi 48, RX 9070 XT) and Chromium 152.0.7977.82. Probe page (headless, `--enable-gpu`):

| flags | `navigator.gpu` | `requestAdapter()` |
|---|---|---|
| none (stock) | present | **null** |
| `--enable-unsafe-webgpu` | present | google / swiftshader (software) |
| `--enable-features=Vulkan` (= chrome://flags/#enable-vulkan) | present | amd / rdna-4 (hardware) |

So under Auto, index.html saw an API but no adapter and went WebGL2 directly; the `hstWebGL2` fallback (`?webgl2`)
never ran. Stock Chrome on Linux keeps WebGPU off until Vulkan is on; turning on Vulkan alone gives the hardware
adapter. Only the unsafe-WebGPU flag hands out SwiftShader, which the old pick would have taken as "WebGPU".

## Changes
- `web/index.html`: under Auto a software adapter (`isFallbackAdapter` / `info.architecture === 'swiftshader'`) gets
  WebGL2 (an explicit `renderer = webgpu` still takes it). The reason WebGL2 was picked goes to localStorage
  `hst-renderer-why` (`webgpu forced setting gecko no-api no-adapter software`) and the console line; a WebGPU build that
  stopped and fell back stores its reason in `hst-webgpu-stopped` (cleared the next time WebGPU is picked).
- `web/site.js` Settings → Display: a note under Renderer (`rendererNote`), from its own adapter probe plus those keys:
  Chrome on Linux without an adapter → "open chrome://flags/#enable-vulkan, set Vulkan to Enabled, relaunch; Auto
  then picks WebGPU; chrome://gpu should list WebGPU as Hardware accelerated"; software-only adapter says so; other
  Chromium → chrome://gpu / acceleration / driver; no API → which browsers have it; adapter now usable but the last
  start was WebGL2 → "Auto uses it from the next start"; last WebGPU run fell back → its reason. Nothing for Firefox
  (WebGL2 on purpose, B298) or a WebGL2 setting.

## Checks
Headless Chromium 152 against a local server: the Settings note shows the right text in all three flag setups; the
pick lines of index.html, run on their own, give `false no-adapter` / `false software` / `true webgpu`.

## Not verified / not 1:1
- A full match on the WebGPU build with Vulkan on in a headed Chrome on this card: the full boot doesn't complete headless
  (no disc pack there) and a headed window would pop up on the user's desktop. The user should try it with the flag on.
- Headed (non-headless) Chrome's adapter policy is assumed to match headless; Chrome may later enable WebGPU on Linux by
  default, after which the note simply doesn't show.
- Front-end/web page only; no gameplay, so no original to compare against.
