# B288 — result camera far too zoomed out (online doubles, far end)

## Cause
Not the cut-away or replay camera: the user's shot is the winners' ceremony / result page. The players stand on
their ceremony marks (the service lines) but the picture is the match camera's eye (~41 m behind the court) with
the ceremony clip's last field of view (hand_99/win_99 end on 75°, 1.309 rad, 3 m from the mark): a wide shot of the
whole jungle with the players tiny.

`ceremony::show` (and `intro::show`) write the camera's Transform inside the mode's Update. The base app's `orbit`
(main.rs) also writes it from the `Orbit` rig every frame, and Bevy had the two unordered: the base Update schedule's
ambiguity report listed `orbit and plugin::{{closure}}` (the mode runner `run(w, Update)`). Whenever the executor ran
`orbit` after the mode runner, the rig's transform (the match camera) landed over the ceremony's, keeping the clip's
fov. Local `--play` runs happened to run `orbit` first, so local near and far ends both looked right
(context/b288/local_8.png, far_8.png, far_16.png with a forced swapped ending); the online build/timing hit the other order.

## Fix
- mode.rs: the mode's Update runner runs `.after(crate::orbit)` (the ambiguity is gone from the report).
- play.rs `camera`: writes the rig's transform itself (`Orbit::transform`, factored out of `orbit`), so the match
  camera has no one-frame lag now that `orbit` always runs first. Only `play::camera` writes `Orbit` in any mode.

## Checked
HST_AUTOPLAY + HST_STATS forced endings, near end and (with a temporary `swapped = true` poke, not committed) the far
end: ceremony close shot and result page as before, match camera unchanged (context/b288/fix_grid.png).
tools/check.sh -p hst: 303 passed.

## Not verified / not 1:1
- Not reproduced online: an online doubles match needs two machines/sessions played to a real match end; the fix
  removes the scheduling race that produces the user's picture, checked by Bevy's ambiguity report, not by an
  online run.
- Not compared with the original winning from the far end in PCSX2: the ceremony framing itself (turned clip,
  offset) was already right on both ends locally; this task only removed the overwrite.
