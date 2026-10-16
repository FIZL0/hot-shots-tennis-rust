# B469 — iPhone 13 web app: page reset on a smash

Report: on an iPhone 13 (4 GB) the web app's page reset when the player hit a smash. Guess on the task line: something
(effect, SFX, voice, replay buffer) loaded on first use spiking memory.

Result: nothing is allocated on a smash, on the first one or any later one, native or in the browser. No CPU or GPU
asset, pipeline, mesh, image or heap growth from the smash's contact through its landing (crater, dust, sparks, marker).
The reset is the general footprint B468 is about. `?log` now logs each time the wasm heap grows, so a phone's
Web Inspector can show where it grows. Iterations: [1-FINAL.md](1-FINAL.md)
