# P53 — Auto graphics preset (phones hold 60)

The friend's iPhone 13 ran the web app at ~30 fps on the Phone preset (67 %, Low shadows, 2x MSAA). Now a first run
gets the **Auto** preset: a pick from the device, then timing in each match that steps down until 60 holds.

## What landed
- `graphics.rs`: `Graphics::auto` (settings.txt `auto_graphics = on|off`, kept through `keep` like the window).
  - `Device::here` gathers the signals: browser (`web::device`, page's `hstDevice` in index.html) CSS screen,
    DPR, `hardwareConcurrency`, Chromium's `deviceMemory`, WebGPU `maxBufferSize`; iOS app `hw.machine` +
    `hw.memsize`; Android app `Build.MODEL` (JNI, `android::model`) + sysconf memory; cores everywhere.
  - `pick`: Steam Deck → its preset (no Auto: hand-tuned, SteamOS may cap below 60); desktop → Default + Auto;
    phone/tablet → Phone stepped down `Device::steps` times. Biased high, since timing only steps down:
    iOS model iPhone16+ / iPad13+ 0, iPhone14–15 1, older 2; no model: known memory ≤3 GB or ≤4 cores 2, ≤4 GB 1;
    Safari (no memory) by iPhone screen size (390×844 etc. = iPhone 12–14/16e → 1; pre-X, XR/11 → 2; 14 Pro+ → 0);
    WebGPU maxBufferSize ≤ 256 MB → ≥1; > 4.5 MP screen (tablets) +1.
  - `cheaper`: one rung: speed-ups on → scale 85/75/67 → shadows Low → MSAA halved to off → scale 50 → shadows off
    → mesh 25. From Phone that is Low, then down to Lowest. Every rung keeps 60 (a 1–59 cap becomes 60); never 30.
  - `auto_tune` (Last, only while a match is on: `play::match_on`): skips the first 3 s, times 2 s windows (mean
    frame time without the slowest 5 %; gaps > 0.25 s reset the window), steps down after each window under 93 %
    of min(60, display Hz), waits 1 s after a step, done after two held windows / the bottom / 16 s timed. Each
    match checks again (heavier court), never steps up. Steps saved to settings.txt (`main_menu::save_graphics`).
    Off in `--shot`/HST_BG runs (`NO_AUTO`). Drawing only: no sim/RNG access.
- Settings → Preset: "Auto" after Phone (re-picks for this device and turns timing on — also how an existing
  player, e.g. the iPhone 13 with an older settings.txt, opts in). Changing any graphics row by hand turns Auto off;
  picking another preset too.

## Tests
`graphics::tests::auto_picks_and_steps_down_at_60` (ladder, picks, tiers), `graphics::tests::auto_timing` (skip,
step per slow window, settle, hitch/hidden gap, 30 Hz display), `options::tests::presets_and_switches` (Auto row).
Native, wasm32, Android and iOS builds check clean.

## Not verified / not 1:1
- Not run on a phone (no device here): the tiers (model/screen/memory → steps) are reasoned from public device specs
  and the two data points we have (iPhone 13 Safari ~30 at Phone; M91 iPhone 15 Pro app at 100 %/2x 40 fps), not
  measured. The timing corrects a too-high pick within a few seconds; a too-low pick is never raised.
- Safari's `hardwareConcurrency` is ignored unless memory is known (WebKit's value isn't the device's).
- iOS app: Low Power Mode / thermal throttling aren't detected; on a 60 Hz display a throttled phone would step down
  (arguably right). The web build uses the page's measured refresh, so a 30 Hz Low Power Mode browser isn't stepped.
- Not played end-to-end in a live match here (HST_BG runs disable Auto, and settings.txt sits beside the user's ISO);
  the per-frame logic is unit-tested on `Tune::frame_time`.
- Native px for the tablet rule is only known in the browser (apps use the model/memory).
- Not an original-game feature (remaster-only settings): no PCSX2 comparison applies.
