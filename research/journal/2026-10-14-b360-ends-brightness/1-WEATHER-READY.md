# B360: the screen brightens at a change of ends on court 4

## What it is
It's the match's weather. The original sets a weather for each game (`hst_sim::weather::schedule`) and switches it
when the next game's first point is set up (`play/weather.rs::step`). After an odd number of games, that's the end of
the change of ends. Court 4 often gets cloudy spells. Four schedules from random seeds gave `[0,0,0,0,0,0]`,
`[0,0,1,1,1,0]`, `[0,0,0,0,0,0]` and `[0,0,0,1,1,0]` for games 1–6. When a spell ends on an odd game count, the
cloudy → clear switch lands right as the players appear at their new ends. A cloudy look has a smaller light scale,
fog from the look row and shadows ×0.75. In doubles neither game draws clouds (singles only, P17s7), so all you see is
the whole picture getting brighter.

Ruled out first (temp probes, since reverted):
- The lens flare/whiteout and VU1 sun glare (`gs.wgsl` `glare`). With bots or several local humans, the camera stays
  at its end through a change of ends (`play.rs`, only solo games turn it). On court 4 the camera never comes within
  the glare cone: x stays at about −4 or lower, and glare only starts above 0.
- With clear weather in both games (autoplay doubles on `--stage 4`), frames before and after the change of ends have
  the same mean colours to within about 1 unit (`context/b360/run2/`, `ab.png`).

Reproduced: with the schedule forced to `[1,0,…]` (temp `HST_DBG_W`), regions away from the HUD go from
(112, 91, 84) / (98, 112, 161) on the last change-of-ends frame to (122, 98, 89) / (109, 124, 177) on the first serve
frame. That's about +10 %, in one frame (`context/b360/run3/`, `ab3.png`).

## The original does the same
`research/b360_weather_shots.py 5 context/b360/orig1 1 200 600` loads slot 5 (court 10, bot doubles, clear) and
writes cloudy into every game of its schedule (gm+0xd8). It then takes shots at load and after the change of ends.
- Before the change of ends (clear): left edge (113, 117, 82), mid-court (107, 113, 56).
- After it (cloudy): (101, 106, 75) / (94–96, 100–102, 52–54).
- That's about −10 %, the same size of step as the port's, in the other direction (`context/b360/orig1.png`).

So the brightness step is the original's per-game weather. The port matches it. Holding the brightness steady
would mean not applying the schedule's weather. That changes the look and, in rain, the gameplay (agility ×1.5).

## Needs the human
Keep it as the original (close B360), or change it? Possible change: fade the look between weathers over the
change of ends instead of in one frame. That's presentation only and wouldn't touch the sim or its RNG. Entry in
`plan/HUMAN.md`.

## Not verified / not 1:1
- Whether the original's step is a single frame or a short fade: the screenshot hotkey lands ~200 frames late, and the
  global light block (0x1ea310) didn't change through the switch, so the weather's light colour lives elsewhere (not
  found).
- The original was checked on court 10 (slot 5, the only bot save) going clear → cloudy, not on court 4 going
  cloudy → clear. Court 4's cloudy look row wasn't compared against the original's RAM.
- The user's exact match (4 humans, their seed) wasn't replayed. I'm assuming the reported step is a cloudy → clear
  switch, because nothing else in a clear match changes the brightness at the change of ends.
