# B196 — recover when a peer stops responding online

A peer going silent mid-match (crash, lost network, closed tab, frozen machine) no longer hangs or silently drops
the others. Code: `crates/hst/src/play/recover.rs` (new), `net.rs` (relay: rejoin, `Msg::Alive/Away/Choose/Restart`),
`play/rollback.rs` (sessions from a base tick, a restart round on every datagram), `play/netplay.rs` (wiring,
`Lockstep::from`, tests).

## How

- **Detect**: every machine sends `Msg::Alive` every 0.5 s; any datagram counts too. The host marks a guest away after
  3 s of silence (30 s before its first word, while it still loads). ggrs' own disconnect is switched off (timeout
  2^20 s), so ggrs never drops a player on its own: without that peer's pads the match stalls at the prediction
  window, the same tick on every machine.
- **Pause + countdown**: the host sends `Msg::Away { who, left, ask }` each time the countdown changes; every
  machine shows "Waiting for <name> to come back... N s" (20 s), then "<name> didn't come back. Confirm: a bot plays
  for them  Back: everyone to the lobby (N s)" (15 s; no answer = lobby). Shorter waits per the user (2026-10-13). Any remaining player's press answers
  (`Msg::Choose`), the host's own or a guest's.
- **Rejoin**: the same lobby code, joined again while someone is away, takes the away player's seats (`net.rs`
  relay; a same-name joiner first), gets `Msg::Start` with `room.rejoin` set (skips the court intro, holds until the
  restart), and the host restarts the match with it in.
- **Restart** (rejoin and bot takeover alike): the host encodes its state at a tick every machine has all pads for
  (rollback: ggrs' confirmed frame; lockstep: the gate's tick) with B154b's codec and sends it with the new room
  (`Msg::Restart`, 48 KB pieces). Every machine, the host too, decodes it, sets `Pads::seats` (a seat now played by
  a bot is the computer's from that tick, as an empty seat at the start is) and starts a new ggrs session (or
  lockstep) whose frame 0 is that tick. Datagrams carry the restart number in their first byte so an old session's
  late ones are dropped. The sim and its determinism are untouched: the same state + same pads on every machine.
- **Back to the lobby** (user 2026-10-13): the host clears the away players' and the computer's seats and everyone's
  ready (`Room::back_to_lobby`), sends `Msg::Lobby(room)`, closes the away peers and its relay returns to the lobby
  loop (`host()`), same code, same connection. Every machine shows "<names> didn't come back. Back to the lobby...",
  hands its `Net` + room to the menu (`netplay::LOBBY`), and the menu opens on the lobby screen with that room;
  ready up and start again as usual. **Host silent**: guests wait 10 s past the 3 s, then end with "The host stopped answering." (no host
  migration).

## Results

- In-process harness (`netplay.rs` `mod matches`, real match, lagged wire, 3 peers = 4 seats: host 1P, h 2P+4P,
  g 3P; or 2 peers):
  - `a_bot_takes_a_silent_peers_seat`: g killed at tick 600; host and h show the waiting countdown, then the
    question; the host presses Confirm; both restart at the same tick (within the window of the kill), seat 3 is the
    CPU, and they hash the same for 600 more ticks.
  - `a_bot_takes_the_only_guests_seats`: 2 peers, the guest killed; the host plays on alone with bots.
  - `a_silent_peer_can_end_the_match`: h answers Back; both end with "g didn't come back. Back to the lobby..." and
    hand over a room with g's seat open, nobody ready, g's name gone.
  - `net.rs short_code_on_a_testnet` (extended): after the start the host sends `Msg::Lobby`, the guest gets it, readies
    again and a second Start reaches it: the host's relay really goes back to its lobby loop.
  - `a_silent_peer_rejoins_and_a_silent_host_ends_it`: g killed, a new app joins with g's seat, all three restart
    on the same tick and hash the same 1300 ticks later; then the host is killed and both guests end with "The host
    stopped answering".
- Native, three processes (`drop.sh`, doubles, host 2 seats + guests a, b; all mashing; relay off, one machine):
  see `1-RECOVER-FINAL.md`. Bug found there: a rejoiner never set `seated` and was dropped by the 20 s
  handshake timer; fixed (Start seats it). The hash log now prints absolute ticks (it restarted at 0 per session).

## Not verified / not 1:1

- Not 1:1 by nature: online play is ours, the original has none.
- **Browser**: no browser peer was killed. The browser uses the same `recover.rs` and the relay path; a browser-LAN
  (WebRTC offer/answer) host can't accept a rejoin (a new offer needs a new answer by hand), so there a silent peer
  can only be replaced by a bot or end the match.
- **Pulling the network** (vs killing the process) was not tried separately: the host sees only silence either way
  (keep-alives stop). A frozen peer that answers again within the wait (no rejoin, the same session) just goes on
  ("is answering again"): written, not tested.
- **Four separate processes** (four guests): run with three processes holding four seats; the relay code doesn't
  depend on the count.
- **Lockstep mode** (`room.lockstep`, a debug mode) restarts through `Lockstep::from` but has no test of its own.
- The end shows the score as text in the end notice, not the match result board.
- A rejoiner keeps the seat's old character pick (the room's), not its own picks.
- A Confirm/Back press used to answer may also reach that tick's pads (harmless: the match is held then).
- The lobby return is checked by the harness (room handed over) and the net test (relay back in its lobby), not by
  driving the lobby screen in a live run.
- `--play` from the command line (no menu) has no lobby to go back to: there "Back" just ends as before.
- The pop-ups are plain text notices, not the game's style: B196a.
