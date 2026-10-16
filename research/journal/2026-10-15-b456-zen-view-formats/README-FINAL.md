# B456 — web build stopped in Zen: "Downlevel flags VIEW_FORMATS … main_texture_a"

## Cause: already fixed by B438
Zen (Gecko) gets the WebGL2 build on the `auto` renderer (index.html, B298). `2f258567` ("8bit graphics",
2026-10-15 16:46) took `Hdr` off the scene cameras, so the main texture became `Rgba8Unorm`, and Bevy asked for
an `Rgba8UnormSrgb` view format on it. wgpu's GL backend never has `DownlevelFlags::VIEW_FORMATS`, so
`main_texture_a` failed validation and Bevy stopped. B438 (`6f2771b0`, 18:24 the same day) fixed it in the patched
`third_party/bevy_render` (`prepare_view_targets` asks for the extra view format only when the adapter has
`VIEW_FORMATS`). The user's report came from a build deployed between those two commits. No code change was needed here.

## Verified in Zen 1.23.2b (Linux), headless
New `tools/web_zen.mjs` drives Zen over WebDriver BiDi, the way `web_check.mjs` drives chromium: it imports the ISO
into OPFS, then a second visit runs the game, with screenshots and the page's console. It works around these
pitfalls: a headless tab is `hidden` (rAF at 1 Hz, the game crawls) until `browsingContext.activate`; the site page
comes first unless `hst-autoplay` is set in localStorage; BiDi's `log.entryAdded` misses some messages, so the
page's own console is also captured by a preload hook. `RENDERER=webgpu|webgl2` sets the game's renderer setting.

- Repro: HEAD with B438's check forced on (`let view_formats = true || …`, a temporary local edit, not committed),
  WebGL2 build: `Caught rendering error: Validation Error … In Device::create_texture, label = 'main_texture_a'
  Downlevel flags DownlevelFlags(VIEW_FORMATS) are required but not supported on the device.` This is the user's message
  word for word.
- HEAD (3d351051) WebGL2 (`auto`): main menu (`?`), `?play`, and `?play&stage=1` (court, 4 players, HUD) all run
  with no rendering error.
- HEAD WebGPU (`RENDERER=webgpu`): Zen's Linux WebGPU, main menu and `?play&stage=1` match, no error. Its WebGPU
  accepts the sRGB view format.
- A pre-8-bit build (12:09, Hdr main texture) never hit it on either backend. That matches the cause.

Screenshots and logs are in `context/b456/` (`match-auto.png`, `match-webgpu.png`, `webgl2-new-.png`, `zen-repro/`).

## Not verified / not 1:1
- The user's own Zen install (OS and version unknown) was not tested; only Zen 1.23.2b on Linux, headless. On
  WebGPU, wgpu's browser backend reports every downlevel flag, so a Gecko WebGPU that refused view formats would
  still fail. Linux Zen's WebGPU doesn't refuse them, so this couldn't be tested. The reported error was the GL path
  in any case (Gecko defaults to WebGL2).
- Gameplay exactness: not applicable (rendering setup only, no sim code touched).

## Wrap-up note
`tools/done.sh` stopped on `play::main_menu::pointer::tests::wide_window_hits`. That test also fails on a clean HEAD
(3d351051): it broke when B450/B422 were merged, and this task touched no Rust. It's filed as its own task; B456 was
ticked and committed by hand after `tools/audit.sh` passed.
