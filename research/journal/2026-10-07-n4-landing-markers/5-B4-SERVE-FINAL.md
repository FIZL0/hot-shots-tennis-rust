# B4 — a marker on the serve

## Cause
The user's note ("on serve there should be no red indicator", 2026-10-07 18:05Z) came 25 minutes before N4b
merged. At that point the red marker was already gated (`players == 1 || shots > 1`, cleared in `next_point`), but
the yellow smash search still started on **every** strike, serves included. So when a CPU served to P1's team, the
search could place `smash_p` on the serve. N4b added the original's gate to the search too ("no search on the
serve"). That is the only marker that could appear on a serve, so B4 was fixed by N4b.

## Checked
- Current code: the port ran with P1 human (keys sent to the window with `hyprctl dispatch sendshortcut`), and a
  temporary log printed whenever the red marker was set. Over 2 points it was up only with `shots >= 2`, never in
  `Phase::Serve`. Every way back to a serve (point, fault, let, change of ends) goes through `next_point`, which
  resets `Marks`. The yellow marker now has the same gate.
- The original: N4 already showed no red marker on the serve (slots 4 and 5) and showed it from the return on.

## Not done
- No new PCSX2 capture: the original's behaviour is already in N4's notes.
