# B162 — tabbing out disconnects an online match

## Cause
Not the connection: iroh keeps its QUIC connection alive on its own task (1 s keep-alives, 30 s idle timeout) and
WebRTC data channels are kept by the browser, neither tied to the frame loop. ggrs is: its keep-alives and input
acks only go out when the frame loop calls `poll_remote_clients`, and its default `disconnect_timeout` is **2 s**. A
hidden tab (no `requestAnimationFrame`) or a stalled/minimised window stops that loop, so after 2 s the other peers'
ggrs fired `Disconnected` → "stopped answering. The match is over."

## Fix
- `play/rollback.rs`: `GONE` = 5 min, passed as ggrs' `disconnect_timeout`. Meanwhile the others stall within
  `WINDOW` ticks (ggrs' prediction threshold) with the existing "Waiting for 3P..." notice: the match is paused for
  everyone, no seat plays idle input, nothing in the sim or snapshot changes. (Chosen without asking: unattended run.
  Idle input would need the absent machine to keep ticking, which a hidden tab can't.)
- The datagrams the hidden machine misses wait in `net.rs`'s unbounded channels; when it comes back, `gate` pumps
  them into ggrs before polling, so it catches up instead of timing the others out itself.
- `play/netplay.rs` `watch`: after 5 s of waiting (rollback only) the notice adds "The match ends in M:SS if they don't
  come back."; on the timeout: "3P, 4P gone for 5 minutes. The match is over." (seats, not peer ids).

## Verified
- `rollback_survives_a_tab_out` (in `tools/check.sh`): the doubles harness, the guest's frame loop stopped at tick
  300 for 5 s of real time while its inbox keeps filling; the host must not run more than WINDOW+DELAY ahead, neither
  side ends, and the pad logs agree for 800+ ticks after. Without the fix it fails with both ending ("gone").
- `rollback_survives_a_tab_out_for_2_min` (ignored, 122 s): the same with 120 s away — passes.

## Not verified / not 1:1
- No live two-instance run (browser+desktop, browser+browser): an unattended run can't drive two lobbies and tab one
  out. The harness stops the frame loop exactly as a hidden tab does, but browser timer throttling of iroh's wasm
  keep-alives (≥1 s while hidden; Chrome's intensive throttling to 1/min starts only after 5 min hidden) is reasoned,
  not measured.
- Desktop minimise on Wayland: whether the loop stops (no frame callbacks) or crawls doesn't matter now — either way
  it's within the 5 min — but not observed.
- Pausing via the menu while a peer is away doesn't count toward the on-screen countdown (`stalled` only runs
  unpaused), but ggrs' timer still does, so the countdown can read longer than what's left in that case.
- Online play has no original to compare against (remaster addition).
