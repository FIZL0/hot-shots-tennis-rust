# B439 — web app froze after loading mods on the first opens

## Setup
Fresh Chromium profile (own origin, port 8795, so OPFS starts empty), ISO imported, 13 Fore! character mods
written into OPFS `mods/`, no cached select cards (cold cache). Runs through `tools/web_check.mjs` with Vulkan ANGLE,
WebGL2 and WebGPU builds, unthrottled and with 4× CPU throttling + `MOBILE=1`; CPU profiles via `WEB_CPUPROFILE`.
Captures/profiles in `context/b439/` (not in git).

## Findings
1. **WebGL2 startup quit (separate regression, fixed).** Since the "8bit graphics" commit (hud_gamma's 3D cameras
   render to an `Rgba8Unorm` main texture), bevy_render asks for an `Rgba8UnormSrgb` *view* of it. WebGL2 has no view
   formats (`DownlevelFlags::VIEW_FORMATS`), so wgpu raised a validation error and Bevy quit — the page stayed on
   "Reading mod …" on every WebGL2 browser, mods or not. This landed after B439 was filed, so it's not the user's
   original freeze, but it looks exactly like it. Fix: `prepare_view_targets` only adds the sRGB view when the
   adapter supports view formats (`third_party/bevy_render/src/view/mod.rs`). WebGL2 build now starts with 0 errors.
2. **Cold-cache face bakes (the reported freeze).** With no cached cards, the main screen bakes each shown mod's face
   in the background, one per turn: the page reads the mod (async, 2–3 s, not blocking), then the browser (no loader
   thread) runs the whole `mods::load` inside one frame, then bakes. Measured: ~500–670 ms frames per mod for ~50 s
   with 13 mods (throttled), ~300 ms unthrottled. Profile: ~80% of each load was building *every* donor and own
   motion clip, while a card needs only `inspect::MOTIONS` (pose + two idle motions) — disc characters already load
   only those. Cards persist in OPFS `previews/`, so later opens skip all of it: matches "first 5–7 opens bad, then
   fine" if the user reloaded during the hitching.
   Fix: `mods::load_with(…, only)` loads only the listed motions (donor via `disc_motions_with`, own clips filtered by
   id before conversion); `previews.rs` uses it with `inspect::MOTIONS`. Test `a_card_load_has_only_its_motions`
   (filtered load = full load on visual/faces/parts/clips it keeps). Result: per-mod frames ~200–250 ms throttled.
3. Boot frame (~3.5 s throttled) and `main_menu::setup` (~1 s) are menu art/textures, not mods (B409's items).

## Not verified / not 1:1
- No hard, permanent hang reproduced: Linux Chrome WebGL2 (throttled and not) and WebGPU (unthrottled) all keep
  running and bake all 13 faces in one ~75 s open. The user's freeze is explained by the long per-mod frames (and
  after 16:46 by the WebGL2 quit), but not confirmed on their machine/browser.
- Not tested: phones / iOS Safari, Firefox, WebGPU throttled, memory pressure on small devices.
- Store-installed mods (listing fetch, listing picture for the card) not tested: needs the live store over the
  network; the test mods have no `store.json`.
- Court and umpire mods, and ~40 installed mods, not tested (13 character mods only).
- Remaining hitches: mods with long own win/loss clips still spend up to ~500–900 ms in one frame (own clip
  conversion ~470 ms worst, motions.glb JSON parse ~140 ms). Splitting `mods::load` into between-frame steps
  (B409's `jobs::spawn_steps`) would need the load cut into phases; not done.
- Menu/front-end/rendering are ours to design; no 1:1 needed.
