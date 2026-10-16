# B515: iOS web host stuck on "Waiting for a relay"

## What the code does
`host()` waits (in 30 s steps) for the endpoint's home relay before publishing the short code (B175/B266: a
browser can only dial the ticket's relay). In the browser iroh 1.3 picks the home relay from its net report, which
in wasm runs **only HTTPS probes**: a `fetch` of each relay's `/ping` (`iroh-relay` serves it with
`Access-Control-Allow-Origin: *`), each given up after 3 s (`PROBES_TIMEOUT`). Then the relay connection is a
WebSocket. If every probe fails or takes more than 3 s, no report names a relay and the endpoint never gets a home
relay, so the host never gets past this line. Without the phone's log there's no telling which of these it was:
- the fetches fail (no internet, a content blocker, VPN, iCloud Private Relay);
- they answer, but too slowly (more than 3 s, e.g. through Private Relay's two hops on a slow network);
- the probes are fine but the WebSocket is blocked or dropped.

## Change
When the 30 s run out, `relay_check()` fetches every relay the endpoint was bound with (`RELAYS`, recorded in
`endpoint()`: iroh's `Endpoint` doesn't expose its relay map), the way the probe does, 10 s cap each, and
`relay_words()` turns that into the status line, logged as a warning too (it's in the console / "Copy the error"):
- `the relays answer (X in N ms) but none has taken the connection yet: … may block their WebSocket`
- `the relays answer too slowly to be picked (X in N ms, more than 3 s): …`
- `none of the relays can be reached (X: <fetch error>): no internet, or … blocks them`
Each relay's result is also logged at info (`relay check <host>: …`).

Tests: `relay_words_name_the_failure`, `relay_check_reports_an_unreachable_relay` (hst-base). wasm32 build checked.

## Not verified / not 1:1
- Not tried on an iPhone (no device here): the actual cause on the user's phone is still unknown. The new status
  line will say which of the three it is; the fix follows from that (e.g. too slow → fall back to our own probe's
  fastest relay; WebSocket blocked → a different relay/port). That's why the task stays blocked on the human.
- Not 1:1 with the original: online play doesn't exist in it (n/a).
