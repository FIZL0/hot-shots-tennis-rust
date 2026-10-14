# B113 — touch stick no longer selects the page on iOS

The page already had `touch-action: none` and `-webkit-touch-callout: none` on html/body, and the pad's stick/buttons
called `preventDefault` on **pointer** events only. iOS Safari doesn't let a pointer event's `preventDefault` stop text
selection, the magnifier, the callout or double-tap zoom; that needs the **touch** events cancelled (non-passive).

- `web/index.html`: `user-select`/`-webkit-user-select: none` and `-webkit-tap-highlight-color: transparent` on
  html/body (inputs put back to `user-select: text`: iOS won't type into an input under a `none` parent, which would
  break Join by Code's `#codekeys`); `touch-action: none` on the canvas.
- `web/touch.js`: non-passive `touchstart`/`touchmove` plus `gesturestart`/`gesturechange` (pinch), `dblclick`,
  `contextmenu`, `selectstart` cancelled when the target is the canvas or the touch pad. Not page-wide: the mod-zip file
  input, the code-entry input and the stop screen's links sit over/next to the canvas and still need native taps.
  The game reads taps via pointer events (winit), which still fire after a cancelled `touchstart`.
- `web.rs`: nothing needed.

## Not verified / not 1:1
- Not tried on an iPhone (no device here): only `node --check` on touch.js. Whoever has one: play a match from the
  home-screen app, drag the stick while pressing buttons (B165's case), long-press and double-tap the canvas — nothing
  should be selected, highlighted, magnified or zoomed; Join by Code typing and Add mod should still work.
- Front-end/web shell only, no gameplay: nothing to compare with the original.
