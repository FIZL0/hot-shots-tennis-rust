# B298 — Gecko gets the WebGL2 build

Report: Zen on Windows 11 draws the select and the match broken (exploded skinning, holes in the court, empty
previews, fallback ticker font); Zen on Linux is fine. Firefox ships WebGPU on Windows only, so Linux Zen already
ran the WebGL2 build: the fault is Gecko's WebGPU (D3D12) path, not our scene.

Fix (the task's quick fix): `web/index.html` skips the adapter request when `navigator.userAgent` has `Firefox`
(Zen's UA does too), so Gecko always loads `pkg/` (WebGL2). Checked the selection expression in node for both
cases (`false?.catch` is not nullish, hence the ternary to `null`).

## Not verified / not 1:1
- Not reproduced: no Windows machine here, and Linux Gecko has no WebGPU by default; the root cause inside Gecko's
  WebGPU (skin/index buffers) is not found. Proper WebGPU fix only if someone can run Firefox WebGPU on Windows.
- Not checked on the friend's Windows Zen that WebGL2 now draws right (Linux Zen on WebGL2 is reported fine).
