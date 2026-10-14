# B330 — iOS Safari Share button stops the page on the ISO prompt ("The game stopped: Script error.")

## Cause
The page's global `error` listener (web/index.html, boot script) passed every error event to `fail()`. "Script error."
is the masked message a browser gives for an exception thrown by a script from another origin (HTML spec "muted
errors": no `error` object, no filename, no line). None of the page's scripts are cross-origin. When you tap Share,
iOS Safari runs its share extensions' JavaScript preprocessing scripts on the page, so the throw comes from one of
them, and the page stopped on it. This matches the reported text exactly ("script error", without a file or panic
line). No page handler is involved: index.html has no pagehide/blur handler. Its visibilitychange handlers (`settle`,
`awake`) can't throw: the wake-lock rejection is caught. touch.js has no visibility, pagehide or blur handlers.

## Fix
`addEventListener('error', …)` now drops an event that has no `error`, no `filename` and the message "Script
error." It logs a `console.warn` (the warning stays in Web Inspector). Every other error still stops the page as
before, including the game's own panics, which arrive with an error object.

## Check
`node tools/web_script_error.mjs [webkit]` takes the boot markup and script out of index.html and loads them with a
script that throws, once from another origin (port) and once from the same origin, in headless Chromium and in the
system WebKitGTK MiniBrowser. Playwright's bundled WebKit build can't start here: libicudata.so.66 is missing.
- before (HEAD's index.html): cross-origin → FAIL "The game stopped: Script error." in both browsers (the bug).
- after: cross-origin → running; same-origin → stopped with its own message, in both browsers.

## Not verified / not 1:1
- Not tested on a real iPhone: this is an unattended run with no iOS device or Web Inspector. The cause is
  inferred: the same "Script error." text comes from a cross-origin throw in WebKit, and the Share sheet runs extension
  scripts. If the message still shows on iOS, check Web Inspector's console for the `hst:` warning or a different
  error.
- Front-end/web only, no gameplay: nothing to compare against the original.
