# B522 (+ B521, B520): small online lobby fixes

## B522: a guest's □ / X swaps team
- New `Msg::Team(u8)` / `Cmd::Team(usize)` (net.rs): a guest asks the host to move its player in seat `i` to the other
  team. `Room::team_target` picks the other team's first open seat, else its first computer seat (the computer comes
  over in trade); a person's seat is never taken. Teams: seats `0..n/2` and the rest (singles: 0 vs 1, so a guest
  facing the host has nowhere to go → buzz).
- Menu: lobby (`Screen::Lobby`, guest's □ from anywhere: the chosen own card, else the first) and the online roster
  (`online_team_step`, the device's own seat). The guest moves its player locally at once (`switch_team`) so its
  players keep the order the host's room will have (`align_to` pairs this machine's players with its seats in order;
  matters for a guest with two players).
- Legend: "Swap team" (lobby and roster); the roster's side pill shows "Swap teams" with the □ as locally.
- The host's □ is unchanged (Singles / Doubles); a guest's never sends `Doubles`.
- Tests: `net::tests::a_guest_swaps_team`, `online::tests::a_guests_square_swaps_team`.

## B521: description bar cut off
- Couldn't reproduce on native Linux at 1280×720, 1100×620, 1500×844, 1700×956, 900×506, 1600×500 or 420×860
  (shots in `context/b522/run1`, `run2`): the bar wrapped every time. `ticker_text` assumed Fira Mono's metrics
  (0.6 em advance, 1.2 em line); a platform laying it out wider (web / phone font rendering) would run past the clip.
- Fix: `Grow` measures the text node's laid-out size (`ComputedNode`) against what those metrics predicted and
  raises a width / height factor the next fit uses, so the text wraps or shrinks inside the bar whatever the
  rendering. Native: width unchanged; the tall window's height factor came in a bit over 1.2 em, so it now wraps
  one word later at a slightly smaller size (still inside the bar).
- Test: `description_grows_to_the_laid_out_size`.

## B520: mods off swaps the host's own modded character too
- `Room::set_vanilla` already put every seat's donor in, but each machine's own players stayed on the mod locally
  (card, live place, and the next Picks was refused). `align_to` now calls `unmod` for this machine's seats when the
  room is vanilla: a custom character / costume-pack costume goes to its donor (the seat's costume), keeping the hand
  toggle, Random card and ready. Not while a player is still choosing on the select (the cursor would jump);
  a mod readied there is refused by the host, then put on its donor and resent (`vanilla_repick`, from `poll`).
- `set_vanilla` keeps `random` and `swap` on the donor pick (so nothing needs resending in the normal case).
- Random: a human's draw (`Menu::draw`) takes only disc characters and costumes `< COSTUMES` in a vanilla lobby
  (computers' `bot_draw` already did; Random court via `court_mods::values` and online umpire were already disc-only).
  The host's mods pill on a Random court re-sends the venue so the court is redrawn from the disc's.
- Test: `mods_off_puts_own_mods_on_their_donors`; `vanilla_lobby_has_no_mods` extended.

## Not verified / not 1:1
- B521: the cut-off itself was never reproduced (native always fit), so the fix is verified only by the unit test and
  by native shots staying inside the bar; not checked on a phone or the web build (none available to this run).
- B522 / B520: checked by unit tests on the menu and the room only, not with two real peers over the network
  (no second machine in an unattended run). A guest with two players whose swap the host refuses in a race (room
  changed in between) could have its two players' cards exchanged until its next pick; noted, not handled.
- Front-end only: no original-game behaviour to compare against (the online lobby is ours).
