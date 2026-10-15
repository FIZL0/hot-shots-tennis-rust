# B401 — Get a Grip court mods: selectable, with pictures, playable

Task (user 2026-10-15): the Get a Grip courts should be selectable on the court select, show a picture like the
disc courts, and work properly when played on. The first attempt (`old/B401`, its own journal of the same name
there) built a loader in parallel with M8f6; this redo sits on M8f6 (`play/court_mod_match.rs`, `mods_court.rs`,
`main_menu/court_mods.rs`) and ports only its extras.

## What landed

- `wip/B401` merged (the port onto M8f6): `court_mods::Pics` reads each court mod's `previews/court.png`
  (`Tex::CourtMod`); it stands in for the layout court's picture on the strip tile, the select's backdrop, the
  confirm court card and the lobby card (`court_picture`, `Menu::court_pic`); a guest's room value is the installed
  mod's (`room_court_value`). The intro's title plate and the pause screen's venue say the mod's name.
  `HST_COURT_PREVIEW=1` hides all 2D for the preview shots.
- Strip labels: a mod's tile was labelled name-over-game on one squeezed line each (Fore's long names were
  unreadable at 16:9). Now `court_tile_lines` breaks the name over two lines at its middle space, as the disc
  names are; the game stays on the name card.
- Previews for the 21 Fore / Out of Bounds / Open Tee 2 court mods (all 37 court mods now have one), with
  `tools/court_previews.sh`. The moment matters per layout (the flyover is the layout court's): 4 s for most;
  layouts 7 and 9 at 3 s (at 4 the match camera with the 3D "Start the Match!" banner, or no court in view for
  layout 9); OOB c06 2 s. Times written in the script's header. The pictures are written into
  `../opentennis-mods/out/mods/*/previews/` (git-ignored export output there: a re-export must keep or redo them).
- `tools/court_previews.sh` no longer needs its own env wrapper (`tools/shot.sh` passes `HST_*` on).
- Test `court_mod_pictures_and_labels` (picture only for mods that have one, guest room value, tile label split).
  "Plays properly" is M8f6's `a_court_mod_is_seen_and_heard_and_never_moves_the_match`.

## Not ported from old/B401

- Stand clipping near the camera (`GsUniform::court_clip`): main now has M8f6's `__clear` set pieces that hide
  while between the eye and the court, which covers the same case (c05 hoop, c09 end stands); c09 at the match
  camera looks clear (`context/b401/run11/match.png`).
- Window-of-twelve strip: main's M8f6 strip already windows 17 tiles around the pick.

## Checks (shots in `context/b401/run9`–`run11`)

- Previews grid of all 37 (`run9/grid_all.png`).
- Court select on a mod at 16:9, 1600×600 and a 700×1000 window (`run11/courts-mod-*.png`): picture never stretched,
  letterboxed in a tall window exactly as the disc courts are (`run10/courts-tall.png`).
- Confirm card with the mod's picture and name; intro plate "Central Arena" on gag_c09 (layout 0); pause venue;
  a match on gag_c09 at the match camera.
- `tools/check.sh -p hst court_mod`: 11 passed; full run in done.sh.

## Not verified / not 1:1

- Previews are our own intro flyover shots, not Get a Grip/Fore/OOB art; picked by eye.
- Lobby card with a mod picture not shot (needs an online room); covered by the shared `court_picture` path and
  the `room_court_value` test only.
- No comparison with Get a Grip itself (no PSP emulator run): the courts are judged by eye in our own match
  (open follow-ups from the first attempt: B403 flyover, B404 gimmicks/lens flare, B405 per-court issues).
- In a very tall window the strip's labels get small (same as the disc courts' labels; the strip shares the
  window's width).
