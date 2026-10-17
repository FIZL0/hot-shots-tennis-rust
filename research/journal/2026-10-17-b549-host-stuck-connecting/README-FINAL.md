# B549 — host stuck on "Connecting to the relay" with a guest in the lobby

## Cause (from the code; the user's report had no log)
The host's lobby turns joinable (`Event::Joinable`) only when its short code went out (`rolled` → `host` loop) or a
put failed (the retry branch in the code task). Two stages had no time limit:
- the home-relay wait (`ep.online()` loop): endless while no relay connects, by design (a relay-less code breaks
  browsers, B266) — it already says "Waiting for a relay..." in the status;
- the code check + put (`roll` → `publish`): `dht.put_mutable` and the pkarr relay's PUT (`modstore::client()`,
  reqwest with no timeout) can stall for good, and then neither branch ever fires.
Guests still reach the host through the board, LAN or link, so the lobby filled while the host said friends can't join.

## Fix (crates/hst-base/src/net.rs)
- `ROLL_CAP` (60 s) around the host's `roll`: no answer counts as a failed put → the ticket as the link, Joinable,
  retried with the usual backoff. The republish's `publish` has the same cap (a stalled put is a missed republish,
  not a hanging task).
- A guest seated before the code is out (relays on) sends Joinable, with a log line.
- Stage logs: "short code: waiting for a home relay", "short code X: checking and publishing it (t in)".
- Test `a_joined_guest_makes_a_stuck_host_joinable`: a relays-on host whose endpoint never gets a relay (stuck in
  the relay stage) is not joinable until a guest joins by its ticket, then is. Fails without the fix.

## Android emulator (hst34 read-only, port 5596, x86_64 APK)
Online → Host: log shows `short code G9XC shown` → `checking and publishing it` → `short code Some("G9XC") in 9.6 s`,
chip "Online, joinable" (screens in `context/b549/`). The normal path works on Android and logs its stages.

## Not verified / not 1:1
- The stall itself wasn't reproduced on the emulator: its network put the code out in ~10 s; a stalled DHT/pkarr
  request can't be forced there without changing the read-only image's network. Covered by the loopback test for the
  relay stage and by the timeout's construction for the put stage.
- No guest joined the emulator's host (it sits behind the emulator NAT); the join → Joinable path is the unit test's.
- The 60 s cap is our choice (relay put 5-20 s plus a lookup up to 15 s); networking/front-end, nothing in the original.
