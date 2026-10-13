# 1 — not reproduced; port matches the original (FINAL)

## Where the marker comes from
`strike` builds the search from the shot being struck (`g.flight` and `g.shot` are set before `g.marks.smash`, and
the heights are the receiving humans'). `smash_frame` grows and searches it in `start_effects` on the same tick, and
`LandingMarks::tick` starts `smash_p` the tick the first point is placed. Nothing reads the previous shot's state.
This code is the same as it was when the bug was filed (1a32e51, after N4a/N4b).

## Port, headless
A bot match (chars 6, 1, court 0), with a temporary hack that counted player 0 as a human receiver (not committed):
- Every opponent lob (class 1, kind 3, peaks 5.2–7.5 m) placed its point, and `marks.start` fired 4 ticks after
  that lob's own strike (e.g. strike at t1051, start at t1055). That is N4a's placement timing on the original's
  forced lob (5th frame, path 75). `LandingMarks.smash_at` held the point until the bounce.
- No other shot (serves, normal hits, smashes, slices; peaks ≤ 2.7 m) placed one.

## Original (`research/b12_lob_mark.py`, slot 4, P1 human pressing △ through `tools/vpad.py`)
Two runs of 2400 frames, `context/b12/r1.out` and `r2.out`. P1's lobs (path peaks 3.7–4.2 m) placed **no** marker,
and neither did any CPU hit after them. The marker is only for a human on the receiving team (N4a's per-slot table:
CPU slots are −1), so your own lob never shows one.

## Conclusion
No one-hit lag in the port. The original and the port agree in both directions: the opponent's lob shows the marker
on that lob, and your own lob shows none. The report most likely came from expecting a marker on one's own lob, so
the marker first showed up on a later shot coming back to you. That later shot was a lob or a high ball, which the
original marks too.

## Test
`crates/hst/src/play/smash_marker.rs` `lob_places_its_own_marker`: in a 2-minute bot match, the search `strike` sets
up for the receiver, run over each shot's own flight, places a point for every lob and for no other shot (≥ 2 lobs).

## Not verified / not 1:1
- The full pipeline with a real human receiver isn't in the committed test. It needs a human seat that returns
  shots, and autoplay drives no human seats. It was checked once with the temporary receiver hack above.
- No port screenshot of the marker during that lob. The draw side is unchanged since N4a's and B91's side-by-side
  screenshots.
- The original wasn't recorded with two humans on one team (the midpoint), or in practice mode.
