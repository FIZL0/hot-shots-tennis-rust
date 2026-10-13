# B195 — lobby browser first, character after joining

## What changed
- **Find** goes straight from Online to the lobby list (no singles/doubles, controller or character pick first) and
  lists every open lobby, singles and doubles together. Each row: host name, Singles/Doubles, court, "N open"
  (`online.rs` Lobbies layout); the info bar repeats it with the seat total.
- Choosing a lobby sets singles/doubles from its listing and caps this machine's players at its open seats
  (`View::fit` → `local_max`), then the controller screen (Assign), then **join on placeholder picks** — that holds the
  seats. The first line-up (`Event::Room`) while `pick_next` opens the character select; everyone ready there sends
  `Cmd::Picks` → `Msg::Picks` into the held seats (`Room::repick`: same count only, clears the peer's ready).
- **Host / Quickplay / Join by Code**: singles/doubles → controllers → lobby at once (no character, no confirm screen
  first), then the same select. A lobby link (`linked`) now starts on Assign; the first press seats and joins.
- `Room.picking` (serde default): peers seated but still choosing; the lobby row shows "Picking..." instead of the
  placeholder character. The host puts itself in it when opening; a guest's Hello puts it in; `repick`/`leave` take it out.
- Lobby buttons: host Start · Characters · Court · Copy Link · Leave; guest Ready · Characters · Leave. Characters
  re-opens the select (not ready meanwhile). Court opens the confirm screen (court, sets, games, umpire); its Start
  sends `Cmd::Venue`, the host updates the room, rebroadcasts and relists (Find shows the new court). Back from
  the confirm screen / select returns to the lobby. Refused/kicked while picking → the lobby screen with the reason.

## Verified
- `online::tests::find_first_then_pick_in_the_lobby`: Find lists a singles and a doubles lobby; joining each takes
  1 player (open-seat cap), lands in Chars on the first Room, ready sends Picks, a later Room doesn't reopen it.
- `online_flow_host_and_code`: Host → lobby → select → Picks; Court → confirm → Venue.
- `net::tests::lan_lobby_by_mdns`: a singles and a doubles host both listed by mDNS.
- `net::tests::short_code_on_a_testnet`: the guest's seat is in `picking`; its Picks and the host's Venue reach the
  guest; the started match has those picks and court.
- Live (two `hst net host [doubles] --relay off` + `hst net find`): both found in 3 s; `hst net join` to the
  doubles one seated with the joiner in `picking`.
- Shots (`context/b195/`): lobbies and lobby at 16:9, 4:3 and 8:3; nothing clipped.

## Not verified / not 1:1
- Two GUI peers end to end (browser + native) not driven (now on PLAN.md *Needs the human*, user 2026-10-13): the menu flow is covered by unit tests and the network
  side by the loopback/testnet/LAN tests and the CLI, not by two windows pressing through it.
- Other peers' live cursors while picking are B201 (the select is local; others see "Picking..." then the result).
- Front-end only, no original to match (PLAN: menus are ours to design). Online gameplay untouched: `Room`'s new
  field is lobby-only and frozen into the Start room as before.

## Follow-ups (user 2026-10-13)
- Online menu order: Quickplay, Host, Find, Join by Code (Quickplay first).
- Merging with B201a (voting, on main): the host keeps the lobby's **Court** button (straight to the confirm screen,
  `Cmd::Venue`, no vote) next to **Vote**: both, not one replacing the other. Six host buttons need a narrower
  `BUTTON_STEP`/`BUTTON_W` (or two rows) to fit 4:3.
- Public/private lobbies don't exist yet (every hosted lobby is announced): B213.
