# B73: player markers during a change of ends (FINAL)

## What the original does (slot 5, 4-COM doubles; `research/b73_marker_ends.py`, `research/b73_hold_shot.py` → `context/b73/`)

- The marker show flag (balloon manager +0x725) turns **on** on the change-ends phase's first tick (gm+0x55 = 1,
  vsync 14154). It stays on for all 81 ticks and through the next serve set-up, then goes off at the toss (serve tick 91).
  This repeated across runs 1 to 4.
- The tags are drawn, not just flagged. The screenshot at change-ends tick 30 (`context/b73/run5/ph1_t30.png`) shows
  "COM" over all four players at their new spots, mid camera glide, under "Change Sides".
- So the original shows the markers during a change of ends. The user's report was made before B49 removed the walk.
  It matches the original's timing; only the rule in `shown()` was wrong, and it was wrong the other way: it hid them
  in `Phase::ChangeEnds`.

## Port

- `play/markers.rs shown()` now also returns true in `Phase::ChangeEnds(_)`. `change_ends_placement` puts players
  at the new spots with `prev = pos` on the phase's first tick, so a tag never streaks from the old end.
- Test: `play/change_ends.rs` glide (both the bot and Grand Palais variants) asserts `markers::shown` on every
  change-ends tick.
- App shot `context/b73/app/t47.png` (`--play --singles --chars 6,1 --shot-at 47`, autoplay) shows a COM tag
  under "Change Sides".

## Tool notes

- With copy 2, `tools/screenshot.sh`'s F8 never landed and the L3+R3 fallback came about 3 s later, which is longer
  than the phase. Several screenshot.sh calls at once also all copied the same newest snap.
- Writing gm+0x58 does not hold the phase.
- `b73_hold_shot.py` presses L3+R3 through vpad at the exact tick instead.

## Not verified / not 1:1

- Whether the original's tags are hidden on any single tick at the phase boundaries wasn't checked. The flag log
  shows no gap, but the draw was only seen at one tick (30).
- The flag was checked only for bot players (COM). A human's "1P" tag uses the same flag and draw, but no
  human-side change of ends was recorded.
