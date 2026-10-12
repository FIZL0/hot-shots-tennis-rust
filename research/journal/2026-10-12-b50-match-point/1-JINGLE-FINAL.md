# B50 — Set / Match Point sound and HUD

## What the original does (decompile, the scoreboard's new-point message 0xe)

- The per-team point check (team 0 then 1, first ≠ −1) is the only consumer of set/match point in the HUD: its two
  callers are the scoreboard's message handler and the banner start it calls. No other score-panel element changes
  at a set or match point. The banner itself was ported in P12b4 (`popups::Finish`, checked centred in B44).
- With the banner armed (a point before that wasn't a set/match point) and not in a replay it plays the system
  bank's slot 9 program 0 key 13 at volume 0x40, then sets that play's speed: **1.0 when the check is 1 (set point),
  1.1 (0x3f8ccccd) otherwise** — match point 2 and 3 alike. (P12b4's note "3 differs only in pitch" was wrong: 2 and
  3 both play at 1.1; set point vs match point is what differs.)

## Port

- `popups::Finish::new_point` queues `jingle = Some(speed)` with the banner; `popups::point_jingle` (FixedUpdate,
  after `simulate`) plays it from the system bank (`SND/SE/SYS/SYS_SE00.XB`, loaded with the banner art), once per
  heard tick (`heard::Played::first`, so rollback re-runs don't replay it).
- `Sound::play_centre` now honours `Play::speed` (every other caller passes 1.0 = the old 0x1000).
- Test: `popups::banner_tests::set_points` — match point queues 1.1, set point 1.0, nothing when not re-armed.

## Not verified / not 1:1

- Not heard side by side with PCSX2: in-match SFX cues only need to match by ear (PLAN), and the key/volume/speed
  are read straight from the decompile.
- No umpire "set point"/"match point" voice: none is called from this path (the umpire's match call is P13's).
