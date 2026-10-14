# B266: the browser said "Relay off" for an n0 host (FINAL)

## Cause (`net.rs host()`)
A browser can only dial the relay URL in the ticket stored under the short code. The host could publish that
ticket without a relay URL, or with a relay it had stopped using, in three ways:
- The first publish only waited 30 s for a home relay (B175). If none had come by then, the code went out with
  direct addresses only. A browser reads that as Relay off.
- In iroh 1.3 the home relay comes from each net report (`relay/actor.rs on_network_change`). A report without
  a preferred relay *clears* it (`my_relay.clear()`), so `ep.addr()` has no relay until the next report. The
  30 s republish took `code_ticket(ep)` as it was at that moment, so it could publish a relay-less ticket
  → "That host has Relay off".
- A report that picks a different relay changes the URL. The record kept the old one until the next republish
  (≤ 30 s), and a joiner dialling it in that window waited for nothing → "Couldn't reach the host (timed out)".
  This is the likely cause of the reverse direction (phone hosts, desktop joins), since a phone's network
  changes often.

## Fix
- The roll task waits for a relay with no time limit while the relays are on. After each 30 s with none, the
  status says "Waiting for a relay to put the code online (no internet?)...". It stops when the host leaves.
  With relays on, a code is never published without a relay, so a relay-less record now really means Relay off,
  and `unreachable()`'s message stays as it was.
- A watcher on `ep.watch_addr()` sends each new home relay to the host loop. On a change, the record is
  republished at once (`dirty`, `republish`), not at the next 30 s tick.
- `code_ticket(ep, home)`: while the address has no relay (a report cleared it), the last home relay goes into
  the ticket, so a republish never drops it.
- Test `net::tests::code_ticket_keeps_the_last_relay`. `net::` (25) and `short_code_on_a_testnet` (ignored) pass.

## Runs (`context/b266/`, rig copied from `context/b175`: headless Chromium on Vulkan, web/pkg built from this tree)
| run | setup | result |
|---|---|---|
| run3 | native host n0, browser joins the short code straight away | dials `Relay(use1…)` + IPs, "Player joined" at 19 s |
| run4 | native host `--relay off`, browser joins | fails at once: "That host has Relay off (LAN only)…" |
| run7 | browser Host, native join | in the lobby at 5.5 s |
| run8 | browser Quickplay (no lobbies) → hosts, native join | in at 5.2 s |
| run9 | browser Quickplay with a dead lobby on the board (30 s time-out), then hosts, native join | in at 5.2 s |
| run10 | native host with an unreachable own relay | no code published; status at 31.8 s "Waiting for a relay…" (before: published relay-less at 30 s) |
| run6 | browser Quickplay with stale driver keys (several Quickplay presses, joins abandoned mid-dial, one relay-off listing), then hosts, native join | **timed out**, though the record had the browser's relay. Not reproduced in run8/run9 |

## Not verified / not 1:1
- Not 1:1: online play is our addition, so the original has nothing to compare it with.
- A home relay cleared or switched while the lobby is up was found by reading iroh's source, not seen on this
  machine (it stayed on use1 throughout). The fix for it (republish on change, keep the last relay) is covered by
  the unit test and the log line `home relay Some(…)`, not by a live relay switch.
- run6's time-out (browser Quickplay after several abandoned joins, then hosting) wasn't reproduced cleanly, so
  its cause is unknown. If a phone host still times out, the phone's console lines (`home relay`, `short code`)
  and the desktop's `dialling` line are needed.
- One machine and one network only, as in B175. No real phone was used.
