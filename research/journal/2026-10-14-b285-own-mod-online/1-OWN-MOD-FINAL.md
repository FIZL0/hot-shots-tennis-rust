# B285 — a player's own (or downloaded) mod character shows as itself online

## What was wrong
1. **Browser: the owner's mod vanished from the pick.** `mods_online::pack` walked the mod folder with `std::fs`,
   but the browser build holds mods in memory (`modfs`). `Wire::of` failed (warned), so the pick went out as the
   bare donor: nobody, the owner included, got the mod in the lobby's seat or in the match. `find` (own roster
   pack hash) and `unpack` (writing a received pack) had the same `std::fs` problem, so a browser guest couldn't
   keep a download either.
2. **Everywhere: a downloaded mod showed as its donor in the lobby and the select.** Those look a seat's mod up
   in `Menu::mods` (the installed roster) by id; received looks only went into `net.looks`, used at match launch.

## Fix
- `pack` / `find` / `unpack` go through `modfs` (`read_dir`, `is_file`, `read`, `write`, `remove`): same bytes
  on disk or in memory (test: an in-memory copy packs identically). In the browser a received mod stays in memory
  for the session.
- `Menu::borrow` (online.rs, run from `poll` before `align`): the lobby's mods this machine has looks for but
  hasn't installed are put after the roster in `Menu::mods` (`for_match` copies, id = the wire's, select text from
  their base row), dropped with the lobby. `Menu::own_mods` (first mod with an online skeleton) bounds the roster
  grid, Random's draw, the filters and the legend, so borrowed mods are shown, never pickable.
- Test `own_and_downloaded_mods_show_as_themselves` (test_pc00): owner's pick carries the mod and finds its own
  looks; a guest with a downloaded pack draws the seat as the mod (lobby face/name via `mods`, select via `align`),
  off its roster, gone when the lobby ends; with no looks the donor stands in.

## Not verified / not 1:1
- Not run live across two peers (native↔native or native↔browser): no second machine/browser in an unattended run;
  covered by unit tests on the same code paths. The phone/browser crossplay item under *Needs the human* (B195/B201)
  exercises it.
- A borrowed mod's grade bars on the select use its base TParam row, not its stat overrides (those need the disc's
  TParam through `mods::tparam`); menu-only, not gameplay.
- Online-only feature (remaster addition): nothing in the original to compare against.
