# P58 — long rAF gaps with an idle main thread (hst-log 10)

Log: `~/Downloads/hst-log (10).txt` (Chrome 152, Linux, RX 9070 XT, WebGL2/ANGLE-on-GL, Hyprland desktop).

## Two different causes

**Menu gaps (24 s at 13→37 s, then 5.1, 1.2, 2.6, 0.9 s): the window wasn't on screen.** During them:
no pipeline went ready (29 ready at 7.1 s, the next +2 only at 52 s), no images/meshes uploaded, game updates 3–10 ms,
the 50 ms page timer never late (no `blocked`), no `visibilitychange`. The GPU process had nothing new to do. Gamepads
connect only at 51.9 s (Chrome exposes them only after a pad button press on a focused page): the user came back
then. Reproduced on this desktop: a probe page (rAF counter + WebGL clear + 50 ms timer, Chromium
`--ozone-platform=wayland`) launched on the hidden Hyprland workspace `special:p58` got 4 frames, then **none for
15 s**, while `visibilityState` stayed `visible`, `hasFocus()` was `false` and the timers ran on time. Hyprland doesn't
send frame callbacks to a surface nobody sees, and Chrome doesn't count that as hidden. So: occlusion, not ours, but
it froze the game outside a match too (B260's timer stand-in only covered `document.hidden`), so downloads and the
lobby's keep-alives stopped while the window was on another workspace.

**The 4 s before the match's loading screen drops (108.2→112.2 s): Chrome's GPU process.** It comes right after the
last of ~150 pipelines (MSAA 4 variants) went ready (`ready +3 (150 in all)` at 108.14 s). The links themselves showed
as `blocked` + `pipeline queue` render stalls (wgpu's GL backend checks the link status at once). The 4 s after them
has no `blocked`: the main thread is idle and the page's own rAF stops too. It's the first frames that draw with the new
programs: ANGLE/Mesa (radeonsi ACO) build the real shader variants on first draw in the GPU process and the frame
isn't acked until they're done. Ours to shrink: P54 (MSAA off on WebGL2, merged) cuts the variants; P55 (warm the
match's pipelines ahead of time) has to *draw* with them once, not just create them, or this 4 s comes back at the
first frame (note added to P55).

## Changes

- `web/index.html` (B260 shim): outside a match, an unfocused visible window whose frame hasn't come in 250 ms gets it
  from a timer (cancel handles both). A focused window or a match is unchanged (a match holds, as when hidden). The
  page sets `hstTimerFrame` per frame (timer or real).
  Probe with the real shim code: hidden workspace → real rAF stops at 7, the loop goes on at 4 frames/s from timers;
  headless (focused) → 421 real = 421 loop frames, 0 timer frames.
- `?fps` log: `raf gap N ms (window unfocused)` when the window lost focus during the gap, plus `window blur`/`window
  focus` lines; the watcher uses the native rAF so it still shows the real gaps behind the timer frames.
- `stall.rs`: a timer frame's wait isn't logged as a game stall (otherwise every frame on another workspace would be);
  a gap ≥ 1 s says how many pipelines went ready how long before it (`; 3 pipelines went ready 0.1 s before it` = GPU
  process compiling first draws). `web.rs::timer_frame()` reads the flag.

## Not verified / not 1:1

- The fixed page isn't checked with the full game in the browser on the hidden workspace (the probe ran the shim code
  extracted from index.html, not a web build): building and running the web build headed on the user's desktop for
  this wasn't worth the window popping up; the next `?fps` log shows it (`(window unfocused)` gaps, no game stall spam).
- The GPU-process explanation for the 4 s load gap comes from the log (timing after the last pipelines, idle main
  thread, no new uploads), not from a Chrome GPU trace (`chrome://tracing` needs the human at the browser).
- That the user's 24 s gap was another workspace/window rather than a covered window isn't knowable from that log;
  both give the same signature, and the new `(window unfocused)` tag will tell next time.
- Rendering/menu only, nothing to match against the original.
