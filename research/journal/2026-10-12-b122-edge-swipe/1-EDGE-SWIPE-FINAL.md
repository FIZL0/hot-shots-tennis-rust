# B122 — edge swipe leaves the game (web)

`web/touch.js` (end): once the page is past the ISO import screen,
- non-passive capture `touchstart` that begins within 20 px of the left/right edge is `preventDefault`ed (iOS Safari's
  edge back/forward swipe). Pointer events still fire, so the touch stick and buttons near the edge keep working.
- on the first `pointerdown` in game a spare history entry is pushed (after a user activation, so Chrome's
  history-manipulation intervention doesn't skip it); `popstate` re-pushes it, so a back gesture that still gets
  through (Android Chrome's system back gesture can't be cancelled) only pops the spare entry.

## Not verified / not 1:1
- Not tried on a real iOS Safari, iOS home-screen app or Android Chrome: no devices in this unattended run. Only
  syntax-checked. Someone with a phone should swipe from both edges in a menu and a match.
- The import page isn't guarded (as asked).
