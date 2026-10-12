# B110 — web app freezes on a hit (first lob)

## Cause
Not a disc read: every `Iso::open` in the match runs in a setup system (the pack is already mounted in memory),
and the lens flare's sheet loads on its first draw frame, under the loading screen. Sounds are decoded up front.

It is first-use render pipelines. The match's effects (hit sparks, ball flight, bounce, landing marks, the "!"
pop-ups, flare) are spawned hidden at setup. Bevy only specializes a material's pipeline for a mesh once it is
drawn, and on wasm `PipelineCache` builds pipelines synchronously in the frame (no async compile there: shader
composition in wasm plus the browser's shader compile, on the main thread). So the first frame an effect shows
compiled its pipelines mid-rally. A phone's GPU and CPU make that much slower than a desktop does.

Trace (temporary render-world system logging every new `PipelineCache` entry with its frame, plus the first frame
each `Mesh3d` is in view; headless Chromium, `--use-angle=vulkan`, RX 9070 XT, doubles bot match started like the
menu does (through `mode.rs` with its loading screen), 90 s; runs in `context/b110/`):

| build | pipelines under the loading screen | new pipelines mid-match | worst rAF frame after load |
|---|---|---|---|
| WebGPU, before (`gpu4`) | 77 | 12 (frames 240, 331, 608, 681, 1105: first hits, bounces, pop-ups) | 33.4 ms ×6 |
| WebGPU, after (`gpu3`) | 97 | 0 | 16.8 ms |
| WebGL2, before (`gl4`) | 77 + 2 | 10 | 50 ms |
| WebGL2, after (`gl5`) | 97 | 0 | ≤ 34 ms (none over) |

On a fast desktop each mid-match compile costs ~1 dropped frame; it's the same work that freezes iOS Safari. So it
is only the first time each effect shows, not every hit. One WebGL2 run after the fix (`gl3`) had 0.4–1.7 s rAF gaps
that the render-world trace didn't see; a rerun (`gl5`) had none. The machine was under heavy load from other
agents' builds then (load average ~16).

## Fix
`mode.rs`: while a mode's loading screen is up (after its Startup, until `loaded` takes it down), `warm` makes every
`Mesh3d` visible with frustum culling off each frame, and `unwarm` puts back its own `Visibility` (and
`NoFrustumCulling`) at the start of the next frame, before the mode's systems run. Every effect's pipelines compile
behind the black cover, and `loaded` already waits until no pipeline is compiling. The menu → match path (the only
way a browser starts a match) always goes through it. `--play` started directly from the command line has no
loading screen and is unchanged.

## Not verified / not 1:1
- iOS Safari itself: no iOS device here. Measured only in desktop headless Chromium (WebGPU and WebGL2). The fix
  removes every mid-match pipeline compile (0 in 90 s of play with 3 cut-aways), which is what freezes iOS.
  The < 33 ms target on the phone is still for the user to confirm.
- A 90 s doubles bot match covers serves, strokes, volleys, bounces, pop-ups and cut-aways. It may not have reached
  every effect (smash, tornado, rain), but those are spawned at setup too, so the warm-up covers them the same way.
  Anything spawned only later, mid-match, would still compile on its first show (none seen in the trace).
- The traces used a temporary hack so `?play` went through the mode switch like the menu does, plus a pipeline
  logger. Both were removed before commit.
- Seen in passing, not part of B110: in the WebGL2 build the loading screen always waits for its 10 s cap
  (`loading screen down after 600 frames`), before and after this change. Some pipeline stays in
  `waiting_pipelines` there. WebGPU takes it down after 7 frames.
