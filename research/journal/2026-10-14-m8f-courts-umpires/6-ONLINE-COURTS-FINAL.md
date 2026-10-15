# 6 — M8f8 online court mods (FINAL)

An online room now carries the host's court mod, the same way M8f3 carries umpire mods.

## How it works
- `net::Room::court_mod: Option<(id, name)>` (serde default, skipped when None, so older peers' rooms still parse);
  `Room::court` stays the mod's **layout** court, so every peer's match (collision, judging, chair, state hash)
  is on that disc court. The host sends `Cmd::CourtMod` with every `Cmd::Venue` (lobby pill, court select closing,
  Start) and its opening `Room` includes it.
- `netplay::flags` adds `--court-mod-id ID` after `--stage/--court layout+1`, unless the lobby is vanilla.
- `main.rs`: `--court-mod-id` looks the id up under `mods/` beside the disc (`mods_court::find`). Not installed,
  or installed with another layout than the room's `--stage`, → no mod: the disc court plays (logged).
- Menu: the host's court strip lists court mods online too (not in a vanilla lobby). The lobby card shows the
  layout's picture; its name is the host's own pick for the host, for a guest the mod's name when it's
  installed there, else the layout disc court's name (what that guest will actually see).

## Checks
- `online_rooms_carry_the_court_mod` (court_mods.rs): strip online / vanilla, Start sends `Venue(layout)` +
  `CourtMod(id, name)`, a disc pick sends `CourtMod(None)`, guest/host card names.
- `flags_carry_the_court_mod_id` (netplay.rs), `court_mod_id_flags` (main.rs: installed / missing / other layout).
- `live_places_and_a_hosts_bot_reach_everyone` (net.rs, local connection): the mod reaches the guest and is in
  the Start room.
- Shots `context/m8f8/run1/`: `mod.png` (`--stage 6 --court 6 --court-mod-id gag_c01_yamate`: Yamate drawn),
  `missing.png` (unknown id: the plain layout court).

## Not verified / not 1:1
- No original to compare against (online and court mods are ours); checked by tests and the two shots only.
- Not tried in a live two-machine online match (the full netplay runs are `#[ignore]`d and don't touch the
  flags); the path is the same `flags` → `parse` one the tests cover, and the court mod never changes the
  match's state (M8f6's hash test).
- A peer without the mod isn't offered a download (as with umpire mods); it plays the layout court.
- The lobby browser's listing shows the layout court's name, not the mod's (Listing carries only the court index).
