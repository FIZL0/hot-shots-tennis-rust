# 1 — the ceremony fov never reached the camera (FINAL)

## The suspects, ruled out
- **`(fov / 2).tan()`**: correct. In the original's result-object camera update (clip state), the global fov in
  degrees is set to the .CAM FOV key × 180/π with no ×2, and its tangent is taken of deg × π/360. The match cameras
  store a half angle and double it, so the .CAM key really is the full vertical angle.
- **Doubles marks**: correct. Read from a save state at the ceremony's start (slot 8, saved by
  `research/p26c_ceremony.py 5 context/b246/orig_d 20000 24`): the players' ceremony matrices sit at (0.75, 0, −6.4),
  (−0.75, 0, 6.4) turned, (−0.75, 0, −6.4) and (0.75, 0, 6.4) turned, the same as `ceremony::plan`.
- **Camera path**: the original's camera globals (eye, rows, fov) per frame match `ceremony` `view()` at the same
  clip frame (e.g. ~60 frames in: eye (0.007, −0.508, −2.212), fov 30.24°).

## The bug
`ceremony::show` wrote the right `Projection` fov in PostUpdate, but `Camera::clip_from_view()` stayed at the match
camera's last fov (~0.26 rad, tight) for the whole ceremony. A probe after `CameraUpdateSystems` showed the projection
changed and its own matrix right but the camera's clip matrix stale. Ordering `show` before `CameraUpdateSystems` did
not help; why Bevy's camera system skipped the update from PostUpdate wasn't found.

## Fix
`ceremony::show` and `intro::show` (same pattern) now run in Update, after the match `camera` system (via
`.after(character::animate)`, the end of play's chained Update set), so their transform and fov are the ones Bevy
turns into the clip matrix. `show` only writes the Orbit camera: no sim state, RNG or timing touched.

## Checked (shots in `context/b246/run3/`)
- Doubles win, 16:9 (`sheet.png`, port top, original `orig_d/s0_024/048/072` bottom): the same framing through the
  clip, both players whole in frame, temple behind. Tall 720×1280 (`dtall_21.png`): both figures in view.
- Singles win (`s_21.png`, `stall_21.png`) and a singles loss (`l2_*.png`, human on team 1): figure framed at 16:9
  and tall.

## Not verified / not 1:1
- **Singles win and loss vs the original**: only the bot-only doubles win was recorded; the singles win and lost
  clips are checked by eye in the port only (no capture of the original's singles or lost ceremony).
- **Characters differ** in the doubles side-by-side (port default line-up ≠ slot 5's), so framing was compared,
  not the figures.
- **Intro**: moved to the same schedule for the same reason; not re-shot against the original; a `--play`
  shot at 3 s (`intro_3.png`) is already past it and shows the normal match camera.
- **Root cause in Bevy**: why a Projection written in PostUpdate before `CameraUpdateSystems` didn't reach the clip
  matrix wasn't pinned down; the fix avoids it rather than explains it.
- A ball trail showed in one loss shot (`l2_22.png`) — the match's last flight, not part of this task.

## Tests
`tools/check.sh` passes except `play::netplay::tests::matches::rollback_desync_is_saved_and_play_goes_on` ("the
desync's note", netplay.rs `reported`), which fails the same with B246's change reverted: already broken on main
(netplay area, last changed by B197), not this task's. Filed as its own task; B246 ticked and committed by hand.
