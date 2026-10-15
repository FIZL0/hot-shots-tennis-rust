# B315 — post-point cut-aways missing in 1v1 on one phone

## Cause
1v1 on one phone (B118, `SIDEWAYS` 2-player local) shows this machine's own camera (`own_camera::OwnCam`) whenever the
shared match camera shows the plain match view. `own_camera::plain` decides that each tick by comparing `g.cam.view`
with the shared camera's own step, but it was only ordered `.after(simulate)`. The cut-away (`cutaway::step`), the
replay camera and the change-ends glide write `g.cam.view` later in the same tick, and Bevy was free to run `plain`
before them: it saw the plain view and kept showing the own camera, so the cut-aways never appeared. Online
(same `OwnCam`) had the same race; matches without an own camera (1P vs COM, 4 players) don't use the path, hence
"other mobile matches play the cut-aways".

Second, smaller: `own_camera::shown` treated `instant.ticks == 0` as "before the match's first tick" (B230's intro),
but that count restarts at every serve set-up, whose tick still holds the cut-away shot (the game's one-tick late
switch). Every point's last cut-away tick was replaced by the match view (own camera or shared). Now it also needs
the match's time (`match_stats.frames`) to be 0.

## Fix
- `plain` ordered after `cutaway::step`, `replay_camera::step` and `change_ends::step` (the first and last made
  `pub(super)` for it).
- the intro check in `shown` as above.

## Verified
`own_camera::tests::a_cut_away_takes_the_own_cameras_picture`: bot singles with an own camera on player 1 (other end
from the shared camera, so the pictures differ); through three cut-aways the picture is the shot from the shot's
second tick on (its first only picks it), including the serve set-up's held tick, and the own view shows otherwise.
Fails without the ordering fix (own match view instead of the shot), and on the held tick without the intro fix.

## Not verified / not 1:1
- Not run on the phone/browser build itself: no device in an unattended run. The test drives the same systems
  natively; the race is a schedule-order one, so it applies to wasm the same way.
- No PCSX2 comparison: cut-away presentation is "feels right" (PLAN), and the change doesn't touch the sim or RNG.
