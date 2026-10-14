# B321: browser host shows no join code (FINAL)

## Setup (scratch in `context/b321/`, git-ignored)
- `bhost.mjs ISO|- OUT [hold]`: headless Chromium (own ports 8781/9341, own profile with the pack in OPFS) loads the
  page fresh, goes to Online and on to Host at once (`KEY_MS=150`, so it overlaps the Online screen's Find), reads the
  host's code from the console, then a second tab joins by `?code=`. `NO_GUEST=1` only holds the lobby (for
  `target/debug/hst net join <code>`); `TICKET=1` waits for the ticket fallback's link; `BLOCK_PKARR=1` makes
  relay.pkarr.org unresolvable (`--host-resolver-rules`) to force that fallback.
- "before": main's existing `web/pkg` (pre-fix); "after": this branch's wasm (`tools/web.sh`'s `pkg` step, no wasm-opt).

## Cause (two things, both on the first host after a fresh page load)
1. The network runs one command at a time. Entering Online starts the network with a background `Find`. In the
   browser that is 8 board-slot reads through the pkarr relay (an unwritten slot takes ~7 s, 12 s timeout). A Host
   chosen meanwhile waited behind it, so no `Hosting`/short code arrived (spinner, "....") until Find ended. The
   "before" run showed `online` at 10.9 s and the code at 14.3 s, with the host's keys done well before. When the
   network was already up, a second Host had no Find to wait for.
2. Those slot reads use up the pkarr relay's rate limit (~10 requests). The short code's put that comes right after
   is refused. In the browser this shows as `error sending request`, because a 429 carries no CORS header. `roll`
   then returned None, the host cleared the short code, and the lobby showed "...." for good with "No short code (no
   internet?)". Reproduced in "after" run 1: two failed puts, then the retry got it out at 14 s. With the old code
   that lobby would never have had a code. Board-slot puts failed the same way in every run, so the lobby stayed off
   Find until the next 30 s REPUBLISH. That is the "took a few Find refreshes" note.

## Fix
- `net.rs run`: during a Find, a Host/Join/Quick/Find that comes in drops the search and runs at once. A Name is kept
  and applied. Test `net::tests::host_cuts_a_running_find`: Host during a native Find (3 s LAN wait) gets its Room in
  under 2.5 s, and the Name sent alongside is used.
- `net.rs host`: a failed short-code put is tried again (2 s, doubling up to 30 s) while the lobby is open, relays on.
  Until it goes out, the host sends `Short("")` and `Code(ticket with its relay)`. The lobby shows **JOIN LINK / LINK**,
  and its QR and Copy link carry `SITE?code=<ticket>`, which both builds already join (`code_in`). The link is logged
  once as `lobby link …`. When the put succeeds, the short code replaces the link.
- `net.rs list_in_slot`: a refused slot put is tried again 3 s and 6 s later.
- `online.rs`: `shared`/`ticket_only` (the ticket link only while hosting with no short code). The code label reads
  JOIN LINK then. Copy link copies the ticket link. `online_open` clears an earlier lobby's code/short and the
  dropped Find's count.

## Results (after)
| run | setup | result |
|---|---|---|
| after1 | fresh page, Host at once | 2 refused puts, then `short code Some("JRZ3")` at 14 s; lobby showed LINK + QR meanwhile |
| a1 | fresh page host, browser guest by `?code=3TDD` | guest seated ("Player joined.") |
| b1 | fresh page host, native `hst net join 27UR` | Joined in 6.5 s, Room received |
| c1 | `BLOCK_PKARR`: host has no short code, browser guest by the ticket link | JOIN LINK shown, guest seated |
| c2 | same, native `hst net join <ticket>` | Joined in 4.4 s |
`short_code_on_a_testnet` (ignored) passes.

## Not verified / not 1:1
- Not 1:1: online play is our addition, so the original has nothing to compare with.
- The user's own failing first host wasn't seen. Cause 2 was reproduced here, but I can't tell which of the two (or
  both) hit them.
- Only Chromium headless (SwiftShader) on one machine and one network. Firefox, Safari and phones weren't run.
- The slot put also gets `428 Precondition Required` when an earlier put of the same slot is still in flight on the
  relay (a lobby change right after a REPUBLISH). Those retries mostly hit 428 again, and the next REPUBLISH lists it.
  I left this alone (it's a pkarr relay conflict, not a missing listing).
- No match was played after the joins; that path is unchanged and covered by B175's runs.
