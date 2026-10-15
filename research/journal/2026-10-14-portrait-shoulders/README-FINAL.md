# B313 — portrait touch pad: L1, R1, Select, Start

The buttons were never removed in portrait: `web/touch.js` lays them out along the top edge at `top: u*.3` with no
`safe-area-inset-top`. With `viewport-fit=cover` and the `black-translucent` status bar, a phone held upright draws
the page under its status bar / notch (~47–59 px on an iPhone), and the buttons (u = min(12vw, 7vh) ≈ 47 px → 14–47 px
from the top) sat entirely behind it. In landscape the notch is at the side, so they showed there.

Fix (`web/touch.js` CSS): in portrait the four small buttons move to the bottom, `u*4.1` above the safe-area bottom,
a row over the face buttons and under the letterboxed picture — the same place B118's 1v1 pads already used. The
top-edge positions (landscape) now also add `env(safe-area-inset-top)` (0 in landscape on current phones).

Checked: headless chromium with a harness page loading `touch.js?touch` at 390×844 and 844×390
(`context/B313/shot-*.png`): portrait shows L1 SELECT START R1 in a row above △, clear of the 4:3 picture band and of
the face buttons; landscape is as before.

## Not verified / not 1:1
- Not tried on a real phone (unattended run): the notch/status-bar cover is inferred from the CSS (no top safe-area
  inset under viewport-fit=cover), not observed. The new position is at the bottom, so it doesn't depend on it.
- Not run through the full wasm build (`tools/web_touch.mjs` needs the imported-disc profile and is landscape-only);
  the harness page uses the same touch.js and CSS.
- Front-end/touch layout is ours to design (PLAN.md), so nothing to match against the original.
