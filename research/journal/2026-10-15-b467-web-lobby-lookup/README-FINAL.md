# B467: two iOS web apps couldn't find each other's lobby or join by code

User report: iOS web app to iOS web app, same version. The friend couldn't see the host's lobby in Find, and joining
by code failed.

## What's wrong
The browser has no DHT. Short codes and the 8 board slots (Find's list) go through the pkarr relay
`relay.pkarr.org`, over HTTP.

- **The relay rate-limits by IP.** Each address gets 10 requests at once, then about one a second. Over that it
  answers `429` with `Retry-After: 1` (checked with curl: 14 at once gave 10 × 200 and 4 × 429).
- **Its 429 replies carry no CORS headers.** The 200s have `access-control-allow-origin` (it mirrors any origin); the
  429s don't. So in a browser a refused request isn't seen as a 429. The fetch just fails: "Failed to fetch" in
  Chromium (16 fetches at once from a page on opentennis.dev: 10 × 200, 6 × failed), "Load failed" in Safari.
- **The browser makes a lot of these requests.** Opening the Online screen does a Find, which is 8 slot reads. Host
  then adds a code lookup and a put, a slot read, 8 more slot reads and a slot put. That's over 20 requests in a few
  seconds. Two phones on the same Wi-Fi share one IP, so they share the budget.
- **Every failure was treated as "nothing there".** `relay_get` returned None on any error. So a refused code lookup
  became "No open lobby has the code K7QX", and refused slot reads quietly left lobbies out of Find ("None open
  right now"). A host whose code record went stale got the same "No open lobby" message. On iOS that happens when
  the host's Safari goes to the background (for example, to send the code in Messages).

## Changes (`net.rs`, `play/main_menu/online.rs`)
- `relay_get` / `lookup` return `Result<Option<..>, String>`: 404 means none; a failure comes back as a reason
  (`relay_trouble`). A 429, and in the browser any failed fetch, waits 1–3 s with jitter (`relay_pause`) and tries
  again, up to 8 times; the callers' timeouts cut that shorter. `relay_put` retries the same way, up to 5 times.
- Join by code (`resolve` → `missed`) now names the actual case:
  - no record: check the code against the host's screen;
  - a stale record: "stopped answering N minutes ago … the host's game has to stay open on screen";
  - the lookup failed: the code server's reason.
- Find: `find_slots` returns why some slots couldn't be read. If every lookup is done and nothing was listed, Find
  sends `Failed("Couldn't read the lobby list: …")` instead of an empty list. The Find screen shows it as "Couldn't
  search for lobbies" with the reason below. The Online screen's card shows the reason (`View::why`) where it said
  "None open right now", and its Lobbies chip says "Couldn't check". The next answer clears it.
- Host: when a code or slot put fails, the lobby status says "Others may not find this lobby: … Trying again in
  30 s"; when one gets through again: "The lobby is listed again". The status while retrying the short code now
  includes the reason.
- Shot: `HST_MENU=online-unread` (`context/b467/unread-1280x720.png`).

## Verified
- `slot_through_the_pkarr_relay` (ignored, needs the internet; `tools/check.sh -p hst --bin hst
  slot_through_the_pkarr_relay -- --ignored`) passes. It now also does 12 reads at once against the live relay, which
  is over its limit of 10; all 12 answer, and a name nobody wrote reads as `Ok(None)`. With 15 at once, one read was
  still refused after 8 tries in one run, so the test uses 12.
- `why_a_code_or_find_came_up_empty` (messages; when Find turns "none" into a failure) and
  `online_card_says_why_it_found_none` (menu); all 448 hst tests pass; the wasm32 build checks clean.

## Not verified / not 1:1
- **Not reproduced in two Safari tabs or on two iPhones.** There's no Apple device here, and Playwright's WebKit
  won't start on this Arch host (missing ICU 66 / libxml2). The CORS-less 429 was confirmed in headless Chromium
  through the Fetch API. That is spec behaviour, so Safari should fail the same way, with "Load failed".
- The browser build's own retry path (wasm `relay_get` on a failed fetch) wasn't run end to end in a page. The same
  code ran natively against the live relay, and the wasm build compiles.
- An iOS host's tab in the background still stops republishing and drops its relay connection. The guest now says
  so ("stopped answering … background"), but the host side can't prevent it.
- The number of requests itself isn't reduced: 8 slot reads per Find. A board that needs fewer reads would need a
  server of ours (the SLOTS ponytail).
- Only the 1280×720 shot came out; the 960×720 and 1600×640 runs gave black frames under load average ~20. The
  card's text is `para`, wrapped to the card's width.
