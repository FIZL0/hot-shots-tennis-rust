# P31 — iOS: loading stalls

Build 2012's log: a 1.0–1.5 s frame at match start (game 122 ms, then a frame with render 548 ms right after the
loading screen came down) and 0.4–0.9 s at result → menu (game 459 ms). The task: find out which it is (pipeline
compile, asset upload or disc read), then keep the loading screen up until it's over.

## Measuring: `stall.rs` (on with `HST_FPS`, which is always on for iOS)
One line per stall (any stage or frame over 50 ms), and every frame in full for the 10 frames after a loading screen
comes down:
- `game stall: update X ms, then Y ms to the next frame …; disc +N MB, entities E (+d) at T s`. Update is First →
  Last. The gap is extract, window events and present back-pressure. Disc is bytes read through `hst_data::iso`
  (new counter `iso::READ`).
- `render stall X ms: assets (images +n M MB, meshes +m) specialize acquire queue+prepare pipeline queue (ready +k,
  compiling c) graph+submit at T s`. Stage marks in the render schedule. The image count and size are new
  `GpuImage`s (base level, no mips). `ready` counts pipelines that finished that frame.
- `pipelines ready +k (… compiling c) at T s` for every frame where a pipeline lands. One that lands after
  `loading screen down after N frames at T s` (that line now has a time too) was compiled mid-match.

## Desktop (RX 9070, Vulkan; `--stage 1 --play` and the boot to the menu; context/p31/run1–5)
- Match build: update 1.47–1.82 s, disc 89 MB, +5511 entities. It all runs under the cover.
- The next render frame: 310–377 ms. Assets 137–155 ms (436–442 new textures, 490–497 MB; meshes +283),
  queue+prepare 148–198 ms, graph+submit 21–40 ms, 59–66 pipelines ready. Still under the cover.
- With Mesa's shader cache off (`MESA_SHADER_CACHE_DISABLE=true`), the remaining ~80 pipelines land over the next
  0.3 s. The last one lands at 2.21 s and the cover comes down at 2.25 s (26 frames). After that only one pipeline
  lands, at 27 s, mid-match.
- The 10 frames after the cover lifts: game 5–10 ms, render 9–18 ms, nothing uploaded, nothing compiling.
- Menu build: 1.16 s, 7.8 MB disc. The render frame after it takes 127 ms (72 textures, 227 MB). Cover down after
  24 frames.

So on desktop, the stalls are the build itself (a blocking Startup, by design under the cover) and one heavy
upload/prepare render frame right after it, also under the cover. Nothing stalls once the cover is down. The iOS
spike after the cover lifts can't be reproduced here, so it's an iOS-side difference.

## Fix: the cover also waits for the render side to settle (`mode.rs`)
Until now the cover waited for `Compiling` (queued render pipelines) to read 0 for 3 frames. That count misses
work that hasn't been queued yet. A material whose textures land a frame late isn't specialised, so its pipelines
aren't in the queue, and the cover can come down before that upload/specialise/compile frame. Now a render-world
system (`unsettled`, Cleanup) marks a render frame as unsettled if it added new textures or meshes
(`ExtractedAssets::added`) or took over 100 ms since the last one ended. That covers uploads, a compile that holds
a lock the frame needs, and the GPU catching up at acquire. `loaded` counts an idle frame only when nothing is
compiling *and* the last render frame settled. The 10 s cap stays. Headless (no render app), nothing ever reads as
unsettled.

Desktop with the fix: match cover down after 23 frames, menu after 24 (same as before, so it doesn't hold longer
where nothing is late).

## Not verified / not 1:1
- **The iOS cause is still unconfirmed.** No device here. The next build's hst-log.txt will say which it is (the
  `render stall` stage, `images +`, `ready +`, and whether any of it comes after `loading screen down`). If the
  spike still comes after the cover with all the work under it, the cover coming down is itself the trigger (the
  first unpaused frame: game 122 ms on the phone vs 7.7 ms here). That needs that frame's breakdown, which the
  10-frame watch now logs. Follow-up P31a1; human step in plan/HUMAN.md.
- Result → menu (0.4–0.9 s, game 459 ms) is the menu's blocking build under the cover (desktop: 1.16 s). It's
  expected for a loading screen and the cover is up the whole time, so I didn't change it.
- Pre-specialising pipelines: not needed beyond what `warm` already does (every mesh and off camera drawn under the
  cover). On desktop every pipeline but one is ready before the cover comes down.
- Menus and loading screens don't have to match the original (PLAN), so there's nothing to compare against the
  game.
