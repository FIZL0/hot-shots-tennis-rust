# B539 — iPhone Safari tab: full screen as far as Safari allows

## What iPhone Safari allows
- No Fullscreen API for page elements on iPhone (iPad has it; touch.js's first-tap `requestFullscreen` covers iPad,
  Android, desktop). Video-only `webkitEnterFullscreen` is no use for a canvas game.
- `minimal-ui` in the viewport meta was dropped in iOS 8. `scrollTo()` from script doesn't collapse the bars.
- What works: a *user* swipe that scrolls the page down. Safari then hides its bars (landscape: completely; portrait:
  down to the small address pill). They come back when the page returns to the top or the user taps the bottom edge.
- No bars at all: the home-screen web app (already set up: apple-mobile-web-app-capable, black-translucent, B314
  safe-top). M123 is the tutorial for that; the swipe card names it too.

## What changed (web/index.html)
- `html.swipe` (iPhone, `navigator.standalone === false`, game up, not the site): html/body scroll, body is the
  screen's height + 240 px; html/body/#swipe allow `pan-y`, the canvas and the touch pad keep `touch-action: none`
  so a match never scrolls the page.
- `#swipe` card (z 15: over the touch pad, under Copy Link's sheet and errors) while the page is at the top: "Swipe up
  for full screen", Add to Home Screen hint, a tap closes it until the phone is turned.
- B333's `fit()` no longer scrolls back to the top in swipe mode (that would bring the bars back).
- `tools/web_swipe.mjs`: headless Chromium posing as an iPhone tab / home-screen app / Android checks the page's
  side (8 PASS).

## Not verified / not 1:1
- On a real iPhone: no iPhone here. Whether Safari (iOS 18/26) hides its bars on that swipe with this page, and that
  the canvas resizes cleanly as they go (B333's innerHeight tracking should follow) is untested; the check above only
  covers the page's own logic in Chromium.
- Bars brought back by a tap on the bottom edge mid-match while scrolled: the card doesn't reappear (to not cover a
  rally); the player has to scroll the page to the top (status-bar tap) to get it back. Deliberate, untested on device.
- Not in the original (a website feature).
