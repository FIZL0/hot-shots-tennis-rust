# 1 — streaming install, measured

## Desktop (`HST_LIVE_INSTALL=oob_pc12_anya … live_install --nocapture`, debug build, live store)
- Progress lines from 0% to 90% over about 39 s, then "Checking Anya...", then `Ok("Anya")` at 44.6 s with 239
  files. The log is in `context/notes/b204-live-desktop.txt`.
- Peak RSS (VmHWM) was 140 MB for a 489 MB zip and a 525 MB unpacked mod.
- `package_then_install` passes. It streams in 64 KB chunks, checks that a wrong id and a flipped byte fail and leave
  the old copy and no `.download.zip`/`.import` behind, then installs.

## Browser (headless chromium, WebGL2 on SwiftShader, `tools/web_check.mjs`; runs in `context/b204/run*`)
- Install from Browse mods (run7): pressed Enter at 28 s. "Downloading Anya... 246 / 489 MB (50%)" etc. counted up
  until about 76 s, "Checking" until about 110 s, "Saving" until about 120 s, "Added Anya" at about 124 s.
- Memory: wasm memory went from 427 MB to 936 MB (+509 MB). On web the unpacked mod is meant to stay in modfs
  memory, so that is about 1× the mod. JS heap peaked at 184 MB, its usual level at startup. There was no zip copy
  and no staging clone, so the peak stays well under mod × 2.
- **Slow-install finding:** the first browser run took about 100 s to check and more than 100 s to save. Each await
  (a Blob read, a `setTimeout` yield, an OPFS write step) waits for a whole frame, and on SwiftShader a frame is
  60–100 ms. Fixes:
  - One read per zip entry: the header, the data and 256 bytes of room for the extra field. A second read happens
    only if the extra field is longer than that.
  - Checks yield only after 50 ms of work, not after every file.
  - `hstModSave` writes 8 files at a time instead of 4.
  - Result: checking took about 34 s instead of about 100 s, and saving about 10 s instead of more than 100 s.
- **Loading installed mods (startup and roster)** with Anya installed:
  - Startup: "mods read from OPFS in 776 ms" (new `console.info` in index.html) for 525 MB. `add_mod_file` now copies
    once into the shared slice; before, it made a `to_vec()` and then an `Arc` copy. No startup frame was longer than
    without the mod (the longest was 1.9 s, the same as with no mods).
  - Roster: the first visit bakes Anya's card once (`m-oob_pc12_anya-0-0.png`), along with the stock cards on this
    new profile. After that, the custom tab (Tab) costs one 1.2 s frame, her model load. Nothing else stood out, so
    no more changes there.

## Not verified / not 1:1
- WebGPU pkg (`web/pkg-webgpu`): not rebuilt or run. The changes don't touch rendering, and only the WebGL2 pkg was
  built (`WEB_FAST`-style, no wasm-opt).
- Browser memory: measured as wasm `memory.buffer` plus `performance.memory`, not the OS process RSS or the
  task manager. OPFS writes may buffer in the browser process, which this doesn't see.
- Timings come from SwiftShader in headless chromium. On a real GPU, frames and so each await are about 4× shorter.
  No real-GPU browser run was done.
- A picked zip in the browser (`import_mod(File)`): same code as the store install after the download, but not
  driven in the browser, because headless file picking for the mods input wasn't scripted.
- `bad_mods_leave_nothing` skipped its body: `opentee2_pc16_lauryn` isn't in the local mods.
