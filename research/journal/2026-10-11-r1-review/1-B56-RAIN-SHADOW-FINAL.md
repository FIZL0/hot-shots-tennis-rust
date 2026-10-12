# B56 — shadows in rain

## The strength (S) per weather
- Setter (court object, called from the light set-up that runs on every weather change, after the light block is
  copied): S = ⌊row·255⌋ with row = the court's shadow strength for the time of day (`envir_cNN.dat` 0x410 + k·0x10);
  **if the row is 0, S = ⌊intensity·128⌋** (the court light's intensity, already weather-scaled, the same value
  `gs::court_light` uses); then ×0.75 / ×0.5 / ×0.25 in weathers 1 / 2 / 3 (4 and 5 unscaled); the shadow texel is
  (S·255)>>8 (table entry 255 of the shadow CLUT object).
- The weaker agent's flat 0.75 was wrong (already reverted). The row-0 fallback was missing: court 7's first time of
  day has row 0.0, so the port drew no shadows there. Fixed: `shadow::strength(row, intensity)`, `gs::court_intensity`,
  applied in `weather::apply`.
- PINE (slot 5, court 10, schedule poked to 3 so the game switches weather itself at the next point): texel 0x2c in
  clear, 0x0a in weather 3; with the row poked to 0, 0x0d (intensity 0x3ee8f5c0 → 58 → ×0.25 = 14 → 13). Both in
  `shadow::tests::sun_matches_game_ram`.

## The look
- `context/shots/r1/orig_w3_*.png` (original, court 10 forced to weather 3 via the schedule bytes gm+0xd8, so the
  game's own weather change runs) vs `context/shots/r1/port_w3.png` (`HST_WEATHER=3 HST_AUTOPLAY=1 --stage 10 --play`):
  both show the dark round blob between the feet plus a faint projected shadow; umbrellas' shadows faint in both.
- Poking only the weather byte (+0x135) doesn't re-run the setter (it's done by the weather-change setter), so
  rain look captures must poke the schedule, not the byte.

## Not verified / not 1:1
- Time of day k=0 only (the port's ponytail, P17e); the fallback is checked at k=0 on court 10 with the row poked.
- Blob alpha: the port clamps the GS's 252·77/128 (> 1) to 1, as P17i noted; compared by eye only.
