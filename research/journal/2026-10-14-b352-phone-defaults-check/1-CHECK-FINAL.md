# B352 — B351's phone defaults checked by eye

Nothing needed fixing. Shots and logs are in `context/b352/` (run3/run4 `pairNN.png`: Low on top, High below;
`menu2/`, `fps4-*`, `bench/`).

## The first run on a phone picks Phone
I ran the browser build (`tools/web.sh`, WebGL2) in headless chromium as a phone (`tools/web_check.mjs`, MOBILE=1:
844×390 at 2×, touch, `(pointer: coarse)` true), with a fresh profile and no `./settings.txt` in localStorage (logged
every 10 s: still null, because nothing is saved until the player changes a setting). Settings → Graphics showed
**Preset Phone · Resolution 100 % · Shadows Low · Anti-aliasing 4x · Frame Limit 60 fps · VSync Off**.

## The Low caster list, on all 11 courts
Native `--play --stage N --shadows low|high --show-fps-detailed`, 1280×720 shots (plus court 1 at 844×390 and
court 4 in the 844×390 browser window). Low keeps every shadow that lands on the court or the run-off next to it:
- c02 the palms beside the court
- c03 the saloon's shadow across the left alley, the umpire's barrel
- c04 the floodlight poles and the fence
- c08 the jungle trees over the court
- c10 the columns and parasols on the right
- the umpire's stand on each court, and the players and the umpire

What goes is far off the court: the benches and parasol bases past the margin (c04, c11) and the spectators' and
walkers' shadows. The view during play looks the same as High. Number of meshes casting, Low → High (Show FPS →
Detailed): c01 85→170, c02 78→112, c03 96→227, c04 80→253, c05 78→109, c06 78→197, c07 71→109, c08 105→178,
c09 73→140, c10 87→157, c11 74→191.

## fps
- At phone resolution on a phone-class GPU: `tools/bench.sh`, HST_GPU=igpu (the 9800X3D's RADV Raphael),
  HST_WINDOW 1688×780 (844×390 at 2×), 4-bot doubles, scale 100 %, msaa 4x, 45 s:
  - court 4: **69 fps at High → 144 fps at Low**
  - court 1: **78 → 142 fps**
- Headless chromium only has SwiftShader (software rendering), with 4× CPU throttle on court 4: median frame
  183 ms at High → 167 ms at Low. That run is CPU-bound and only shows which way it moves.
- Real phone fps wasn't measured (no phone here). B357's report from a real iPhone (shadows Off) is the reference
  for that.

## Seen along the way (not B351's doing)
- Headless chromium under `Emulation.setDeviceMetricsOverride` (dpr 2) gives the canvas only CSS pixels
  (844×390), so the page looks soft in web_check shots. A real iPhone draws at its full 2556×1179 (B357's
  overlay), so this only happens in the test tool.
- In headless, switching Settings → Display → Renderer from Auto reloads into WebGPU, which SwiftShader can't run
  ("Instance dropped in popErrorScope"). This is a test-tool problem too.

## Not verified / not 1:1
- On a real phone: not run. There's no device here, so all of the above is a headless emulation, a native
  phone-sized window, and the iGPU as a stand-in for a phone GPU.
- fps before B351 at Low with all casters (instead of High): not measured. That would need a second build of
  B351's parent; the High vs Low numbers above are the gain a phone gets from the new default.
- Low vs the original game: not compared. Shadows Low is a remaster setting, and High stays as before.
