# B216: adding a bot is its own step; its character picked on the select — FINAL

## What landed
- **Local** (`main_menu.rs`): `Menu.bots` marks the seats 1P added a computer player to. On the controller screen 1P's
  ←→ moves a hand over the seats and △ (or the seat's `+ COM` / `- COM` pill) adds or removes a computer player on a
  seat with no device. A joining device passes over a computer's seat. On the select, 1P drives each added computer
  player's cursor after the humans are ready (character, costume, level, hand; B66 taken costumes skipped as for a
  person). A seat with nobody and no bot is drawn "Open" and filled when the last pick is ready (`fill_open`): the
  first character number nobody has, on its first free costume — the same rule as an online lobby's open seats at
  Start (`Room::bots`). Back from the confirm screen opens it again.
- **Online** (`online.rs`): the host's OK on an open seat adds a computer player (as B201) and now opens the select on
  it (`View.edit`); the host's first device drives its cursor (disc or custom page, so a mod character and costume),
  `align` leaves that seat alone meanwhile, and ready sends `Cmd::Bot(seat, pick)` with the mod's wire (`wire()`, the
  same as a person's pick) and goes back to the lobby; ○ goes back leaving it as it was. OK on a computer's seat
  picks it again; △ (or the row's red ✕) removes it. ←→ / L1 R1 on the row still change it quickly.
- **net.rs**: a computer player's mod looks are the host's own (`want` packed only "host" seats; a guest asking for a
  bot's mod waited forever). The Start room carries the bot's pick, so it is frozen into the handshake like a person's.
- `HST_MENU=lobby-bot` (the host's lobby with a computer player); `assign`/`coms` show 2P/3P bots, 4P open.

## Tests
- `main_menu::an_open_seat_fills_at_start`, `locked_costumes_and_match_order` (add / remove / re-add a bot on 3P, a
  pad joining passes over it, 4P open filled with Jun, match flags unchanged), the custom-page, costume-pack and hand
  tests now add their 2P bot first; `pointer::taps_from_main_menu_to_match` taps `+ COM`.
- `online::shared_select_and_host_bots`: add → select on the bot, align doesn't overwrite it, ←→ / R1 / OK → `Cmd::Bot`
  with that pick, OK on a bot re-opens it, ○ back with no command, △ removes.
- `net::a_hosts_computer_player_on_a_mod_reaches_the_guest` (loopback host + guest): the host's bot on a mod in
  costume 2 reaches the guest, the guest gets its looks pack, Start carries it (peer cpu, donor, costume 2, mod id).
  Without the `want` fix it times out (checked).
- Legend probe test passes with the new Assign (Seat, Add/Remove COM) and lobby (△ Remove) entries.
- Shots `context/b216/` (assign/coms/lobby/lobby-bot at 960×720 and 1680×720): everything laid out, nothing clipped.

## Not verified / not 1:1
- Not 1:1 by design: the front end is ours (PLAN); the PS2 game has no "add a COM" step (its free seats are always
  the computer's, picked by 1P).
- Guests don't see the bot's cursor move while the host picks: the bot's pick reaches them when the host readies it
  (a `Live` place for computer seats would need a peer key for them in `Room.live`).
- Each `Cmd::Bot` still clears everyone's ready (B201's rule for a line-up change), so a guest readies again after
  the host changes a bot.
- Two real machines (native + browser) running the GUI flow with a store mod were not run (one machine, unattended):
  covered by the loopback net test plus the menu tests; the crossplay check is already on *Needs the human*
  (B195, B201).
- The match itself with a mod bot was not launched here; the launch path (`launch` → `MatchMod` by seat) is seat
  agnostic and unchanged.
