# 1: reproduce, fix (FINAL)

## Setup (scratch, `context/b114/`, git-ignored)
- A copy of the release binary (the shared `target/` is rebuilt by other slots).
- The menu instance: `tools/shot.sh` with an `HST_BIN` wrapper (`menu.sh`) that sets `HST_CODE=<short>` (the menu's
  link path: straight to the character pick, then the same `Cmd::Join` that typing the code sends) and
  `HST_KEYS="8:Enter …shot=…"`. The ISO is a symlink in `context/b114/iso/` next to an `online.txt` with
  `relay = off`, the same as the user's own `online.txt` (`relay = off`, written 2026-10-12).
- Browser: a WebGL2 build of this branch served on :8775 (B175's origin, so the copied B175 Chromium profile already
  has the pack in OPFS), `tools/web_check.mjs` with `?code=<short>`, click + Enter.

## Results
| run | setup | result |
|---|---|---|
| CLI | `hst net host` / `hst net join <short>`, relay n0/n0, off/off, n0/off, off/n0 | all joined and started in 3-6 s |
| 3 | menu joiner (relay off) | my key script pressed Enter on the lobby's Leave (only button before the room arrives): back to Join by Code |
| 4 (before) | menu joiner, one Enter | **seated in the host's lobby, but the status line still read "Joining d98d4770..."** for the rest of the run: this is the "sits on joining forever" |
| 5 (fixed) | same | "Joined: choose Ready when you are." |
| 5 | `join <bare ID of no endpoint> --relay off` | before: after 10 s "Couldn't reach the host: No addressing information available" (= the user's "no address"); now the same 10 s, with a clear message |
| 5 | `join ZZZZ` | "No open lobby has the code ZZZZ." after 20 s |
| web3 (fixed) | native host n0, browser joins by `?code=` | seated, "Joined: choose Ready when you are." |

## Causes and fixes (`crates/hst/src/net.rs`)
- **Endless "Joining…"**: the joiner's last status was "Joining <id>…", and nothing replaced it once the host's
  lobby came in (`Event::Room` doesn't touch the status), so a seated guest looked stuck. Now `join()` says
  "Connected: waiting for the host's lobby..." after connecting and "Joined: choose Ready when you are." on the first
  room.
- **No lobby after connecting**: `join()` waited for the host's first room with no limit. Now it gives up after 20 s
  with "The host didn't answer (no lobby after 20 s)."
- **"no address"**: iroh's `ConnectWithOptsError::NoAddress`. It happens when the dialled address has no IPs and no
  relay, and the DHT/mDNS lookup finds nothing either. A code resolves to a bare ID when the host rolled its code
  before its endpoint had any address. That can happen with relay off, because `endpoint()` doesn't wait for
  anything then. Now the roll waits (≤ 5 s) for an address first, and the join turns `NoAddress` into "Couldn't find the
  host's address: the lobby may have closed, or the host has Relay off on another network (ask them to set Relay
  (Online) to n0)."
- **Code lookups**: `resolve()` asks again every 3 s for up to 20 s instead of once (≤ 30 s). A freshly rolled code
  can take a few seconds to reach the pkarr relay (the browser's only source, its put runs in the background, B151)
  or the DHT nodes the joiner asks.
- Every `Failed` is logged at info (`online: …`), so a failing join's log says why.
- Timeouts on every step now: bind ≤ 10 s, code lookup 20 s, dial 30 s, lobby 20 s. B175 already covered the fast
  failures for browser ↔ relay-off hosts.
- Test: `short_code_on_a_testnet` checks the "Joined" status. `lan_lobby_by_mdns` now only looks for its own host,
  because the other test's host (alive longer now that the unknown-code check retries) was on the LAN too.

## Not verified / not 1:1
- Not 1:1: online play is our addition, so the original has nothing to compare it with.
- The codes were not typed on the on-screen key grid by a driven instance. `HST_KEYS` only has arrows and Enter, and
  the browser rig only clicks and presses keys. Both runs went through the link path (`HST_CODE` / `?code=`), which
  sends the same `Cmd::Join(code, picks)`. The menu test `join_by_typing_a_short_code` covers typing → that command.
- A second browser joining a browser host was not run here. Only a native host with a browser guest was run. B175 ran
  browser host ↔ native guest, and nothing in this change is specific to which side hosts.
- The user's exact failing run wasn't seen. The "no address" path was reproduced with a bare ID, not with a code
  rolled too early. That race wasn't caught on this machine, where the address was always there at roll time.
- One machine, one LAN: two networks are still M2f's "Needs the human".
