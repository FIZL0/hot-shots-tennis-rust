# 1 — a hidden tab steps the game on a timer, except in a match

## Why it stopped

Bevy 0.19's winit runner on wasm runs `app.update()` only once per `RedrawRequested` (its
`ran_update_since_last_redraw` guard; a winit wake-up without a redraw just asks for one), and winit's web
`request_redraw` is `requestAnimationFrame` only. A hidden tab gets no animation frames, so everything in the app
stopped: B255's install queue (the next install is started by a Bevy system on the previous one's event), the
lobby's keep-alives, the menus. The downloads' own futures (`spawn_local`) went on, but nothing picked up their end.

## The fix (`web/index.html`, `web.rs`, one line in `play.rs`)

- `index.html` wraps `requestAnimationFrame`/`cancelAnimationFrame`: while `document.hidden` and no match is on, a
  frame comes from `setTimeout` (browsers throttle it to about one a second in a hidden tab), with its id negated so
  a cancel finds the right one. winit asks for a new frame on every event, and it sends `Occluded` on
  `visibilitychange`, so the switch happens right as the tab hides and back to real frames as it shows.
- `web::in_match` sets `hstMatch` on the page (only on change); `play::plugin` calls it in `Last` with whether
  `Game` exists. In a match the hidden tab gets no frames, as before: the match holds, the others see B196's
  "Waiting for …" (`play/recover.rs`).
- Each hidden step is a normal update: `Time<Virtual>` clamps its delta to 250 ms; nothing in menus depends on it.

## Check (`tools/web_check.mjs`, new `WEB_HIDE=s:ms`: minimise the window for ms, count the frames stepped)

Headless Chromium, WebGL2 build (`context/b260/run1`, `run2`):

- main menu, hidden 6 s: 6 game frames while hidden (about one a second), 57 in the second after showing.
- `?play` (a match on), hidden 6 s: `hstMatch` true, 0 frames while hidden, 30 in the second after showing.

## Not verified / not 1:1

- A store download / Download all queue in a hidden tab wasn't run end to end (needs the live store); it rides on the
  game loop stepping, which the menu check shows.
- Lobby keep-alives with two peers and one hidden weren't run; same mechanism, ~1 s steps against `SILENT` = 3 s.
- Chrome's intensive throttling (hidden > 5 min, no audio or WebRTC) slows chained timers to once a minute; a
  download's queue then advances that slowly. A worker-driven timer would avoid it; not done (lazy first).
- In-match keep-alives while hidden are left out on purpose: the match pauses, and B196's away notice relies on the
  hidden peer going silent; a "pause for everyone" message is B162's.
- Hidden steps still render (about once a second); not skipped.
- WebGPU build and Safari/Firefox not run (the patch is plain JS on rAF; winit's `Occluded` on `visibilitychange`
  is the same code on every browser).
- Presentation/runtime only: no sim, RNG or match timing change (a match never steps while hidden).
