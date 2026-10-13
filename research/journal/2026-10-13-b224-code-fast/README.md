# B224: the match code appears at once (FINAL)

Task: hosting took long to show a match code (user, mobile/web); and no QR or Copy for the long endpoint ticket.

## Where the time went (before)

From `Net::start` (Host pressed) to the short code on screen, in order, all awaited one after another:

| step | native (B151 + run1 here) | browser |
|---|---|---|
| endpoint bind | ms | 5-7 ms |
| waiting for the home relay (`ep.online()`, inside `endpoint()`, ≤10 s) | ~3 s | ≤10 s (not timed before the change) |
| short code: DHT/relay lookup that the random code is free | ~0.6 s (DHT) | relay GET, s |
| short code: publish (DHT put, the relay PUT in the background) | ~1-3 s | relay PUT 5-18 s |
| **total** | ~4.9-6.5 s | ~5-30 s (relay PUT dominated) |

## What changed (`net.rs`, `online.rs`)

- `endpoint()` (native and browser) no longer waits for the home relay. Only two things need it, and both now wait
  for it themselves: the short code's ticket (the roll task already waited for a relay URL, B175) and `join()`
  (same bounded 10 s wait, moved there so joins behave as before).
- `host()` rolls the 4-character code locally and sends `Event::Short` at once. The background task then waits for
  the relay and addresses, checks that the code is free and publishes it. If the code is taken, it re-rolls and sends
  the new code to the screen. If publishing fails, `Short("")` clears it with "No short code (no internet?)…". The
  status line reads "Putting the code online..." until the publish lands, then "Waiting for players...".
- Menu: the QR and Copy Link share only `shared()`, which is the short code's link, or the browser LAN's WebRTC
  offer (relay Off: that has to be scanned, and nothing else reaches it). The long `endpoint…` ticket is never shown
  as a QR or copied. Until there's a short code the lobby shows only "Getting the code...".

## Timings (after)

- Native, `hst net host` (n0 relay, real DHT): `Short` at **0.05 s** in both runs (`context/b224/run1`, `run2`);
  published at 6.41 s / 6.56 s.
- Native join typed right after the code appeared (`run2`): `hst net join SL6D` started at 0.05 s, looked it up
  (resolve retries every 3 s for 20 s), joined at 11 s, both reached `Start`.
- Browser (headless Chromium, WebGL2, `context/b224/web1`, `web3`, driver `context/b224/bhost.mjs`): bind ("online
  in") 4.5 ms / 7.4 ms, then `short code … shown`, with no await between the host starting and the event. The relay
  publish finished 5.3 s / 4.6 s later. The driver's "after the last key" (1.2 s / 2.2 s) is measured from a key in
  the Quickplay flow (it waits 3 s on the LAN first), not from a clean Host press.
- Tests: `net::short_code_parts`, `net::short_code_on_a_testnet` (the host's first `Short` is now the unpublished
  code; the guest joins by it straight away and still gets in), `online::join_by_typing_a_short_code`.

## Not verified / not 1:1

- Front-end only, so nothing to match against the original.
- A joiner who types the code within the first ~5 s waits longer inside "Looking up…" (resolve retries until the
  publish lands, up to 20 s). The host's code shows sooner, but join-by-code doesn't get any faster in total. For a
  browser host the publish is the pkarr relay PUT (5-18 s), so a very fast typist can still hit the 20 s limit.
- When a code is taken (rare, ~1M codes) the code on screen changes once the check comes back (≤15 s).
- No browser screenshot of the lobby with the code: my key sequence for headless Chromium went through Quickplay,
  not the Host lobby. So the browser is timed from its console log, not from a screenshot of the lobby.
- Not tried on a real phone.
