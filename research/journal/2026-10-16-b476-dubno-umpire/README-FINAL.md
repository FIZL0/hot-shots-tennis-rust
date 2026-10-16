# B476 — Dubno Tower: the umpire floating beside her chair

**Cause.** Dubno Tower is the Get a Grip court mod `gag_c20_city`, played on layout 3 (Park), not a disc court.
B476 was filed at 00:25 on 2026-10-16. B433, which seats the umpire on a court mod's own chair, was committed at
00:59 and merged into main at 02:26. Before B433 the umpire was drawn at Park's chair, about 1.4 m in from Dubno's stand
(`c20_ob03_shinpandai` at x = −7.94; the court.glb `umpire_seat` is at (−7.98, −1.74, 0) in game space). That put
her at seat height, beside the stand, toward the net post: what the user saw.

**Reproduced and checked** (`context/b476/run1/`, the same fixed `HST_EYE` on the chair, `HST_WEATHER=0`):
- `singles_no_seat.png`: a copy of the court with its `umpire_seat` node renamed (so the old fallback runs). She
  floats in her sitting pose beside the stand, toward the net post, as in the report.
- `singles.png`, `doubles.png`: the current build with the mods repo's court.glb. She sits in the chair in both.

**Change.** No code fix was needed (B433 has it). Added the guard test `court_mod::every_court_mod_seats_the_umpire`.
For every court mod under `$HST_MODS`, it checks that both match kinds get a seat 1.5–2 m up, outside the doubles
sideline. It passes on the mods repo's 32 court mods and fails on the copy without the seat ("gag_c20_city: no
umpire_seat"). Without `HST_MODS` it skips.

## Not verified / not 1:1
- Not compared with the original: Dubno Tower is a Get a Grip (PSP) court and isn't in Hot Shots Tennis, so
  `tools/screenshot.sh` has nothing to show. The seat comes from Get a Grip's own placement data (`pc_locator`).
- I couldn't see the user's own install. The mod store (HF `FIZL0/opentennis-mods`) has no `gag_c20_city`, and there
  is no installed copy on this machine. A court.glb exported before the mods repo's `umpire_seat` change
  would still fall back to the layout's chair and float (as B433 noted: re-upload or re-export the Get a Grip courts).
