# B121 — web: "The game stopped: length out of range of buffer" turning the phone while loading

Runs: `context/b121/run*` (`tools/web_check.mjs`, new `WEB_ROTATE=ms`: a touch phone viewport turned portrait ↔
landscape every ms).

## Cause
"Length out of range of buffer" is JavaScriptCore's RangeError for `new Uint8Array(buffer, offset, length)` past the
buffer's end. The generated glue has exactly one such call: `js_sys::Uint8Array::new_with_byte_offset_and_length`,
used by wgpu's WebGPU backend in `WebBuffer::get_mapped_range` (`new Uint8Array(getMappedRange(...), 0, len)`). That
import has no `catch`, so the exception goes straight through the wasm frames and kills the game.

WebKit's `GPUBuffer.getMappedRange` (WebCore `GPUBuffer.cpp`) returns a **0-byte ArrayBuffer** instead of throwing when
the buffer has no backing and was `mappedAtCreation`. Its WebGPU `Device::createBuffer` makes such an invalid buffer
when the device is no longer valid (lost), or when the allocation/validation fails. iOS loses GPU devices under memory
pressure. A rotation during a load (every canvas-sized target reallocated on top of the load's uploads) is the
likely trigger. Bevy creates mapped-at-creation buffers all through a load, so the next one after the loss threw
before Bevy's once-per-frame poll could report `DeviceLost`.

## Fix
- `tools/web.sh` patches that one glue call after `wasm-bindgen`: if the view doesn't fit its buffer, it returns a
  scratch `Uint8Array(len)` instead of throwing. A view that fits behaves exactly as before. The build fails if
  the original line is still there after the sed.
- `web/index.html`: when Bevy quits on `DeviceLost` after the first frame, the page reloads (once a minute at most,
  `sessionStorage`). Before, it showed "The game stopped". Bevy 0.19 can't rebuild a lost device in place:
  `RenderErrorPolicy::Recover` re-runs RenderStartup but nothing re-uploads the assets. Before the first frame,
  B109's WebGL2 fallback still applies.

## Checked (headless Chromium 152, WebGPU on the real GPU: `--enable-features=Vulkan --use-angle=vulkan`)
WEB_PRELOAD hook (scratch, not committed) that mimics WebKit on a lost device: from 25 s on, a mapped-at-creation buffer's
`getMappedRange` returns `new ArrayBuffer(0)`, and the device is destroyed 300 ms after the first one. Enter presses
go into Character Selection to cause a load.
- run6, glue as built: `RangeError: Invalid typed array length: 262144` from `WebBuffer::get_mapped_range` (V8's
  wording of the same error), then a mutex panic and `unreachable`: the user's crash.
- run7, patched glue + page: no exception; `Caught DeviceLost` → `hst: GPU device lost, reloading` → WebGPU again →
  menus answer input (visit2-70s.png, Character Selection).
- Rotation stress with no injected loss: run1 (WebGL2 build, turned 62× at 700 ms through boot), run2/run3
  (150 ms, 427×/684× through boot and menus), run8 (WebGPU, 300 ms, 215×), and WebKitGTK 2.52 MiniBrowser (WebGL2
  build in an iframe resized portrait ↔ landscape every 700 ms, 160 turns, pack served over HTTP from a scratch
  copy of the page): no errors.

## Not verified / not 1:1
- On a real iPhone: no iOS device or Web Inspector here, and WebKitGTK has no WebGPU, so WebKit's 0-byte mapping is
  confirmed from its source, not seen. Whether the user's device was actually lost (or a single allocation failed)
  is inferred. If a single buffer fails without a device loss, the patched view no longer throws, but Bevy then
  quits on the GPU's own OutOfMemory/Validation error and the page shows that message, which will name it.
- The reload after a device loss restarts at the title, so a match in progress is lost. Keeping the game running
  through a loss needs Bevy to re-upload every GPU asset, which 0.19 can't do.
- The rotation stress runs only covered the boot and menu loads (the Enter presses never reached a match load, the
  only mode change on the way). The ISO import wasn't rotated: it runs in a worker with no canvas or GPU, and its
  views are `FileReaderSync` ArrayBuffers copied whole.
- No lower memory peak on rotation: it would be a guess without the phone to measure.
- Not 1:1: n/a (browser shell).
