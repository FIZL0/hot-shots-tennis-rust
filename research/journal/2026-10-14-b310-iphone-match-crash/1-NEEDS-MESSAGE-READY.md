# 1 — needs the crash message (human)

Checked `web/index.html`: the error box path for a panic is `console.error` (Bevy's panic hook) → `lastError` →
the `error` event → `fail()`, which shows the `panicked at file:line: message` text trimmed of stack lines. A
`createBindGroup`/`Validation` error instead restarts on the WebGL2 build (B244), so a crash that *stops* the game
with an error box is not that one.

Needed from the human (plan/HUMAN.md): on the iPhone, play 1v1 on this phone (WebGPU build) until it crashes and
copy or photograph the whole error box text ("The game stopped: …"). With that, the panic's file:line names the
callback and this task can be fixed.

## Not verified / not 1:1
- The crash itself: not reproduced — needs an iPhone (Safari WebGPU); none available to an unattended run.
