# F0 — Uncapped frame rate (FINAL)
- Present mode `AutoNoVsync` by default; `--vsync` selects `AutoVsync` (main.rs).
- Everything tick-driven is drawn between the last two ticks with `Time<Fixed>::overstep_fraction()` (a):
  - positions/ball: already blended.
  - facing: new `Player.prev_facing`, Quat slerp, so the 22.5 degree snaps stay the tick targets.
  - motion time: `character::animate` draws `time - speed*(1-a)` (the tick adds speed); works for the `--character` viewer too.
  - camera: new `Game.prev_view`; eye and forward lerped, fov lerped. A `remember` system at the head of the FixedUpdate chain
    snapshots facing + view; `reset_positions` sets `cam_cut` so the cut frame does not blend.
  - balloon fades: alpha blended from `balloon_alpha(age-1)`, 0 before the first.
- HUD is plain text with no fades: nothing to blend. N1e crossfade weights will need the same treatment once ported.
- Verified: `hst-sim` untouched (git diff); release autoplay run on stage 1 reached ~780-840 fps with no errors.
- Not verified: visual 60 Hz stepping was not inspected by eye (unattended run) — worth a human glance.
