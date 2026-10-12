# B74d — the net in a close cut-away

**Result:** the original hides the net. It is not clipped. The per-frame court-object pass runs a camera test on one
object, the net, and skips its draw while the camera is close in front of it looking past it. The port now runs
the same test (`crates/hst/src/play/net_hide.rs`).

## How it was found
- **The frame-exact pair from `research/b74b_hunt.py` wasn't needed.** The hunt pauses, then F8, then steps a frame,
  but the stepping is gone from pine.py and F8 doesn't work while paused. The compositor also crashed mid-task
  (Hyprland restarted in safe mode, `hyprctl clients` empty), so no PNG or GS dump could be taken. Reading RAM
  settled the question instead:
  - At vsync 8624, during the c10 cut-away, the net's draw record is classified "drawn" (state 0x100). Poking it to
    culled was never undone, so per-node frustum culling is not what removes it.
  - The VU1 near plane is 0.01 m. That rules out near-plane clipping.
  - The net's court-object record has a "not drawn this frame" byte. It is 1 for the whole cut-away (vsync
    8538–8657) and 0 before and after.
- **Where the byte comes from.** The court-object draw pass handles plant categories 17–20 like this:
  - A record with its "near-camera test" byte set gets an angle test. A record without it gets a sphere sweep near
    the camera, which only runs for records that also have two other bytes clear.
  - A record that fails its test isn't drawn, and its byte is set to 1.
  - The setup sets the near-camera byte only for a category-17 plant whose ground position is within 1 m of the
    court's centre, which is the net.
- **The angle test, in game space (y down).** k = max(1, tan 20° / tan(horizontal fov / 2)). The net is hidden when
  all of these hold:
  - |eye.x| < 7·k
  - |eye.z| / cos(heading) < L·k, where L = 2.0 while a post-point cut-away is on screen and 0.8 otherwise. The
    match-state byte that picks L reads 1 exactly over each cut-away (vsync 8537–8656, 31685–31800, …). The port
    uses `cutaway::Director::showing()` for it.
  - pitch − 0.8·atan(0.7·tan(hfov/2)) < atan((0.91 + eye.y) / |eye.z|), where pitch = atan2(−fwd.y, |fwd.xz|)
  - Three outer gates must also be open. They were open in every state checked.

## Verification
- `research/b74d_check.py 5 8000 36000 1940890` (slot 5) runs the rule against the game's byte on every vsync,
  taking L from the game's own cut-away byte. Result: 28,001 frames, 3 mismatches. The shorter runs before it showed
  that every mismatch falls on a camera-cut frame (8537, 8657, 13907): the draw pass writes the byte a frame after
  the camera block changes. Two cut-aways hid the net: 8538–8657 and 13908–14154.
- A run with L fixed at 2.0 disagreed with the game for the whole match-end shot at vsync 34097+. There the camera is
  2.6 m past the net, facing away, and the game draws the net. That is how L was found.
- Unit tests in `net_hide.rs` use the game's views: both c10 cut-away views hide the net, and the match camera
  doesn't. The match-end view hides the net only at the cut-away distance.

## Not verified / not 1:1
- **No new side-by-side screenshot of the original and the port.** The compositor is in safe mode and has no windows
  (needs the human), so neither `tools/screenshot.sh` nor `tools/shot.sh` can run. The existing pair
  `context/shots/b74b/c10_0.png` / `port_c10_0.png` shows the difference this fixes.
- **The three outer gates aren't ported.** These are a game-mode byte, a player count, and a replay/state flag. When
  they are shut the net is always drawn. They were open over vsync 8000–36000 of slot 5.
- **The sphere-sweep test for records without the near-camera byte isn't ported.** It hides other category 17–20
  court objects whose geometry crosses a small sphere just ahead of the eye. Only the net was looked at, and no
  other prop was seen vanishing in the port's cut-aways.
- **The net's ground shadow decal still draws while the net is hidden.** The game's net shadow is a separate pass,
  and whether it is skipped too wasn't checked.
- **The test uses the tick's view, not the frame-interpolated one, and uses plain f32 math, not the game's libm.**
  This is presentation, so it only has to feel right.
- **Only court 10, slot 5's cut-aways were checked.** Other courts' nets use the same rule and centre placement
  (by code).
