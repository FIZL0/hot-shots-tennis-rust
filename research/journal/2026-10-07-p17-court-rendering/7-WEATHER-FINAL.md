# P17i — weather (schedule, look, play effects, rain particles, sound)

## Schedule (`hst_sim::weather::schedule`, odds from `exe.weather_odds` / `wind` / `gusts`)

- Cloudy spells per game, one rain spell per match (heavy = weather 3), cloudy games either side unless no_border.
- Wind: base speed per court; per-set gusts (chance; kind 0/1/2 -> 20/50/80% full 9.0, else 2.3..5).
- Applied per game at the serve (`play/weather.rs::step` sets `Weather.game = games_played`).
- `HST_WEATHER=n` forces a weather.
- The game draws the schedule from its MT19937; the port uses an xorshift seeded from the clock (ponytail).

## Look per weather (`crate::weather::apply`)

- The look row `[sky, bg, far, z0, z1, light]` overrides the fog; in rain the light goes grey (RGB mean) and the fog
  white, scaled by light.
- The grey mean and the scale go through the PS2 FPU (`ps2::div/add/mul`). PINE on court 10, weather 3: light
  0x3f58d8d8 and intensity 0x3ee8f5c0.
- Ground shadow strength x0.75 / 0.5 / 0.25 in weathers 1 / 2 / 3.
- Clear colour is redone.
- `_acc` is hidden in 3/5; `_clo` and the sun's flare are shown only when weather < 2.
- Clouds: re-laid out in 3/5 as (radius 50, y -2..-5, 200 clouds); tint 0.8 (2/4) / 0.55 (3/5).

## Play effects

- Rain agility x1.5 (slower to pick up).
- Wet dive thud.
- Rain blob shadow per player: `PCDATA/PCCG0.XB` `other/shadow.tm2`, 1.2 m quad at the feet midpoint
  (Bip01RFoot / Bip01LFoot), 0.0075 up.
  - Vertex colour (128,128,128,77). The GS alpha 252x77/128 overshoots 1, so the core is fully dark; the port uses
    alpha 1.0.

## Particles (`play/weather/rain.rs`)

Textures `AZUMA/C_EFF/EFFCT.XB0` `hatsuyama/efct/`. Effects LCG x*0x343fd+0x269ec3, restarted from the match seed at
each serve.

- **Rain streaks** (class vtable 0x1d1380, row 0x1bce50; weathers 2/3):
  - 20 quads in a camera-space box of half-extents 0.45x0.3x0.1, 0.5 ahead.
  - v = down*0.0035 + wind(sin a, 0, cos a)*speed*0.0005.
  - Each quad is 0.6 wide along camera X and 0.4 along v. UV is (0..20, 0..0.8) on rain00 with point sampling and U
    repeating. Colour (128,128,128,64).
  - Wraps one axis at a time; respawns on a camera jump > 1.0 and at each serve.
  - The port draws them x15 further out and bigger (the same on screen) because its play camera's near plane is 5 m.
- **Ground rain** (class 0x1a7880, vtable 0x1d1320, row 0x1bcf50; weathers 2/3):
  - 5x12 cells of 3 m, each 2x2 tiles of 1.5 m at y -0.01.
  - Tile k shows frame ((tick/5)+k)&3 of the 2x2 atlas in groundrain (256x256).
  - Grid offset (3r, 3r) re-rolled while (tick/5)&3 == 0. Colour (128,128,128,64).
  - The game mirrors some tiles' UVs; the port doesn't (ponytail).
- **Leaves** (class vtable 0x1d1350, row 0x1bcea0):
  - On when the court has an entry in the per-court table (0x1bcc80, court*0x20 + idx*8: type, strength), integer
    wind speed >= 3 and weather 0/1.
  - Ring of 20 leaves; each lands at (+-9, +-15) after 400 ticks, falling 0.015 per tick and drifting with the wind
    *0.011.
  - Sway amplitude 0.5 across the wind; after landing it slides at 0.6x and fades over 60.
  - Interval: 700*(1-0.5r)*((strength-1)/3+1)*(1-max(0,speed-2)*0.6/7).
  - The first leaf of a point is pre-aged and stays invisible until it lands (game quirk, kept).
- The game's parent update (0x1a3520) is dead code.
- The wind00/01 and rain01 textures are unused by these classes.
- The particles aren't fogged in the port (the game draws them in the fogged court pass): ponytail.

## Sound

Rain ambience is 4 court-bank voices, program 1 key 2, at bearings 90/135/225/270, started when rain starts and
stopped when it ends. The game sweeps each +-45 deg (1 deg every 5 frames); the port keeps them fixed (ponytail).

## Verified

- Screenshots in `context/shots/p17i/` (original rain3.png vs port_w3 / port_s_w3) show grey fog, white sky, faint
  shadows, blobs, ripples and streaks.
- `tools/check.sh` passes.
- `gs::tests::court_clear_colour` checks the PINE light values to within an ulp.

## Not ported, for later

- The character lighting mode in rain.
- The wind-rate model deformer (tree sway, rate speed*0.0889 + 0.2).
- Walkers: in the original's rain capture, no walking spectators were seen; why is not checked.
- The camera-cut flag respawn (only the jump test is ported).

## Note for P17j

Footprints are off in rain on some courts. The object at 0x3bb490 (class 0x1d1fc0) is enabled from the court table
0x415084 (clear) / 0x415085 (rain), stride 0x90.
