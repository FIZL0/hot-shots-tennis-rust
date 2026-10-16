# B432 — Random court/umpire draws from the mods too

Most of it had already landed in aa106fb2 ("Random court/umpire draw from the mods too"): `Menu::court_pick` /
`umpire_pick` (main_menu.rs) draw Random from the disc courts/umpires plus `court_mods::values` /
`umpire_mods::values` with the menu's own `roll` (std `RandomState` hash, never the sim RNG); the match flags
(`--court-mod`, `--umpire-mod`) and the online wire (`CourtMod`, `UmpireMod`) follow the drawn value.

This task closed the online part ("online keeps to what every peer has"):
- Court: `court_mods::values` already filters online to mods every peer's Hello lists (none in a vanilla lobby), so
  Random draws only shared ones. New test in `online_rooms_carry_the_court_mod` (guest without gag_c03 → Random never
  lands on it).
- Umpire: peers don't announce their umpire mods, so online Random now draws only the disc five (every peer has them).
  An explicit umpire mod pick online is unchanged (peers without it hear the donor). Test in the umpire mods test.

## Not verified / not 1:1
- none (front-end behaviour, ours to design; no original counterpart for mods).
