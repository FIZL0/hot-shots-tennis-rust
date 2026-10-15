# B377: does the original fade between weathers at a change of ends?

No. It changes in one frame, the same as the port. Nothing to port.

## Code
- The weather change runs when a point is set up. It's a one-shot setter on the court object (+0x135 weather byte,
  look fields +0x50..+0x64). It copies that weather's look row straight into the fields: sky/bg/far fog F as
  (1 − v)·255, the z range and the light scale. Then it calls the court light set-up and the fog set-up once each,
  and remakes the clouds if the weather changed.
- There's no timer, counter or target value. Both set-ups mix each value as `own · 1.0 + other · 0.0`, a blend
  whose weight was compiled to a constant, so no fade is left in the code.

## RAM, frame by frame
`research/b377_weather_frames.py 5 context/b377/w1b.tsv 1 300` loads slot 5 (court 10, bot doubles, clear). It sets
every game of the schedule (gm+0xd8) to cloudy and logs every frame up to 300 frames after the next change of ends
(6967 frames, no skipped vsyncs).
- The court's look fields go from clear `-1 -1 -1 -1 -1 1` to cloudy `178.5 153 102 10 400 0.7` between vsync 8655
  and 8656. That's the frame the phase goes 4 → 2 (a new point is set up). They don't change again.
- The setter runs again at each point set-up after that, including the one that ends the change of ends (phase
  1 → 2 at vsync 14235). With the same weather the values stay the same. In a real schedule, that point set-up is
  where a game's new weather lands. It's the same one-frame write.
- The port switches at the same moment (`play/weather.rs::step`, on the first `Phase::Serve` of the new game). In
  B360 it went from the last change-of-ends frame to the first serve frame in one frame (`1-WEATHER-READY.md`).

So the step B360 reports is how the original behaves too. The open question in `plan/HUMAN.md` (keep / fade /
suppress) is still the human's call. "Keep" is what the original does.

## Not verified / not 1:1
- On screen frame by frame: PCSX2's screenshot hotkey lands about 200 frames late and doesn't work while paused. So
  the step is shown in RAM (the look fields the renderer reads) and in the code, not in pictures. B360's
  before/after shots already show the size of the step.
- Court 4 cloudy → clear wasn't run: the only bot-only save is court 10 (slot 5), so I checked clear → cloudy.
  Both directions go through the same setter, which has no per-direction or per-court path.
- The scene light block (0x1e7d10 / 0x1e7d90) I logged holds direction rows that change every frame, not the
  weather's colour. I didn't find where the light colour lives in RAM. I relied on the setter's own inputs
  (+0x50..+0x64) instead.
