# B215 — lobby ready-up countdown

The host counts: `Room::count` (seconds left, 0 = none) goes out in a `Room` each second, so every peer's lobby
shows it ("Cancel (N)" on the host's Start / the guest's Ready, and the info line "The match starts in N...").

- Every person's seat picked (`picking` empty) and ready, with at least one guest in → the countdown starts by itself
  (`Room::all_set`). After a cancel it doesn't restart until someone readies (or the host picks) again (`auto`).
- Host's Start → the same countdown, ready or not (no more "Waiting for X").
- Cancel: the host's Start button reads Cancel (`Cmd::Cancel`); a guest's Ready reads Cancel (`Msg::Cancel`); a guest
  un-readying (`Prepared(false)`, also Characters) cancels; a guest leaving / taken out cancels.
- At 0 the old start path runs (mods-settled gate, round-trip probe, seed, `Msg::Start(room)` with `count` 0), so the
  frozen line-up and handshake are as before.
- Length: `net::COUNTDOWN_MS` (4000; the tests set 1000).

Test: `net::tests::ready_up_countdown` (host Start + guest Cancel stops both; all-ready auto countdown and un-ready
cancel; host Start with an un-ready guest starts; back in the lobby, all-ready starts by itself). `online.rs`'s
`lobby_start_ready_kick_and_name` checks the buttons. The ignored `short_code_on_a_testnet` now relies on the
all-ready auto start and passes.

## Not verified / not 1:1
- Not in the original (online is ours), so nothing to compare against.
- Not run by hand in the app across two machines (unattended run); covered by the local-connection test only.
