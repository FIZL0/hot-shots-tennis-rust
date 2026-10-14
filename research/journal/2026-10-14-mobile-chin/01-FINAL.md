# B333 — mobile page opens cropped with an empty "chin"

User report: on a phone the page sometimes opens with an empty bar along one edge; turning the phone to portrait and
back fixes it. Cause (most likely): the canvas sized from `html, body { height: 100% }`, a size iOS Safari can report
before its toolbar / home-screen status bar has settled, with nothing re-fitting it until the next real resize (the turn).

## Change
- `web/index.html`: a small script at the top keeps `--app-w`/`--app-h` = `innerWidth`/`innerHeight` (px) and scrolls
  back to 0,0 on resize, load, pageshow, focus, visualViewport resize, the tab shown again, and after `orientationchange`
  / `screen.orientation` change plus a few delayed re-checks (50 ms … 2 s), also right after load. Not the visual
  viewport's height: an open keyboard (Join by Code) shrinks that and must not shrink the game.
- CSS: html/body height `var(--app-h, 100dvh)`; the canvas is `position: fixed` at 0,0, sized by the vars (fallback
  100vw / 100dvh), so a page scrolled by the browser can't shift it either. winit's ResizeObserver picks the new
  box up, Bevy resizes the surface.
- `web/touch.js`: the touch layer uses the same box instead of `inset: 0`.
- `crates/hst/src/web.rs` `page_size`: the canvas's own client rect first (touch/mouse mapping matches the canvas even
  if `innerHeight` lags), the window's inner size as before when there is no canvas.

## Checked
- `cargo check -p hst --target wasm32-unknown-unknown` builds (web-sys features Document/DomRect/Element added).
- Headless Chromium (a CDP script in the scratchpad, mobile emulation, a stand-in canvas appended to body like winit's):
  the canvas box follows 844×340 → 844×390 (toolbar gone), a scroll + portrait/landscape turn (back at top, full size),
  and a desktop 1280×720.

## Not verified / not 1:1
- On a real iPhone/iPad (Safari, home-screen app): no device here; the bug is intermittent and Chromium's emulation
  doesn't reproduce iOS's late toolbar settling. Whether the delayed re-fits catch every case needs the user's phone.
- `tools/web_touch.mjs` (full game in a phone viewport) not run: needs a fresh wasm build and a disc-imported profile;
  only the page-level sizing was checked.
- Front-end/page layout, not gameplay: no original to compare against.
