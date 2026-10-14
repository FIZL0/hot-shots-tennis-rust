# B310 — iPhone 1v1 crash mid-match (WebGPU build)

Report: a 1v1 "on this phone" match crashed mid-match on an iPhone, WebGPU build; the error box showed a Rust panic
raised by a JS error in a browser callback, but its message was pushed off screen by the stack lines. Since then
(commit eae38e6d) `web/index.html`'s `fail()` keeps only the panic's file:line and message (first six lines, no
`wasm-function`/URL stack lines), so the next crash shows it.

Nothing to port without that message: the panic site can't be guessed from "a JS error from a browser callback"
(touch, resize, orientation, audio, gamepad and OPFS callbacks are all candidates), and fixing a guess goes against
*No guessing details*. Desktop Chromium WebGPU doesn't reproduce it (no iPhone or Safari here).

Iterations: [1-NEEDS-MESSAGE-READY.md](1-NEEDS-MESSAGE-READY.md)
