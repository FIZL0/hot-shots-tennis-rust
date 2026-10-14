# 1 — ggrs' 128-input drop, a resend, the replay clock (FINAL)

## Logs
No logs from the user's match: nothing in `~/.local/share/hst`, no saved desync. Their build when filing (c2927da6)
was before B196/B197 (ggrs window 8). So the cause below is reproduced in the in-process harness, not confirmed
from their match.

## What can wait forever (ggrs 0.13 and our code)
- **ggrs drops a peer after 128 unacked inputs**: `send_queue_len > 128` → `Event::Disconnected` →
  `disconnect_player_at_frame`, and that endpoint is shut down for good. That side plays on without the peer; the
  other side, which still waits on its inputs, is frozen forever. With WINDOW 80, a one-way loss of ~3 s (wifi
  hiccup, a stalled route) is enough. **Reproduced**: `a_one_way_outage` without the fix left the guest at tick 219
  while the host ran to 620.
- ggrs resends pending inputs only when it got no input from that peer for 200 ms. Any input from the peer, even a
  duplicate, resets that timer. So a peer that keeps getting resends from a frozen side may never resend its own.
  This was not reproduced, but nothing stops it.
- Not reachable: intro lockstep (`Msg::Intro`) goes over reliable ordered streams (native iroh streams, browser
  reliable data channel), and `Net::poll` stops at `Event::Start`, so no intro message is lost. Pause
  (`Msg::Pause`) races leave at most one side paused, never both waiting.
- At ggrs' prediction threshold `advance_frame` returns nothing and `rollback::advance` clears `stalled`, so in
  rollback mode "Waiting for nP" only shows from the intro, a session not Running (start sync, B196 restart),
  end/recover hold, or a gate that isn't called often enough.
- **The replay clock** (matches the symptom: both games running, both showing the wait): `replay_pace::pace` put
  `Time<Virtual>` back to 1× only when it took a `Pace::replay`. `Pace` is in the snapshot, so a rollback or resync
  to a state from before the replay started (no `replay` in it) left the clock at 1/31 (slow-mo) or 1000× (skip).
  At 1/31 the gate runs ~2 ticks/s: that peer crawls and the others wait on it, and with the replay on both peers
  both can crawl at once, until the next replay. Not reproduced (needs a rollback across the replay's first tick).

## Fixes
- `rollback.rs advance`: hold (don't advance, `held += 1`) while any remote's `send_queue_len >= UNACKED` (120),
  below ggrs' 128 drop. The peer's inputs are then still pending and ggrs resends them, and play goes on once the
  other side acks.
- `rollback.rs resend` (from `poll`): when the session's frame hasn't moved for `RESEND` (200 ms), re-queue each
  peer's last ggrs Input message (`Wires::last`, recorded in `send_to`) every 200 ms. It holds every pending input
  since the peer's last ack, so the other side gets what it misses whatever ggrs' own timer does.
- `replay_pace.rs pace`: outside a replay, put the clock back to 1× whenever it isn't, not only when a replay ends.
- `net.rs`: `DATAGRAM_MAX` / `fits()`, the size the harness drops datagrams over (the oversize theory was ruled
  out: the biggest seen is ~520 bytes).

## Tests (`play::netplay::tests::matches`)
`lossy(two, loss, lag, down, oneway)` runs 2 or 4 in-process peers over a fake net: 20 % datagram loss, 5 rounds
of delay each way (~170 ms RTT at 60 Hz), then a full outage of 180 rounds (3 s), both ways or one way, then 300
rounds after it. Each test checks: the longest run of rounds where no peer ticked (outside the outage + 60 rounds,
i.e. it clears within a second) is < 60, and all peers' tick logs agree (> 400 ticks), with no end and no desync.
- `two_peers_on_a_bad_network`, `four_peers_on_a_bad_network`: pass, longest wait 1–8 rounds (~11 s together).
- `a_one_way_outage`: pass (the guest holds ~70 ticks during the outage, then catches up); fails without the hold.
- Without `resend` all three still pass: it is insurance for the ggrs timer case above.
- Also run: `tools/check.sh -p hst --bin hst netplay` (21 pass; `rollback_desync_is_saved_and_play_goes_on` fails
  as on main since B231, tracked as B249), `replay_pace` (2 pass). Ignored runs: `full_doubles_match_rollback_at_150ms`,
  `rollback_survives_a_tab_out_for_2_min`, `synctest_through_a_change_of_ends`,
  `a_silent_peer_rejoins_and_a_silent_host_ends_it` pass; `a_resynced_match_finishes` panics on the same 'the
  desync's note' as B249 (same forced-desync path B231 broke), noted on B249.

## Not verified / not 1:1
- Online play has no original; nothing to match.
- Root cause of the user's match not confirmed: no logs from it. The 128-drop is reproduced; the replay clock is
  a fix by reading, not reproduced.
- Only the in-process harness: no live 2- or 4-machine run and no browser run with injected loss (no loss knob in
  the browser build, and no second machine in an unattended run). The harness uses the same ggrs sessions and
  `Wires`, but not the iroh/WebRTC transports.
- "Whole matches" under loss: the harness runs ~1000 rounds (start sync, a 3 s outage and recovery), not a whole
  match, to keep each test under ~10 s.
- The resend's own deadlock (ggrs' timer reset by the other side's resends) wasn't reproduced.
- Left as is: a peer whose intro never starts (no window, CMN.XB missing) leaves the others in the intro
  lockstep forever. One-sided, not a mutual wait; B196's recover covers silent peers only after the intro.
- 4-peer ggrs start sync over 20 % loss can take up to ~8 s; the court intro covers it, and the tests measure from
  when all sessions are Running.
