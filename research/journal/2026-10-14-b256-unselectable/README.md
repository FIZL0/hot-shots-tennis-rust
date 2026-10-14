# B256 — nothing on the page selectable on phones

B113 only covered html/body (CSS) and the canvas/touch pad (JS). Still selectable: the boot/ISO page text, `#boot`'s
links, anything outside the canvas — `user-select`, `-webkit-touch-callout` and `touch-action` aren't reliably
inherited (touch-action never is), and `selectstart`/`contextmenu` were only cancelled on the canvas and pad.

- `web/index.html`: `*, *::before, *::after` get `user-select: none`, `-webkit-touch-callout: none`,
  `touch-action: none`; `input, textarea` get text selection, the default callout and `touch-action: manipulation`
  back (token paste, join code, ISO/mod file pickers).
- `web/touch.js`: `selectstart`, `contextmenu`, `dblclick`, `gesture*` cancelled everywhere (capture phase) unless
  the target is an input/textarea/contenteditable; a `selectionchange` that leaves a non-empty selection while no
  input has focus clears it (iOS select-all from the long-press menu). Touch events' preventDefault stays on the
  canvas/pad only, so links and inputs keep their taps.

## Not verified / not 1:1
- Not tried on an iPhone (Safari or home-screen app) or Android Chrome: no phone in an unattended run. Needs the
  user's check from the task's done-condition. Desktop-only change otherwise: `node --check web/touch.js`.
- Front-end only; nothing to match against the original.
