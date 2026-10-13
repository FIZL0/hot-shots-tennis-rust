# B218 — online: each machine's camera follows its own player; no free camera

## Cause

Online every machine sets the match up with all the room's humans (`netplay::flags`: remote seats are `REMOTE` pads),
so `g.humans` counts two or more and `reset_positions` takes the original's several-humans path: the match camera
keeps its end (`cam.turned` stays false, behind −z) and never turns round. The camera (`hst_sim::camera`) frames only
the near team (`near_team`: player 0's team on its side), so a player at the +z end, or after a change of ends, was
at the far end of the picture and could run off its top. Not a regression in B49/B164/B174's camera work: online
never had a per-machine camera.

The shared camera can't simply turn per machine: `Game::cam` is in the state hash, and the reaction voices and
foot steps test what it sees (they draw the sound RNG), so each machine's own view would desync the match.

## Fix

- `play/own_camera.rs` (new): online only (a `Netplay` resource), each machine steps its own copy of the match
  camera on the same scene each tick, turned for its own first player (`end < 0` = +z half, as a solo game's
  `reset_positions`), cutting at the shared camera's placements and whenever its player's end changes. It is shown
  while the shared camera shows its plain step (`g.cam.view == expect`); a cut-away, instant replay or the
  change-ends glide (which write `g.cam.view`) show the shared camera's shot as before. Rollback re-runs don't step it
  again (newest lockstep log tick only).
- `Game::cam_of` (online only, the same on every machine, in the hash): per player, the first player of the machine
  driving it. `pad_run` now takes the player and flips the stick by that player's machine camera end
  (`own_camera::flipped`), so controls stay screen-relative to each peer's own picture. Offline `cam_of` is empty
  and `pad_run` reads the shared camera's eye as before: replays unchanged.
- Presentation readers of the view use the shown one: the `camera` system, the ball outline/shadow
  (`ball_shadow`), the net hide (`net_hide`), the foot puffs' facing (`foot_fx::draw`).
- Free camera removed (user 2026-10-13): `CamMode`/`CamState`, the C/Select toggle in matches, the arrow keys /
  right stick turning it; `turn_left`/`turn_right` gone from `controls.rs` (an older `controls.txt` naming them is
  read without errors). The `camera` action stays: it is the character select's hand swap (C / Select).
  README, `controls.txt`, `plan/REFERENCE.md`, `web/touch.js` comments updated.

## Verified

`pair.sh OUT T [doubles]` (this folder): host + guest native processes on this machine, both mashing
(`HST_NET_MASH`), each `--shot` at T s. Shots in `context/b218/` (`s20`, `s70`, `s140`, `d30`, `d100`):
- singles at 20 s (first game), 70 s (after the first change of ends: host now at +z), 140 s (after the second):
  each machine's picture is behind its own player, who is near the bottom of the frame;
- doubles (host 2 seats, guest 1, a bot) at 30 s and 100 s (after a change of ends): each machine behind its own team.
No desync / panic in any log. `own_camera` unit test: `cam_of` for singles and doubles seat orders.

## Not verified / not 1:1

- No original to compare against: the original has no online play; the rule (one-human camera on this machine's
  first player) is the task's. Local multi-human games are unchanged (camera keeps its end, as the original).
- The change-ends glide is still the shared camera's (it glides to the shared camera's end), then cuts to this
  machine's own view when it ends, if that is the other end. A per-machine glide was not done.
- Overlays that take `g.cam.view.fov` (markers, popups, bodyhit, cheer marks) and the sun flare read the shared
  camera's fov; the own camera's can differ slightly with framing (positions come from the drawn camera transform,
  so they stay right; only their size scale could be off by the fov difference). Not measured.
- Shots were taken at fixed times with mashing pads, not continuously through a rally; "own player in view" was
  checked by eye on 5 pairs of shots.

## Wrap-up

The code landed in main with the WIP commit (merged as `task/B218`); this run only adds `pair.sh` (left uncommitted
in slot s4) and ticks the task after the tests pass. Shots rechecked by eye (`s70`: host behind +z, guest behind −z,
each with its own player near the bottom).
