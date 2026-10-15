# B376 — no post-point cut-away when hosting online (Steam Deck)

## Cause
Online, the picture is this machine's own match camera (`play/own_camera.rs`, B218) whenever the shared camera
(`Game::cam`) shows the plain match view; `own_camera::plain` decides that each tick by comparing `g.cam.view` with
the shared camera's own step. It was only ordered `.after(simulate)`, while the systems that put a cut-away, replay
or change-ends view on the shared camera (`cutaway::step`, `replay_camera::step`, `change_ends::step`, all
`.after(simulate).before(start_effects)`) were unordered with it. Both touch `Game`, so Bevy's executor runs them
one after the other in whichever order it picks; when `plain` went first it saw the plain view, so the picture
stayed on this machine's camera through the whole post-point: no cut-away, "the camera stays in the same place".
The order the executor picks is not fixed by the schedule, which is why it showed on one machine (the Steam Deck
host) and not others. Offline matches have no `OwnCam` (except B118's sideways phone), so they were unaffected.

## Fix
`plain.after(super::start_effects)`: after all three. Test
`play::own_camera::tests::the_own_camera_waits_for_the_cutaway_and_replay_cameras` builds the FixedUpdate schedule
with the cut-away/replay-camera/change-ends plugins and the own camera and asserts Bevy reports no unordered conflict
between `plain` and anything; with the old ordering it fails, listing all three.

`netplay::tests::matches::rollback_survives_a_tab_out` (B299: the guest's picture mustn't flip to the shared camera
for a round or two after a catch-up) then failed with 1- and 2-round shared runs: with `plain` ordered it now sees
the shared camera's real non-plain views, and in that harness (instant replays on) a replay's first tick cuts away
from the plain view for one tick, and a replay can start two ticks into a cut-away. The test now skips rounds where a
cut-away shows or a replay plays; the rollback-flip check stays as it was (passed 3 runs in a row).

Side finding: the replay's tracking shot 6 gives exactly the match camera's eye, so online the guest's own camera
stays on during it (fine: same plain view, from the guest's end).

## Not verified / not 1:1
- Not run on a Steam Deck or in a live hosted online match: no second machine/peer in an unattended run. The fix is
  a schedule-order fix shown by the test; a hosted match on the Deck should now show the cut-aways.
- Instant replays themselves are off in the app for every match (`instant_replay::REPLAYS = cfg!(test)`, user
  2026-10-12, until B127 is fixed); this task doesn't turn them back on. Once they are, their cameras go through the
  same `plain` check, which now waits for `replay_camera::step` too.
- With instant replays on (tests only), the guest's picture still shows the shared camera for one tick at a replay's
  start (the replay's first-tick cut); not chased, replays are off in the app.
- No PCSX2 comparison: presentation only (the cut-away port itself is unchanged).
