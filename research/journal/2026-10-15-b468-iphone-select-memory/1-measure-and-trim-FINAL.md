# 1 — measure, then trim what doesn't need to stay

## Instrumentation (kept)
`web.rs`: on the web a counting global allocator (`Counted` over `System`: live and peak bytes) and the
`memory_log` system (Last) print `hst: wasm memory N MB (live L MB, peak P MB)` when the wasm heap grows 32 MB or live or
peak moves ≥ 8 MB (at most once a second). On an iPhone, Safari's Web Inspector console shows these lines, so the
next report can say where it died.

## What the numbers say (headless Chromium, `context/b468/run1..run11`, `final-*`)
- **Picking itself spikes nothing.** Ashley vs CPU Jun → confirm screen: the wasm heap's live bytes don't move 8 MB,
  renderer/GPU process RSS stays flat, about 1.4 MB of textures are created. Baking every select card from a fresh
  profile (run10) doesn't spike either. Voices/SFX are mixed by our own mixer (`audio.rs`, PCM in the wasm heap,
  which the counter covers).
- **The peaks come at boot and while the menus are set up, before the pick:**
  - wasm heap: live 232 MB at boot (the 171 MB disc pack + ~61 MB), a short peak of 435–439 MB in
    `main_menu::setup`, then 251–292 MB steady. The heap reserved stays at 425 (WebGPU) / 469 (WebGL2) MB because
    wasm memory never shrinks.
  - JS heap: the pack was read whole into a JS `ArrayBuffer` (171 MB) and then copied into wasm. Total JS heap was
    169–318 MB at boot and only freed by a GC ~25–30 s later. That's in the window where a player is already
    choosing a character, so on a phone the pick lands on top of an extra ~170–280 MB that hasn't been collected yet.
  - WebGPU: the renderer and GPU process grew ~2–2.4 MB/s on menus until GC (WebGL2 stayed flat). Part of that was
    bevy_pbr's CPU clustering making 4 new GPU buffers every frame (the cluster index/offset storage buffers,
    `mappedAtCreation`). wgpu's web backend never calls `destroy()` on a dropped buffer or texture, so each one
    stays alive until the JS GC finds it.
- Renderer RSS baseline is ~1.1–1.3 GB on both backends. Compiling the 66 MB (wasm-opt'd) module alone costs ~83 MB
  of RSS in node.

## Changes
1. **Pack streamed into wasm** (`web.rs` `pack_begin(len)` / `pack_part(bytes)` / `run()`, `index.html` `start()`):
   the page now reads the OPFS/memory pack in 8 MB `Blob.slice`s straight into the game's buffer. The JS copy never
   exists. JS heap peak at boot dropped from 169–318 MB to 96 MB (run11), and to 43 MB in the final WebGL2 run.
   `run()` refuses a short pack instead of mounting it.
2. **CPU clustering reuses its buffers** (`third_party/bevy_pbr/src/cluster/mod.rs`
   `prepare_clusters_for_cpu_clustering`): each view keeps last frame's `ViewClusterBindings` (cleared) unless the
   storage/uniform kind changed. WebGPU now makes no `createBuffer` per frame (`final-webgpu9`: gpucount has no
   createBuffer after startup). Native is unchanged in behaviour (`tools/check.sh`: 740 tests pass).

## Checked
- `final-webgl2`, `final-webgpu9` (`--enable-features=Vulkan --use-angle=vulkan --enable-gpu`): boot → Local Play →
  Ashley vs Jun → "Ready to play?" with both cards drawn, no panic or validation errors. WebGPU without those Chrome
  flags loses its device before the first frame in this headless setup. That happens without these changes too
  (`final-webgpu7-nopbr`), and the page falls back to WebGL2 as designed. Note for later runs: `?webgpu` isn't a page
  flag; the game reads it as an archive name and exits.

## Not verified / not 1:1
- **Not reproduced on an iPhone / WebKit**: there's no iOS device or WebKit with Metal here. The cause (tab killed
  for memory) and the effect of the fixes are inferred from Chromium numbers. The user should retest on the
  iPhone 13 and read the `hst: wasm memory` lines (Web Inspector) if it still reloads (`plan/HUMAN.md`).
- **Audio on WebKit not measured**: headless Chromium runs with no audio output, so the WebAudio side (the
  AudioWorklet and its buffers) is unmeasured. The voice/SFX PCM itself is in the wasm heap and showed no growth at
  the pick.
- **WebGPU GC churn remains**: still ~34 `createBindGroup`, 14 command encoders and 4 texture views per frame, and
  nothing is ever `destroy()`ed on the web. If bind groups are Metal argument buffers on WebKit, a lagging JS GC
  could still pile up GPU memory. Destroy-on-drop in wgpu's web backend would need reference counting through bind
  groups and encoders (or it risks validation errors), so I set it aside.
- **Startup peak of `main_menu::setup` (~435 MB live) and the ~1.1 GB renderer baseline not reduced**: the menu art
  sheets (`menu_art.rs` K=4) are the biggest transient. Lowering K changes how the menus look, so I left it as is.
- Front-end only (character select): no comparison against the original needed.
