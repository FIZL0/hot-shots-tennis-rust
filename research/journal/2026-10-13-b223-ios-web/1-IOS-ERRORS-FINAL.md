# B223 — iOS web errors (FINAL)

No iPhone here, so neither iOS report could be reproduced; per the task, the diagnosable/fixable parts are done.

## Fatal error page (web/index.html, web/touch.js)
- The touch pad (`#touch`, z-index 10) sat over `#boot` (no z-index), so Reload couldn't be tapped. touch.js now hides
  the pad while `#boot.err` is up; `#boot.err` is z-index 30 (over the portrait "rotate" cover too) and its
  Reload / WebGL2 / ISO links are padded, bordered tap targets.

## Shader module that fails ("GPUDevice.createShaderModule: unable to make shader module")
- index.html wraps `GPUDevice.prototype.createShaderModule` in validation/internal/out-of-memory error scopes. A
  failure logs `hst: shader module "<label>" failed: <message>` + `getCompilationInfo` lines + the WGSL to the console,
  then reloads with `?webgl2` (same shaders through naga → GLSL); already on WebGL2 it stops saying which shader.
- Which shader it was is still unknown. Every custom variant a mod uses (gs.wgsl SKINNED + MORPH_TARGETS +
  VERTEX_TANGENTS) is also used by stock characters, so a mod-only WGSL feature is unlikely. Likelier: memory. The
  page reads every installed mod's files into wasm memory at boot (`add_mod_file`), and Anya is a 489 MB mod; iOS
  then fails GPU allocations (B121 saw device loss under memory pressure), and a shader module is one of them. If the
  console line shows an empty compilation info on a repeat, that's the memory case: mods would need to stay in OPFS
  and be read on demand.

## "Not installed: JsValue(undefined)" (web.rs, modstore.rs)
- `JsValue(undefined)` is what `web::call` gave when the page has no such function: `Reflect::get` → undefined →
  `dyn_into::<Function>()` fails with the value itself. The store path calls `window.hstZip` (added by B204 the same
  day) and the streamed writable's `write`/`close`. iOS had already saved a mod via `createWritable` (so OPFS
  writables exist there); most likely the page was an older cached `index.html` (navigations aren't fetched
  `no-cache` in sw.js; Pages' max-age is 600 s) paired with the new wasm, which has no `hstZip`.
- `call` now says `the page has no <fn> (an older copy of it: reload the page)`; `js_err`/`io_err` turn an `Error`
  into `Name: message`, undefined/null into "the browser gave no reason", other values via JSON. The web zip steps
  prefix which step failed (make / write / finish / open / read the download).

## Checks
- `cargo check -p hst --target wasm32-unknown-unknown --features webgpu`: builds. Page scripts pass `node --check`.

## Not verified / not 1:1
- Not run on iOS Safari (no device): neither the shader failure nor the install failure was reproduced, so the root
  causes above are inferred, and the WebGL2 fallback after a shader failure is untested on a real failure.
- The web build wasn't run in a desktop browser for this change (only compiled + JS syntax-checked).
- Menu/web shell only: nothing here touches gameplay, so no original-game comparison applies.
