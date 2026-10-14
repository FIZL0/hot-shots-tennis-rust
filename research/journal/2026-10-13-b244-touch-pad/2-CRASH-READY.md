# 2 — the phone's WebGPU createBindGroup crash

All we have from the phone is "createBindGroup after a hit" plus `pkg-webgpu.js:2108:34`, which is the panic hook's
`new Error()` in the generated JS, so it's a Rust panic or Bevy quitting on a wgpu validation error. The full browser
message never got saved.

## What was ruled out (desktop)
- **Limits/features**: a native release build with WebGPU's spec-default limits and no optional features
  (`WgpuSettings { constrained_limits: WgpuLimits::default(), disabled_features: all }`, a temporary patch, since
  reverted) played hits with no validation error.
- **Headless Chromium on WebGPU (Vulkan)**, with every new `createBindGroup` descriptor logged through a match
  (context/b244/bg/bg.txt): the only new bind groups after a serve are `preprocess_direct_bind_group`, and all of them
  are valid. It didn't crash on desktop Chrome.
- **Safari can't be tried here**: WebKitGTK MiniBrowser 2.52 has no `navigator.gpu`.
- **Bevy 0.19.1's morph storage path** (my first suspect, since the impact effects after a hit are morphed meshes): the
  `morphed*` bind groups add binding 8 (morph descriptors) only when the descriptors buffer exists, while the layout
  always has it on WebGPU. But `write_morph_buffers` (PrepareResourcesFlush, before PrepareBindGroups) writes that buffer
  whenever it writes the weights, with a default entry if it's empty, and the bind groups are only built once the weights
  buffer exists. So the mismatch can't happen. Still, B137a (bounce effects stretched into flat panels, iPhone WebGPU
  only) also points at the morph storage path on iOS (`morph_descriptors[mesh.morph_descriptor_index]` reads), and so does
  B253's "descriptor missed the frame it reappears". It's worth checking again once there's a phone trace.

## What changed
`web/index.html` `fail()`: on the WebGPU build, an error whose text (the panic line, the error, or Bevy's first
render error) mentions `createBindGroup` or `Validation` now reloads the page into the WebGL2 build
(`window.hstWebGL2(why)`, which so far was only used before the first frame). The browser's message stays in the
console (`hst: WebGPU stopped (…), reloading with WebGL2`). The user found that the WebGL2 build plays on, on their phone
(B273). The page stays on `?webgl2` for later reloads, and an error on WebGL2 shows the error box as before, so it can't
loop. Checked in headless Chromium on WebGPU (context/b244/fb/log.txt): after the first frame I faked a panic line
saying createBindGroup, then a trap. The log shows `hst: WebGPU stopped (…), reloading with WebGL2` → `hst: WebGL2`, and
the game starts again on `?webgl2=`.

## Not verified / not 1:1
- **The root cause on the phone is unknown and not fixed in the WebGPU path**: it can't be reproduced on desktop
  Chromium, and there's no Safari WebGPU here. The fallback only makes it survivable. The match in progress is lost
  (the page reloads into the menu).
- **The fallback hasn't run on a phone**: needs the user. Play a match on the iPhone (WebGPU). After the first hit, the
  page should reload into WebGL2 and keep running. Before it reloads, Safari Web Inspector's console has the full
  `hst: WebGPU stopped (…)` message. Paste that line into this journal to pin the real cause.
- Touch pad in menus and P1 control: see `1-PAD-READY.md` (checked in emulated touch Chromium, not on a phone).
