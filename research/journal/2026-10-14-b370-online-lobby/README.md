# B370 — online lobby as seat cards (FINAL)

Spec: plan/B368.md § Online lobby. The menu is ours to design (PLAN.md), so no PCSX2 was needed.

## What landed
- **Layout** (`crates/hst/src/play/main_menu/online/lobby.rs`, new): a top bar to the right of the title tab with
  the code pill (small QR, "CODE"/"PRIVATE CODE"/"JOIN LINK", the code in gold) and the host's Start ("n of N ready,
  +k COM"; Cancel (n) during the countdown) or a guest's Ready/Not Ready over "Waiting for host". Under it the
  settings row across the full width; the Mode pill is now blue for singles and green for doubles. Then one card per
  seat: this machine's seats share `2fr`, every other seat gets `1fr` (`columns`). A narrow or portrait window
  (narrower than 480 units, or taller than wide) uses two rows of two. Along the bottom: Characters, Copy link,
  Public, Leave lobby, and the help box in the space left.
- **Cards** in the seat colours (B368 table, halved for the 0..128 menu scale): "NP" tag, character name (bigger for
  this machine's), "You - Keyboard/Controller" / the player's name (`name_of`) / "COM - <level>", pills for costume
  (with a shirt icon on wide pills) and hand, the 3D preview (`Tex::Preview(i)`: `previews.rs` now runs on
  `Screen::Lobby` too), the play style in gold over the pink stats card (grade letters in grade colours), a status box
  (Ready ✓ / Picking... / Not ready, plus a context line: You / Your partner / Opponent / Computer player, "Choosing on
  their screen", "<mod> mod"), the yellow stand-in note ("You don't have Golf: playing as its stand-in look",
  `remote_mod_name`), the ping on the left and the host's action ("Host: OK removes" / "Host: Triangle removes") on the
  right. A computer's or open seat's card gets a dashed rim. An open seat shows "COM", "Random", the ? tile and "Open
  seat: a computer player until someone joins with the code". The host's ✕ removes a computer player from the card's
  corner. Random hides the costume, hand, style and grades (B217).
- **Live follow**: `Live` and `Pick` now carry `swap` (the select's hand toggle) and `Live` carries `random`.
  `align()` copies the remote seat's swap onto `players[i]`, so the card's hand pill and the preview's flip follow.
  This closed a gap: online matches used to ignore the hand toggle. `netplay::flags` now passes `--swap-hands` in
  match-slot order.
- **Ping**: the host measures each guest's round trip (ms, rounded up to 5) on its existing ping tick and puts it in
  `Room::ping`, which is broadcast only when a value changes and only before Start. A guest sees its own ping on the
  host's card.
- **QR big**: the code pill is a new lobby item ("Code"). OK or a tap opens the QR big. Any button or a tap closes it,
  and Back closes it without leaving the lobby. The mod-download prompt is now a centred popup. 3D previews are hidden
  under both overlays because they draw over the menu.
- Removed the old seat-row lobby helpers (`seat_rect`, `team_boxes`, `help_line`, `SeatRow`/`seat_row`, …) from
  `main_menu.rs`, plus the B282 test that only exercised `seat_row`. The lobby cards draw names with one `left` call
  for disc characters and mods alike.
- Tests: `lobby::tests` (seat states / ready count, `2fr` share), `hand_and_random_travel_live`,
  `arrows_cross_the_lobby_cards` (replaces the old two-column test), plus updated `lobby_taps_hit_items`,
  `cursor_walks_the_grids`, the legend test (Code → "Show QR"), the pointer hover test and `flags_put_each_seat_in_match_order`.
  `tools/check.sh -p hst` passes (352). The ignored `full_doubles_match_*` netplay runs and
  `a_silent_peer_rejoins_and_a_silent_host_ends_it` pass. `cargo check --target wasm32-unknown-unknown -p hst` is clean.
- Shots (`context/B370/run2/`): `lobby-16x9.png`, `lobby-4x3b.png`, `guest-4x3.png`, `bot-portrait2.png` (600×1000),
  `ask-16x9.png`. `HST_MENU=lobby*` samples now seat this machine (host keyboard on 1P, guest on 2P), set pings, and
  put Sunny Day on a mod this machine lacks.

## Split off
- **B372**: this machine's card becomes the full B368/B369 column (grid, tabs, footer `✕ ready · □ favourite`) so
  local players pick on the lobby itself. It also adds the "Browsing <tab> on their screen" context line. B368 and
  B369 were both still open and being written in parallel, so their column renderer and tabs didn't exist to reuse.

## Not verified / not 1:1
- **No live native host + browser guest run.** The live follow is covered only by unit tests (`Live`/`Pick` swap and
  random through `lives`/`wire`/`align`) and by the in-process netplay runs. Driving two real clients needs a browser
  session, which wasn't set up in this unattended run.
- **Local players get a wide card, not the full B368/B369 column**: those tasks hadn't landed yet → B372.
- **"Browsing Hot Shots on their screen"**: the card says "Choosing on their screen". No tab is sent yet (B369 adds
  tabs) → B372.
- **Separators**: the font has no `·` glyph (it rendered as a wide gap), so card lines use " - " and Start uses ", ".
- **Host action wording**: the mockup's "Host: ○ remove" uses a glyph. The card says "Host: OK removes" / "Host:
  Triangle removes" in text, because the legend shows device-specific glyphs and the card doesn't know the device.
- **Open seat**: shows "COM" and "Random" without a "Lv N" (an open seat has no character until Start, so no level).
- **Settings row** kept as the existing pills (court, umpire, sets, games, mods, mode) instead of the mockup's single
  text line, so the host keeps ←→ editing in place.
- **Preview size**: a narrow card with the stand-in note leaves a short preview well; the 3D figure is small there.
  Rendering only has to look right; left as is.
