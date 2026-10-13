# 1: reproduce, find the stall, fix (FINAL)

## Setup (scratch, in `context/b175/`, git-ignored)
- `run.sh OUT [host args]`: native `hst net host`, then headless Chromium (`tools/web_check.mjs`, own port 8775, own
  profile with the pack in OPFS) opens `?code=<short>&webgl2` and joins.
- `run_bhost.sh OUT [join args]`: the browser hosts (`bhost.mjs`, from `tools/web_lan.mjs`), writes its short code,
  presses Start after 20 s; native `hst net join <code>` joins.
- Chromium on the real GPU (`--use-angle=vulkan --enable-features=Vulkan`). With SwiftShader the tab runs so slowly
  that ggrs drops it ("stopped answering"), which is a test-rig problem, not the bug.

## Results
| run | setup | result |
|---|---|---|
| 1-11 (before the fix) | native host n0, browser joins; browser host, native joins; mismatched home relays (use1 vs euc1) both ways; `HST_RELAY_ONLY` both ways | all meet, play the match; rollback hashes identical on both sides to tick 3600 |
| probe | relay WebSocket (`wss://…/relay`, subprotocols `iroh-relay-v2/v1`) and pkarr fetch in Chromium **and Zen (Gecko, Firefox 156)** | opens/200 in both: the transport is not engine-specific; the trailing dot in n0's relay host is fine |
| 12 | native host `--relay off`, browser joins | **reproduced**: the browser sits on "Joining…" for 30 s, then "Couldn't reach the host (timed out)" |
| 13 (fixed) | same | fails within ~2 s: "That host has Relay off (LAN only), and a browser can only reach a host through a relay: ask them to set Relay (Online) to n0." |
| 14 (fixed) | native host n0 `--play`, browser joins | match runs, hash at tick 1800 `6afafb5c128a19bf` on both |
| 15 | browser host, native `join --relay off` | timed out: **stale binary** (another slot rebuilt the shared `target/release/hst`) |
| 16 (fixed, rebuilt) | same | fails in 2 s: "That lobby can only be reached through a relay (a browser's?): set Relay (Online) to n0 to join it." |
| 17 (fixed) | browser host, native join n0 | lobby, Start |

## Cause
The only "timed out" in the lobby code is `ep.connect` in `net.rs connect()`: no path exists between the two endpoints.
The browser has no UDP, DHT address lookup or mDNS, so the only way it can reach a host is the relay URL inside the
host's ticket (the one stored under the short code). Native↔native never notices a missing relay: the DHT lookup and
direct addresses still find the host. So it fails exactly when the native side has no relay in play:
- the native host has `relay = off` in `online.txt` (Relay row → "off"): its ticket only has LAN addresses, which a
  browser can't dial → 30 s timeout;
- a native joiner with `relay = off` can't use the relay, which is the only address a browser host has → 30 s timeout;
- (rarer) a native host with relays on whose home relay took longer than `endpoint()`'s 10 s wait: the short code was
  rolled from a ticket without a relay URL and never updated.
With the defaults (n0) everything worked here in both directions, including different home relays.

## Fix (`crates/hst/src/net.rs`)
- `Ctx.relays` (relay n0/own vs off).
- Host: the short code's ticket is taken in the background roll task, after waiting (≤ 30 s more) for the home relay
  when relays are on and the endpoint has none yet, so a browser can always dial the code.
- Joiner: `unreachable()` checks the dialled address before the 30 s connect: in the browser, an address with no
  relay fails at once with the reason; natively with the relays off, a ticket with no direct address (a browser
  host's) fails at once with the reason. Bare IDs (found by mDNS/DHT) are left alone. Test
  `net::tests::relay_off_fails_fast_on_relay_only_tickets`.
- `dialling {addr:?}` info log, so a failing join's log shows what the code resolved to.

A native relay-off host can't be made reachable from a browser at all (no relay; the browser's LAN is WebRTC, which
the native build doesn't speak, M2k), so a clear message is the fix there; with Relay n0 both ways meet.

## Not verified / not 1:1
- Not 1:1: online play is our addition, so the original has nothing to compare it with.
- Only one machine was used. Both peers shared one LAN and one internet connection. Two networks (M2f's "Needs the
  human") are untested, but with `HST_RELAY_ONLY` and different home relays the traffic took the same relay-only path
  a browser on another network would take.
- The user's own failing setup wasn't seen: there is no `online.txt` on this machine, so I can't tell whether
  relay Off was their cause. If it still times out with Relay n0 on both sides, the joiner's `dialling …` line and both
  sides' logs/browser consoles are needed.
- Only Chromium (Vulkan) played full matches. Gecko (Zen) was only checked at the transport level (the relay
  WebSocket and pkarr fetch), and no match was played in it. Safari and phones were not tested.
- Find/Quickplay still don't list lobbies across the two builds: the browser has no DHT, so it neither lists nor
  announces. Join by code/link is the cross-build route.
