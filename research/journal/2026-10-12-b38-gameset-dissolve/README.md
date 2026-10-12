# B38 — the "Game, Set, Match!" screen pops in

User (2026-10-08): the game/set/match screen appears abruptly instead of playing its animations. Read as the match's
end: the cut from the match to the winners' ceremony (the "Game, Set, Match!" banner) and from the ceremony to the
result page. The in-match game/set board already fades in over 14 ticks (P12b2) and was seen fading in the port in
B93, so nothing changed there.

## What the original does

The result object's state 0 → 1 calls a method on the game's one global screen-transition object (pointer at
`0x1ea5d0`); the page's draw leaves out the bottom bar while it runs and the step takes no input and doesn't count
the page timer. `research/b38_transition.py` logs the object every vsync (`context/b38/run1`, `run2`): counter +0x8
starts at length + 2 and drops one a tick, length +0xc, 1/length +0x10, running flag +0x14.

- Match → ceremony: length 16 (the same one runs at every camera cut between points).
- Ceremony → result page: length 60, started 3 ticks before the ceremony clip ends.

`research/b38_hold.py` holds the counter at a value while F8 is taken (`context/b38/hold/t061…t001.png`): it is a
dissolve — a still of the last frame before the cut drawn over the live screen at alpha ≈ counter/length, the new
scene (page, live camera, confetti) running under it.

## The port

`crates/hst/src/play/match_stats/dissolve.rs`: `Dissolve::cut(len)` screenshots the window (Bevy `Screenshot`, as
`mode::leave`), the caller waits for the capture (10 frames at most) before changing the scene, then the still is
drawn full-window over everything but the widescreen bars, alpha min(1, t/len), t from len + 2 down one per 60 Hz
tick. `ceremony::begin` waits on a 16-tick cut, the result page's opening on a 60-tick one; while the page's dissolve
runs it takes no input, doesn't step (timer, pulses, blink) and draws no bar. While the page waits for its capture the
ceremony's plates and banner stay up (they were tied to the clip running).

Seen in the port (`tools/shot.sh` with `HST_STATS=18 HST_KEYS=…shot=…` via an `HST_BIN` wrapper,
`context/b38/port2/`): the court dissolving into the ceremony, the ceremony (NARROW WIN, A Team) dissolving into the
result board, the bar appearing after the fade — as `context/b38/hold/`.

Also: `tools/screenshot.sh` on a copy whose PCSX2.ini binds Screenshot only to the pad (copy 4) waited 9 s for an F8
that never lands; it now presses L3 + R3 when F8 gives nothing in 3 s.

## Not verified / not 1:1

- The dissolve's exact alpha curve and blend: by eye from held-counter screenshots (alpha ≈ counter/length, matches
  t034 ≈ half); the transition's 3.5 KB draw wasn't ported (rendering, look-right rule).
- The page's dissolve starts when the clip ends, not 3 ticks before it, and the still is taken 1–10 frames later
  than the original's (the port waits for the capture); invisible since the port holds the clip's last view.
- After the page opens the original's camera moves on (orbit); the port still holds the clip's last frame (P26c gap,
  not this task).
- The 16-tick dissolve at every camera cut between points (cut-aways, replays) isn't done here: the port cuts. That
  is in-match presentation (B48 / cut-aways), out of this task's scope.
- No pause or online-match check: the dissolve only runs at the match end, after the sim's last tick.
