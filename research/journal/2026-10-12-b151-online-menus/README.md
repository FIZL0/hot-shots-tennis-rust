# B151 — online menu overhaul (FINAL)

## Hosting was slow: timings
Measured with `hst net host` (it now prints seconds since start before each event):
- endpoint bind + home relay online: ~3.1 s
- short code: DHT lookup ~0.6 s, DHT put ~1 s, the **pkarr relay HTTP put 5–18 s** (the culprit). Before, the
  lobby screen only came up after the whole roll: 29–46 s.
- QR: instant.

Fix: `host()` says the Room at once (the menu also shows its own room before the network answers), then the ticket
(`Code`, QR), and rolls the short code in the background (`Short` ~4.9 s). `publish()` returns after the DHT put and
sends the relay put in the background. It awaits the relay put only when the DHT put failed.

## Start with bots, names, ready, kick
- `Seat.peer == net::CPU` marks a computer seat. When the host starts, `Room::bots()` fills the open seats with the
  first unused disc characters. `netplay`/`rollback` count only human seats as remote, so CPU seats run locally on
  every peer, the same on each. Test: `doubles_with_computer_seats_stays_in_sync` (3600 ticks, identical logs).
- Start: the host sends `Cmd::Start`, which is refused until `Room::all_ready()`. Guests send `Msg::Prepared`.
  `Room.ready` and `Room.names` travel in the Room.
- `Msg::Hello` carries the guest's `name` (from `online.txt`'s `name`, now "the player's name", default "Player").
  The host stores up to 16 chars.
- `Cmd::Kick(peer)`: the guest is refused with "the host took you out of the lobby" and its seat is freed.
- Menu: Online → Name types the name on Join by Code's key grid (A–Z, 0–9, space, `-` `.` `!`; Delete/Clear/Save/Back).
  On the lobby screen, the host gets the guest seats (OK = take out) plus Start (+n COM) / Copy Link / Leave. A guest
  gets Ready/Not Ready and Leave. When the lobby closes on a guest (refused, taken out, host gone), only Leave is left.
- Seat rows: four columns (nP, character, name, Host/Ready/Waiting/COM), each centred vertically on the pill's middle
  and spread across a 340 px pill.

## Checks
- `hst net host doubles --bots` + `hst net join <short>` on two local instances: started with CPU seats on chars 1, 2.
- Tests: `short_code_on_a_testnet` (doubles, guest name, no start before ready, CPU seats [0, 2]),
  `mod_looks_peer_to_peer_and_the_ready_gate`, `lobby_start_ready_kick_and_name`, the menu flow tests.
- Shots: `context/b151/` (online, code, name, lobbies, lobby, lobby-guest, lobby-ask, lobby-wide, lobby-tall), taken
  with `HST_MENU=<screen>` through an `HST_BIN` wrapper.

## Not verified / not 1:1
- Two **menu** instances (window, pads) were not driven end to end. The same flow ran on two CLI instances and in the
  unit tests, and the menu only sends those commands. Reason: an unattended run can't drive two windows' inputs.
- Cross-network play is still untested (only one machine). See Needs the human, M2f.
- In the browser the short code still waits on the relay put (5–18 s). Reason: browsers have no DHT, so the relay is
  their only route. The roll runs in the background there too, so the lobby and QR still show at once. Not timed in a
  browser.
- The menu design is our own (PLAN: the front end needn't match the original), so nothing here is 1:1.
- A tall window squeezes the lobby sideways like every other menu screen, which is the menu's shared scaling and not
  specific to this screen.
