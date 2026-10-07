# N4 landing markers — part 2 (final)

## Done
- `effects::LandingMarks` (effects.rs): `taguchi/other/chakudan_p` (red) and `smash_p` (yellow) from `PCDATA/PCCG0.XB`
  as effect models (no ANI: rest pose). Red: looping pulse, started once, ticked only while shown. Yellow: one
  120-frame play, restarted when the smash search places its first point. Both at (x, 0, z).
- play.rs: the red cylinder (`LandingMark`, `landing`, `mark_landing`) is gone. `strike` sets the red marker at the
  shot's aim when `players == 1 || shots > 1` and starts a `SmashSearch` for the humans on the receiving team (smash
  top `pair(64,0)`, middle `pair(64,1)`, character 0's) on a predicted path (launch + 14 steps, y up).
  `smash_frame` grows it 15 entries a fixed tick (to 2 bounces, cap 600) and searches; the yellow goes once the live
  ball bounces. Both clear on the next strike and at the next point's serve.

## Verified
- Real game (my PCSX2 copy, PCSX2's own F8 screenshot sent with Hyprland `send_shortcut` to the hidden window;
  `grim` region grabs don't work for a window on another workspace): slot 5 (bot doubles) and slot 4, a shot every
  0.2–0.4 s, in `context/shots/n4/s5`, `s4`, `s4b`. The red marker: a red soft blob with a light pulsing ring, not on
  the serve, up from the serve return on, moves at each strike, stays through the point-over until the next serve —
  as ported.
- Port (`HST_AUTOPLAY=1 hst <iso> --play --stage 1 --shot … --shot-at 9…15`, `context/shots/n4/app_*.png`): the same
  red blob and ring at the aim, about the same size against the service box. Yellow (with the human filter
  temporarily dropped so the bots count, not committed): a larger yellow soft blob at the smash point
  (`y_13.png`).

## Not verified
- The yellow marker on the real game: it needs an opponent's high ball onto a human (slot 4: P1 whiffs or bots
  never lobbed in two rallies). Its search and heights follow the decompile and its unit tests.
- Exact frames/positions numerically (no marker RAM watch); the 1-frame question from part 1 stays open.
- Practice (`players == 1`): candidate side pick and the red marker's 30-frame timeout.
