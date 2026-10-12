# M2g — lockstep netplay

## What it does

- `crates/hst/src/play/netplay.rs`: the lockstep. `mode::Gate` (new: a mode's fixed tick runs only when the gate
  says so) runs `netplay::gate` before each match tick. It runs the tick only when every seat's pads for that tick
  have arrived, and hands them to the match as a one-tick `tick_input::Replay`, so the match never reads devices
  itself. Each machine sends its own seats' pads `delay` ticks ahead. The first `delay` ticks are idle for everyone.
- Input delay: the host pings every guest each second (`net.rs` `Ping`/`Pong`). When the room fills it pings again,
  waits up to 3 s, and sets `delay = ceil(worst RTT / 2 × 60) + 1`, clamped to 2..15 (`net::delay_for`, 250 ms RTT if
  nobody answers). The delay and a shared seed (the setup's weather/MT seed, `netplay::SEED` → `main.rs`) travel in
  `Room` with `Start`.
- Stall text: after 0.25 s with no tick, "Waiting for 3P, 4P..." is shown centred (`FontSize::Vh`, any aspect).
- Hashes: before every 30th tick each machine sends `snapshot::state_hash`. Hashes are compared whichever arrives
  first. On a mismatch the match stops on every machine ("Out of sync at tick N …"). Each machine saves
  `desync-<unix>-<peer>.json` beside the disc: the room (line-up, seed, delay) and every seat's pads for every tick
  run. Every machine runs the same pads, so the two files are the same input stream from both sides.
  `HST_DESYNC=file cargo test -p hst replays_a_desync -- --ignored` replays one and prints the hash every 30 ticks.
- Pause: opening or closing the pause screen sends `Pause(bool)`, which opens or closes it on every peer
  (`menu::set_paused`). The post-point press is a pad press, so it goes with the pads.
- A peer dropping: the host relays `Bye("Player xxxx left.")` to the others. A guest that loses the host shows
  "The host left.". The match ends for everyone with the message on screen and goes back to the menu 5 s later.
  Leaving through the pause menu closes the connection, which counts as dropping. No bot takes the seat.
- Transport (`net.rs`): star topology through the host. Each stream has a writer task that holds each message until
  it is due (`HST_NET_LAG=ms`, one way, simulated latency), so order is kept.
- Starts: from the menu (Online → lobby full → `Event::Start` → `online::launch` builds the match flags from the room;
  remote seats are human seats on a pad index no machine has, so every machine sets up the same humans), or
  `hst net host|join … --play <disc>` (this machine's seats: keyboard, then pads 0, 1).

## Verified

- `doubles_match_at_100ms_stays_in_sync`: two headless match apps (the snapshot harness, doubles: 1P+2P on the host,
  3P+4P on the guest). Every message is held 6 ticks (100 ms) each way, the delay comes from `delay_for`, and every
  seat presses random buttons. The match is played to its result, every hash agrees, and both pad logs are the same.
- `a_desync_stops_both_and_saves`: one tick's pads flipped on one side → both stop at tick 120's hash and both save.
- `lockstep_over_a_slow_link`, `hashes_compare_whichever_comes_first`, `pads_wire_round_trip`,
  `flags_put_each_seat_in_match_order`.

## Not verified / not 1:1

- The original has no online play: nothing here is checked against the game. It's an addition, and with no `Netplay`
  resource the gate is absent, so offline matches run as before.
- Two separate processes over a real network with the 100 ms lag were (see below).
- The delay is fixed at the start (one measurement). A link that gets slower mid-match stalls instead of adjusting.
- Star topology: guests' pads reach each other through the host (a guest↔guest hop is two links' lag).
- No bot takeover option (the task says "unless asked"; nobody asked).
- Local pads are sampled at the 60 Hz tick as `tick_input::local` does, so a press made between two network-stalled
  ticks is kept (latched) but its timing is the tick it's sent on.
- The acceptance match's players press random buttons (`HST_NET_MASH`), not real play.
