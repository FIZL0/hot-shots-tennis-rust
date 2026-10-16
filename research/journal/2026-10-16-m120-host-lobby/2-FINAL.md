# M120 iteration 2: character cards, Join straight in, relay chip (B519)

## What changed
- **Online record removed** (`hst-base/src/net.rs`, `online.rs`, `netplay.rs`): no `Record`, `Room::records`,
  `Room::scored`/`team`, Hello record, `kept_record`; online.txt no longer has `won`/`lost`.
- **Remote cards show the character** (`roster::compact_card`): the band (player's name over "Online - N ms" or
  "Computer"), the figure full width with M95's platform/input pill in its corner, the character's name, then
  "Right/Left - Costume N", the roster card's own style bar and `stats()` list, and a foot ("Disc character" / "Mod" /
  "COM - rank" left, a Ready / Picking... / Not ready pill right). `stat_bars` is gone.
- **Local cards online** get the M95 pill too (this machine's `device::CURRENT`) in the preview's corner.
- **Join (code, link, Find, Quickplay) lands on the select like Host** (`online_open`). A guest that readies before
  the lobby's line-up came stays on the select (`View::early`); `poll` sends its picks and goes to the Lobby once
  the room arrives.
- **B519 relay status**: `net::Event::Joinable` (hosting: short code on the DHT, or the ticket fallback sent; a
  browser's LAN lobby once its offer is out). `online::reach` → Connecting / Up / Offline (relay off or failed).
  - The select's corner pill: a lock and greyed code until Up, then the chip ("Relay: connecting..." amber,
    "Online, joinable" green, "Offline" red; a guest's "Joining...").
  - A notice band under the select's top bar (wrapped, pushes the columns down) while not Up; open seats say
    "Not open yet: friends can join once the relay is up".
  - The Lobby screen's code pill: lock + grey code, the chip after it, the title line wrapped over the stage.
  - `shared()` is None while a host isn't joinable: Copy link / Code don't copy a dead code.
- Status lines: the description bar (`fit_lines`) already wraps and shrinks, so failures stay there.
- Samples: `HST_MENU=chars-relay`, `lobby-relay` (relay still connecting); the others are joinable.
- Test: `the_code_waits_for_the_relay`; Join tests now expect Chars.

Tests: `tools/check.sh -p hst online` (35), `roster` (6), `main_menu` (184), `-p hst-base net` (37),
`HST_NETPLAY=1 -p hst-play netplay` (30): all pass.

Shots: `context/m120/run4/` chars-relay, chars-online, lobby-relay at 16:9 (`-169`), 4:3 (`-43`), portrait (`-port`).

Not verified / not 1:1: see README.
