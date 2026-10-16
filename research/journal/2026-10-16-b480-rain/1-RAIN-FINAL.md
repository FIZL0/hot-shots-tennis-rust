# B480 — rain: duration, look, cost

## How long the original's rain lasts (decomp)
The match set-up's schedule builder (the function reading the per-court weather odds table, ported as
`hst_sim::weather::schedule`) draws **one rain spell per match** with the court's rain chance. It starts at a
random game in `(2·games + 1)·(2·sets − 1)` and lasts `min + r % (max − min + 1)` games, with a cloudy game on each side
unless the court's no-border flag is set. The schedule index is the game counter, which steps on the end-of-game message
when the result kind is > 0 (a game or a set was won). So the weather changes once per game, and B377 already confirmed
the one-frame switch on the next game's first set-up. The port matches all of this; nothing to change.

Per court, from GAME.BIN (now pinned in `hst-data/tests/effects.rs`):

| court | rain chance | games | heavy |
|---|---|---|---|
| 1 | 20 % | 2–3 | no |
| 8 | 40 % | 1–2 | yes |
| 9 | 15 % | 1–2 | no |
| 11 | 30 % | 2–4 | yes |

Other courts never rain. So the user's "about one game" is right for courts 8 and 9. On 1 and 11 the original does
rain for 2–4 games. Kept as the original (the duration is gameplay: rain's agility ×1.5).

## Why it looked wrong
The original's streaks are 20 big quads (0.6 × 0.4 m, `rain00` tiled 20× across) half a metre in front of the eye. They
sit in a 0.9 × 0.6 × 0.2 m box that wraps round whenever the camera moves, with fresh random positions on every wrap.
The port drew them 15× further out and 15× bigger. In the port:
- `tick` (60 Hz) placed them against the tick's camera, but `draw` used the frame's interpolated camera. At 0.5 m any
  difference becomes a big jump on screen, so the rain stuttered whenever the camera moved.
- The 0.2 m deep box wrapped on almost every dolly, so streaks reshuffled each tick. Pans swept whole sheets across the
  screen.
- In a still frame it showed as a regular grid of short ticks (the tiled texture).

## What changed (`play/weather/rain.rs`), our own rain
- 1200 drops (rain) / 2200 (heavy rain), each at a fixed place in a 40 × 28 × 40 m box that **repeats across the
  world**. The box is centred 24 m ahead of the eye (the match camera sits far back with a narrow view).
- Each drop's position is a pure function of the frame's time (tick + overstep) and its fixed place. The camera
  only decides which tile of the box is shown, so camera moves give real parallax, with no snapping or reshuffles.
- Each drop is a thin quad along its fall (9 m/s ±15 %, plus the wind's drift), facing the eye, with a constant
  on-screen width. It uses a generated soft streak texture (tapered ends) and only the colour-only (A < 0x80) material.
  Drops fade in 4–10 m ahead of the eye and out over the box's last 3 m, so nothing pops.
- Under the court's fog, as before. Alpha 0.42 heavy, 0.32 light.
- Ground ripples: the grid now moves once at the start of each four-frame cycle. The original re-randomised it on all
  five ticks of the cycle's first frame, which flickered.
- RNG: the drops draw from nothing (a fixed hash spread). The ripples still use the effects LCG; the sim RNG is
  untouched, and rain timing is the schedule's.
- Test `drops_keep_their_world_place`: a drop keeps its world place when the camera moves, stays in the box, and
  falls at its speed.

Shots (`context/b480/run1/`): `port_w3.png` (before), `new3_w3.png` (after, heavy, court 11), `fly_w3.png` (no
wind), `new_w2_c1.png` (light, court 1).

## Bench (`tools/bench.sh`, HST_GPU=igpu, court 11, 40 s, two runs each)
Weather 0: mean frame 14.2–15.4 ms, CPU 9.0–10.1 ms/frame. Weather 3 (new rain): 15.4–16.0 ms, CPU 9.7–11.0 ms.
About +0.8 ms GPU frame time and +0.5 ms CPU with heavy rain, and that includes heavy rain's 200 clouds and fog.
Fine for 60 fps.

## Not verified / not 1:1
- The rain's look isn't the original's on purpose (user 2026-10-16: our own rain is fine). It isn't compared with a
  PCSX2 shot this time; the old `context/shots/p17i/rain3.png` is the reference.
- Smoothness under camera motion is shown by construction and the unit test, not by video. The stills can't show it;
  no screen recording was made.
- The old rain wasn't benched (its binary would need another 20-minute build), so the cost is measured against no
  rain, not against the old rain.
- Ground ripple grid: it now moves once per cycle instead of on five ticks (a look change, not the original's).
- The game counter's message (end of game, result kind > 0) was read from the decomp and B377's recording. It wasn't
  re-recorded for a rainy match.
