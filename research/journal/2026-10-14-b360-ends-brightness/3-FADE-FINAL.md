# B360: fade between the two weathers' looks over the change of ends

User 2026-10-15: fade the look over the change of sides. It's presentation only, so sim and weather timing stay
the same. The original changes the look in one frame (B377), and the port still does too when the weather changes
without a change of ends.

## What changed
- `play/weather.rs::fade` (Update, before `weather::apply`) runs during `Phase::ChangeEnds(n)`. It writes
  `weather::Fade(Some((next game's weather, w)))`, where w is the smoothstep of (80 − n) / 80: 0 on the phase's
  first tick and 1 on its last. `games_played` already counts the finished game during the phase, so the next
  game's weather is `schedule[games_played]`. After the phase, the fade holds at 1 until `Weather.game` catches up
  on the point's set-up, then clears.
- `weather::apply` works out the look of both weathers (fogs, court light, ground-shadow darkening, clear colour)
  and lerps between them. `CourtLook.mix` records the mix that was applied. It relights only when the weather or
  the mix changes. Background `_acc`/`_clo` visibility and glare still switch at the real change.
- `shade.rs::light` (players, ball, NPCs) lerps its light and fog the same way. `mix` is part of each rig's key.
- `court_mod::keep` re-applies a court mod's own light/fog whenever the mix changes, as well as the weather.
- `Weather` (the sim/wire resource) isn't touched. Rain agility, rain drops, clouds and the snapshot all keep the
  original's timing.

## Check
A temporary probe (removed) forced the schedule to cloudy, then clear from game 1. It ran autoplay doubles on
`--stage 4` twice, once with the fade and once with it zeroed, and took screenshots through the change of ends.
Mean of the centre region (`context/b360/{fade,step}/`, side by side in `context/b360/ab_fade.png`):
- step: ce00 (103, 109, 145) → after01 (112, 118, 155), a +9 jump on the first serve frame.
- fade: ce01, ce00, after01, after02 and after10 all (117, 123, 161). The change is spread over the glide and
  doesn't jump at the switch.
- The log showed the clear colour going from (61, 114, 208) at ce80 to (77, 149, 255) at ce00, eased.
`tools/check.sh`: 690 passed.

## Not verified / not 1:1
- Not 1:1 on purpose: the original steps in one frame (B377). The fade is the user's requested deviation, and it's
  presentation only.
- Weather changes on even game counts (no change of ends) still step in one frame, as in the original. The user
  asked for the fade over the change of sides only.
- Background `_acc`/`_clo` models, sky clouds (singles), rain drops and glare still switch at the real weather
  change. They're on/off things, not values that can be blended. Only court 4 cloudy → clear was looked at, and
  none of those change there in doubles.
- The user's own 4-human match on court 4 wasn't replayed. The bot doubles run with a forced schedule shows the
  same switch.
