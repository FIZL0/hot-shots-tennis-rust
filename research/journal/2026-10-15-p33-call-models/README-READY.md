# P33 — umpire call models without a second 3D camera

Problem (M91 build 2014 log, iPhone 15 Pro): every call (Fault, Out, Net, Double fault, Change Sides) cost ~1.1 s of
30–40 ms frames, render +5–7 ms. The calls were drawn by their own `Camera3d` (`popups.rs` CallCamera, order 1,
CALL_LAYER) over the match: a second Scene view of the full-size HDR, MSAA scene image (Bevy: MSAA writeback blit of
the scene into its samples, its own opaque/transparent passes with MSAA load/store, resolve, upscale copy, clustering,
GPU preprocessing). On a tile GPU at 2556×1179 that is several full-screen passes; the model itself is a few dozen
vertices.

## Change
- The call models are now drawn by the match camera (`Orbit`) in its own passes: their root sits under a `CallSpace`
  entity that is a child of the match camera, with scale (k, k, 1), where k = tan(match half-fov) / tan(overlay
  half-fov). Both fovs widen by the same `SHOWN_ASPECT.max(1/aspect)` factor, and the camera aspect is the same (both
  render into the scene image), so every call vertex lands on the same pixel as through the old 50° overlay camera,
  and clips at the same near plane (z unchanged). The calls stay screen-fixed through cut-aways, replays and the
  intro's rolled flyover (they're in view space).
- New material `effects::OnTop` (ExtendedMaterial like `Fx`): depth compare Always, no depth write, no hardware depth
  bias, so the calls draw over the scene as the separate camera did. `depth_bias` = `CALL_SORT` (1e5) sorts them
  after every scene blend (landing markers are at 1e4). All call materials are Blend/Add, so before this too they
  never depth-tested each other: the order among them is the same sort.
- `NotShadowCaster`/`NotShadowReceiver` on the call meshes: they are on layer 0 now, which the sun sees.
- The overlay camera stays for the intro's "Start the Match!" and the prize screen's GAME SET banners
  (`OnCallCamera` marker on their roots). It now runs only while one of those is visible (`call_camera_on`, after
  visibility propagates). **Regression fixed**: since P23 the overlay camera only ran while a call played, so the
  intro banner had been invisible (old build: no "Start the Match!" at 4 s; new: shown; `context/p33/run1/intro-grid.png`).
  The intro and the prize screen still pay for the second view while their banner shows (~1–2 s each, outside rallies).
- `HST_CALL=k@f` (debug hook): call model k (`CALL_MODELS` order) is held on its frame f all match, for `--shot`
  checks.

## Verification (desktop, Vulkan)
`tools/shot.sh` old binary vs new, `HST_CALL=1@30` (Out), `3@30` (Fault), `5@30` (Change Sides), with no court and
with `--court 2`, 1280×720 window (2048×1152 shots, `context/p33/run1/`): the calls are identical. 33–728 pixels
differ (>24/255), and all of them are on the players' rackets/limbs (idle animation timing between runs; diff image
`diff-5.png`). `tools/check.sh -p hst`: 390 passed.

## Needs the human
On the iPhone (same settings as build 2014: court 2, 100%, Low, 2x): play until a Fault/Out and a Change Sides; the
fps log should show no 30–40 ms run while the call is up. Also check the intro's "Start the Match!" banner frame time
(the overlay camera now runs there, which it hadn't since P23).

## Not verified / not 1:1
- Device frame time: no iPhone here; the fix targets the suspected per-view cost and wasn't measured on a tile GPU
  (Metal gives no GPU timestamps, see M91 journal). Listed in plan/HUMAN.md.
- Windows narrower than 4:3: `HST_SIZE='600 900'` still gave a 4:3 capture (the window rule kept it landscape), so
  the tall-window case was only checked by the formula (both fovs widen by the same factor), not by a shot.
- With Bloom on (G3, never in the original mode), the calls now get bloomed with the scene, and SMAA (if on) now also
  smooths them. The overlay camera had neither. Rendering only has to look right; not chased.
- Not compared against PCSX2: the change keeps the old overlay's picture (pixel-identical), which an earlier task
  already matched to the original by eye.
