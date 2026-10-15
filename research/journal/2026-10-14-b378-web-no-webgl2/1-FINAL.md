# B378 — web build panics on Linux Chromium (Omarchy, ThinkPad T490)

The report had only the panic's prefix ("panicked at /home/runner/.cargo/registry/src/index.crates.io-…"): a
dependency's panic, on both the WebGPU and the WebGL2 build.

## What I tried (headless Chromium 152, tools/web_check.mjs, local build of this commit)
- software WebGPU/WebGL2 (the default headless run), device scale 1.25 / 1.5 / 1.667, hardware AMD through ANGLE
  Vulkan and ANGLE GL: all reach the match, no panic.
- `--disable-gpu --disable-software-rasterizer` (a Chromium whose GPU is blocklisted, crashed or switched off; recent
  Chromium no longer falls back to software WebGL on its own): no WebGL2 context and no WebGPU adapter, and the
  WebGL2 build panics in bevy_render: `panicked at …/index.crates.io-…/bevy_render-0.19.1/src/renderer/mod.rs:233:22:
  Failed to create wgpu surface: … "canvas.getContext() returned null; webgl2 not available …"`. Same registry
  prefix as the report; the WebGPU build in that state fails in bevy_render's renderer start too. This is the
  likeliest cause on the T490 (Intel UHD 620): its Chromium isn't giving pages hardware graphics.

## Change (web/index.html)
- Before the download, the WebGL2 build checks for a WebGL2 context; none → the page says in words that the browser
  gives no 3D graphics and what to change (graphics acceleration setting, chrome://gpu, #ignore-gpu-blocklist,
  about:support) instead of the panic. A renderer panic that still gets through ("Failed to create wgpu surface",
  "Unable to find a GPU", "webgl2 not available") shows the same text with the technical line under it.
- A panic on the WebGPU build before its first frame reloads with the WebGL2 build (as a render-error quit already
  did); checked by making the game canvas's `webgpu` context null: panic → WebGL2 → match plays.
- Error text drops the build machine's registry prefix (`bevy_render-0.19.1/src/…:233:22: message`), so the message
  itself fits a short report.
- New "Copy the error" link on the error screen: the shown text, the whole console error, user agent, WebGL2
  renderer and URL to the clipboard, for the next report.
- The hidden "Try WebGL2" link no longer shows on the error screen of the WebGL2 build (the err-state CSS overrode
  `hidden`), nor the dot before it.

Screens: context/b378/fix-nogpu2/visit2-10s.png (message), fix-gpupanic/ (WebGPU panic → WebGL2 match), fix-sw/.

## Not verified / not 1:1
- Not run on the user's ThinkPad T490 / Intel GPU: the cause is inferred from the matching panic prefix and the
  "both builds" symptom, reproduced only by switching off Chromium's GPU here. If WebGL2 does work in their
  chrome://gpu, the "Copy the error" text will carry the actual panic for a follow-up.
- If their Chromium has no hardware graphics, the game still can't run there until they change the browser setting
  (a page can't turn on software WebGL; Chromium needs `--enable-unsafe-swiftshader` for that). The page now says so.
- Front-end only, no gameplay touched; nothing to match against the original.
