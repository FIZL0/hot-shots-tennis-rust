# B236 — loading states on the online screens

`play/main_menu/online.rs`:
- `View` gains `finding` (Find's lists still due: 2 per search, LAN then DHT), `fail` (the failure panel's title,
  set in `heard(Event::Failed)` from what was under way) and `spin` (clock, advanced in `poll`).
- Find (Lobbies): nothing listed yet → "Searching for lobbies..." panel with an 8-dot spinner; done and empty →
  "No open lobbies found" (Refresh / Host); failed → red "Couldn't search for lobbies"; lobbies listed but the
  DHT still out → small "Searching..." + spinner by the title.
- Lobby before its first line-up: "Creating lobby..." (host), "Finding a lobby..." (Quick), "Joining..." (Find /
  code), net's step line ("Connecting...", "Looking up K7QX...", ...) in the info bar; failure → red "Couldn't
  create / join the lobby" (or "The lobby closed"), the reason in the info bar. "Getting the code..." got a spinner.
- Timeouts were already in `net.rs` (connect 30 s, no lobby 20 s, DHT find 30 s); they now land in the failure panel.
- Main menu's Online line no longer says "(coming later)".
- Shots: `HST_MENU=lobbies-searching|lobbies-more|lobbies-none|lobbies-failed|lobby-hosting|lobby-joining|lobby-failed`
  (via an HST_BIN wrapper), in `context/b236/` incl. a 1600×500 wide one. Test `loading_states_end`.

## Not verified / not 1:1
- Front-end is ours (PLAN), no original to match.
- Not driven through a real network host/join/find in this run: the states come from the sample setups and the
  unit test feeding the same `Event`s `net.rs` sends.
