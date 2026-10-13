# B201: shared lobby with a live character select — FINAL

## What landed
- `net.rs`: `Room.live` (peer → its players' `Live { char, costume, ready, mod }`, in held-seat order; serde-skipped
  when empty, so old rooms parse), `Room::live_of(seat)`. `Msg::Live` / `Cmd::Live`: each machine sends its places when
  they change; the host stores and rebroadcasts the Room (no `ver` bump, no ready reset, no relisting). `leave()`
  drops the peer's entry. `Cmd::Bot(seat, Option<Pick>)` (host: computer player in an open/computer seat, or open it
  again; a person's seat refused) and `Cmd::Doubles(bool)` (taken seats compacted first; refused if they don't fit).
  Both bump `ver`, clear ready, relist. `Event::Me(peer)` tells a guest its seat name.
- The Start room is the lobby's Room, so bots, court, umpire, mode and picks are frozen into the handshake as before;
  `Live` is display only (the match takes the seats' picks). Replay tests untouched.
- `online.rs`: the select's panels are the room's seats (`align()` each frame in `poll`, after `step`): this machine's
  players move into the seats they hold; the others show their live place (computer: its pick, ready; open: dark
  "Open"). So the existing previews, cursors and B66 `free()` checks work for remote players: a costume someone
  else is ready on is passed over. Remote humans show their name under the card.
- Host lobby: open/computer seats are selectable rows. OK on open adds a computer player on the first free grid
  character (B66); ←→ changes its character, L1/R1 its costume (= its level, shown as the rank name in the row); OK
  clears it. The title is the singles/doubles switch (refused with a status while >2 seats are taken).
  A picking guest's row shows the character they're on now.
- Joining mid-pick: the Room carries `live`, so a joiner sees everyone's current places in its first Room.
- Shots (`context/b201/`): `HST_MENU=lobby` and new `HST_MENU=chars-online` at 960×720, 1280×720, 1680×720 — every
  element pinned, no overlap.

## Tests
- `net::host_bots_mode_and_live_places` (Room rules), `net::live_places_and_a_hosts_bot_reach_everyone` (host + guest
  over a local connection: live place both ways, bot + court to the guest, the guest's seat not clearable, Start
  carries the lobby's picks and the computer player).
- `online::shared_select_and_host_bots` (align, remote ready costume blocked, lives out, bot add/change/clear,
  mode switch). Lobby item-index tests updated for the new rows.
- `a_bot_takes_a_silent_peers_seat` and `rollback_desync_is_saved_and_play_goes_on` each failed once in a full run while
  other slots were building (timing under load) and passed alone; netplay is not touched by this task.

## Not verified / not 1:1
- Not 1:1 by design: the front end is ours (PLAN). No original to compare (the PS2 game has no online lobby).
- Two peers readying the same costume at the same instant: both can be ready on it (the host doesn't arbitrate;
  each only sees the other's ready a round trip later). Picks still reach the match as each chose.
- A remote player's mod that this machine doesn't have shows as its donor at costume 0 on the select.
- The hand swap (Select / C) isn't sent live; a remote card shows the character's default hand.
- Native + browser GUI crossplay of the live select not run here (one machine, unattended): already on PLAN *Needs
  the human* (B195, B201).
- B214 (button grid) / B226 (no Vote) will need to keep the new `Item::Mode` and seat rows; the vote code is untouched.
