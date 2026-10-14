# B279 — after an online match, back to the same lobby

User 2026-10-13: leaving the result screen of an online match goes back to the lobby (seats, picks, court kept), not
the main menu, so everyone can play again.

## How

Built on B196's way back (`netplay::LOBBY`, `Msg::Lobby`, the host relay's return to its lobby loop).

- `netplay::again` (Last, while `Netplay`): the result screen's Continue writes `AppExit`; with the result in and the
  match not ended otherwise, this machine's `Net` + a room go to `LOBBY` and the mode's way back (`online.rs` `launch`)
  opens the lobby screen with the menu as it was at the start (picks) and that room.
  - **Host**: the room with `Room::again` (nobody ready, `ver`+1; seats incl. computer players, picks, court kept;
    unlike `back_to_lobby`, which opens CPU/away seats) and sends `Msg::Lobby`: its relay broadcasts it and goes back
    to the lobby loop (same code, same connections).
  - **Guest after the host**: the host's `Msg::Lobby` arrives while it's still on the results: `take` keeps it in
    `recover.lobby` (no end notice, no auto-close as in B196); leaving takes that room.
  - **Guest ahead of the host**: its own room (`again`), marked waiting: the lobby shows only Leave and "Waiting for the
    host to leave the results..." (`started` = true) until the host's `Msg::Lobby` comes (`Menu::heard`), then the
    usual Ready/Characters/Leave.
- `recover::tick`: once the result is in, no silence detection (a peer who left the results stops sending keep-alives;
  before, the host would have shown "Waiting for g…" and the bot/lobby question, a guest "The host stopped answering").
- `hst net … --play` (no menu): its way back drops the handed-over connection.

## Results

- `netplay.rs the_result_screen_leads_back_to_the_lobby` (2-peer rollback harness, real match): result set at round
  120 and the host gone quiet; 12 s later the guest has no notice and no end (fails without the `recover` change).
  Leaving: guest ahead → its room, waiting, nobody ready, same seats and court; host → `Msg::Lobby` sent (fake `Net`,
  `Net::wired`), same seats, nobody ready; guest after the host's `Lobby` → that room, not waiting.
- `online.rs waits_for_the_hosts_lobby`: waiting guest has only Leave; the host's `Lobby` opens Ready/Characters/Leave.

## Not verified / not 1:1

- Not 1:1 by nature: online play is ours.
- No live run of two processes through a whole match to the result screen and back (a full match is long and both
  menus would need driving); the relay's return to its lobby loop on `Msg::Lobby` is B196's, covered by
  `short_code_on_a_testnet` (ignored, DHT) — not re-run here.
- A guest waiting for its host whose host then disconnects keeps the stale room with Leave only (the failure reason
  shows in the status line) rather than the "lobby closed" panel (`started` blocks `Failed`'s reset).
- A guest's Ready pressed in the lobby before the host is back can't happen (only Leave); chat during the wait isn't
  relayed by the host until it is back.
- Browser: same code path, not run.
