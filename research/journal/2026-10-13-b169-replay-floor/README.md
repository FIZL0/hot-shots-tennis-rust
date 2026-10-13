# B169 — floor in front of a low replay camera vanishes (FINAL)

## Cause
Not frustum culling and not the near plane. B157's VU1 guard band (`gs.wgsl` `GS_GUARD`, every court model) set a
vertex's position to NaN when it was behind the eye (w ≤ 0) or outside GS 2048 ± 1280. That drops every triangle
using the vertex. With the camera low and close to the ground, the big floor triangles under and beside the camera
have such vertices, so the floor in front of the camera disappeared and the clear colour showed through.

Repro: `HST_EYE=3,-0.4,13,0,0.15,-0.989,0,0.989,0.15,0.577` (eye 0.4 m up at the near baseline), passed through an
`HST_BIN` wrapper to `tools/shot.sh … --stage N --play --shot-at 8`. Stage 1: the bottom third of the screen was
the clear colour. With `GS_GUARD` off, the floor is back.

## Fix (`gs.wgsl`)
1. The band is tested against |w|, as the VU's `CLIP` does (reject pass 0x37b: `CLIP.xyz` + `FCAND`). The port's
   extra "w ≤ 0 → drop" rule is gone. That alone fixed stage 1, but stages 2–5, 9 and 10 still lost big ground
   triangles whose far vertices are in front of the eye but outside the band sideways.
2. Ground is exempt: a vertex with an upward world normal (y > 0.7) under 1 m. The GPU clips those triangles
   instead. "Upward normal" alone brought stage 8's near-camera foliage back (the leaves face up), so it also has
   to be under 1 m.

## Check (`context/b169/`)
- `run3/low_s1..11.png` (`sheet3.jpg`): low camera on every court; the floor is whole on all of them.
- `run3/play_s7/8.png`: the default match view on stages 7 and 8 still matches B157's
  `context/b157/fix_s07/fix_s08.png` (no w-house2 wall, no foliage).
- `run4/` (`sheet4.jpg`): the default match view on courts 1–6, 9–11 looks normal.
- `tools/check.sh -p hst`: 182 tests pass.

## Not verified / not 1:1
- **Ground exemption:** the original has no "ground" exemption. Its reject pass flags any vertex outside its clip
  volume, so its own floor would also lose big near triangles under a camera this low. The original's replay
  cameras were not checked against PCSX2 at such a height (driving a replay to a low angle in PCSX2 wasn't tried).
  The task asks for a floor that never drops out, and rendering only has to look right.
- **Thresholds:** 0.7 and 1 m are picked by eye, not taken from the game.
- **`CLIP`'s z test:** not ported. The clip matrix the reject pass loads (vf17–20) wasn't identified in the VU1
  memory dump, so the near/far planes it tests aren't known.
- **The app's own replays** are still off (`instant_replay.rs` `REPLAYS`, B127), so the repro uses `HST_EYE`
  instead of a live replay camera.
