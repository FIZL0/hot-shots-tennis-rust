# B404: Stage 1's "darker with Gaussian shadows" open court is the weather roll, not a shadow

## Finding
G4 measured Stage 1's open court at 112 mean luma with Bevy's Gaussian shadow filter and 121 with shadows off,
Temporal, or one tap, and read it as self-shadowing. It isn't. A `--stage 1 --play` run without menus takes a
clock-picked match seed (`main.rs`, `Seed::after(subsec_micros)`), so each run can roll a different weather. Clear (0)
and cloudy (1) light the ground differently. Every shot in the bisect was a fresh roll.

Checks (`tools/shot.sh`, 1280×720, court box = the middle of the screen; `context/b404/`):
- Unpinned (run1): high 124.7, off 116.2, high with `sclwall-02` not casting 115.9. A debug shader writing
  (sun visibility, `gs.shadow`) showed visibility 1.0 over the whole open court in every run. What changed was the
  ground's `gs.shadow` (weather-scaled strength), so the weather differed between runs.
- Pinned `HST_WEATHER_SEED=1234` (run2): open court 124.7 with shadows High, Low, Off, Soft Shadows, and High with
  the wall not casting. Seed 9 gives the same.
- Pinned seed, `HST_WEATHER=0` vs `1`: 124.7 vs 115.9 (shadows high), 125.1 vs 116.0 (off). That's the 7.5 %.
- Stage 1's casters (34: walls, trees, stands, nets, club house) have no flat ground-level faces. The suspect
  `sclwall-02` (bounding box spanning the court) is all vertical walls (7.7 m tall). Its real shadow is the dark band
  on the left run-off, which is meant to be there.

So no caster gets dropped or biased; `shadow.rs` / `graphics.rs` are unchanged.

## Change
`tools/shot.sh` passes the caller's `HST_*` variables on (hyprctl's exec dropped them, so A/B shots needed a wrapper
script for `HST_WEATHER_SEED`). Its header says to pin the seed for A/B comparisons. Checked: `HST_WEATHER=1`
through it gives the cloudy court (114.9), and no variable gives the clear one (123.7).

## Not verified / not 1:1
- No PS2 screenshot of Stage 1 was taken. The question was whether the remaster's shadow map darkens the open court,
  and with the weather pinned it doesn't. How bright the court is in each weather is the weather look's job (ported
  earlier), not this task's.
- Bevy's Hardware filter wasn't re-run separately. With the seed pinned, Gaussian (default), Soft (G4) and Off
  already agree.
