# B534 — local humans on one team keep the camera on their side

With 2+ humans on this machine all on one team (2P doubles vs two COMs: players 0 and 2, room seats 0 and 1),
the shared match camera (`Game::cam`) keeps its end (play.rs `next_point` only turns it for ≤1 human), so after a
change of ends the humans played from the far end.

## Change
`own_camera.rs`: `link` now also runs offline. `local(g, pads)` picks the first human as owner when there are ≥2
humans (by `Pads::slot_of`) and all of them share an end; it sets `Game::cam_of` (humans → owner) and inserts the
same `OwnCam` online uses. From there everything is the online path: the turned display copy, B486's own
change-ends glide, the listener's end, and `flipped` turning every human's stick with that camera. `Game::cam` is
untouched (state hash, replays). One human or humans on both teams: `local` is `None`, nothing changes.
play.rs not edited.

## Test
`own_camera::tests::local_humans_on_one_team_stay_near_through_changes_of_ends` (~1 s with the ISO): bot doubles,
`link` run with 2 connected pads → owner 0, `cam_of = [0,1,0,3]`; 1 human, 4 humans, and 1P+3P (both teams) →
none. Through two changes of ends: every glide tick has the eye behind player 0; every plain tick after has the eye
on player 0's side and the sticks turned with the picture, while the shared camera does have the humans far.

## Not verified / not 1:1
- Not played with two real pads in the running remaster (unattended run, no second controller); covered by the
  headless test only. Presentation-only per PLAN (camera), so no PINE proof needed.
- Which human the camera follows when both are on one team: the first by player number (player 0, i.e. 1P), as
  online's `cam_of` picks; the original has no such mode to copy.
