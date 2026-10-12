# B74 — skyboxes not lined up

## What the original does
The court draw (every frame) draws the four backdrop models — 0 `_sky`, 1 `_clo`, 2 `_acc`, 3 `_bg` — with
matrix rotY(yaw) and translation **main view eye + (0, lift, 0)**. They follow the camera position, not the origin.
- yaw = court envir `envir_cNN.dat` f32 at 0x450 + 4i (season row 0, see below) + 0.0 − hole envir yaw (degrees,
  `envir_cNN_h01.dat` +0x58) × π/180, wrapped to ±π (`hst_sim::ps2` add/sub/mul).
- lift = hole envir f32 at +0x10 + 4i. No hole file: lift 370, hole yaw 0.
- Row: the draw reads the season row for clo/acc, row 0 for sky/bg, but the load copies row 0 over every season row,
  so it is always row 0 (copy seen in RAM, s05.bin). The same copy covers the light rows → **B74a**.
- Gates already in the port: clo only in weather < 2, acc not in weather 3/5, only the first sky.

## Port
- `gs::backdrop(envir, hole, i) -> (yaw, lift)` (+ unit test `backdrop_yaw_lift`).
- `main.rs`: backdrop entities get a `Backdrop { yaw, lift }`; the `backdrops` system (PostUpdate, before transform
  propagation, after every camera move) sets them to eye + lift, rotY(yaw), in the court root's game space.
- `HST_EYE=ex,ey,ez,fx,fy,fz,dx,dy,dz,tan` (`fixed_eye`): holds the camera at a view read from the original's main
  view (eye, forward, down, tan of the horizontal half-fov) for same-camera shots.

## Verification
- RAM, `research/backdrop_check.py` on `context/ram/{npc_c01,npc_c02,s03,s05}.bin` (courts 1, 2 = resort/beach, 4,
  10): every sky, clo and bg matrix the game built sits exactly at eye + lift with the port's rotation (e.g. beach:
  sky yaw −0.4662 lift 170, clo −0.1869 / −67, bg −0.4662 / 344). None of these courts has an `_acc`.
- Screenshot, court 10 (slot 5 cut-away, static view): `context/shots/b74/orig5_0.png` (PCSX2) vs `port5_0.png`
  (`HST_EYE` = that view), overlay `blend5_full.png`: temple, clouds, bg hills and walls coincide. (PCSX2 shows the
  game widescreen-patched: compare against the port's full 16:9 frame, not a 4:3 crop.)
- Beach port shots: `port_c02.png` (dump's view), `port_c02_up.png` (tilted up): bg hotels/hill sit on the sea horizon.

## Not verified / not 1:1
- Rotation uses glam sin/cos, the game uses its VU0 sincos approximation (sky tiny-angle sin 3.45e-4 vs 1e-4 on
  court 10): sub-milliradian, not bit-exact.
- Beach and courts other than 10 have no original screenshot (no save state there; needs a menu drive) → **B74b**.
  The hard-edged dark band at the top of `port_c02_up.png` is unverified. → B74b: untextured draws were 2× bright
  (gs.wgsl), fixed; beach, courts 1, 4, 10 sky screenshots match (2-B74B-FINAL.md).
- Courts 3, 5–9, 11 checked only by code path (no RAM dump of them); `_acc` (courts 3, 9) never seen in RAM.
- Season row = 0 from the decompile and the RAM copy; no non-zero-season singles state checked.
- clo/acc weather draw gates only as already ported.

## B74d — the net in a close cut-away (3-B74D-FINAL.md)
The original hides the net (it isn't clipped). It runs a camera test on the plant at the court centre: the eye within 7 m of
the court's long axis, within 2 m of the net plane in a cut-away (0.8 m otherwise), and the net top above the lower view.
Ported as `play/net_hide.rs` and checked per frame against the game's flag (slot 5, 28,001 frames).
