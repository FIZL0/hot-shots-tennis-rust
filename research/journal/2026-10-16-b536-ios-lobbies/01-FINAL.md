# B536: iOS app can't find or host online lobbies

## Compared with the web fix (B515)
B515's cause was Apple's networking stack (Safari's fetch/WebSocket) never reaching n0's relay hostnames with their
trailing dot. The native app doesn't go through that stack for iroh: the relay connection and its probes use rustls
with webpki roots (iroh-relay without `platform-verifier`) and hickory DNS (Apple's system DNS config isn't readable
in the iOS sandbox, but iroh's resolver falls back to public resolvers). Only our pkarr code-relay requests use
Apple's certificate check (reqwest's platform verifier), and those go to `relay.pkarr.org` without a dot. So the
dot fix doesn't carry over.

## What does differ natively on iOS: mDNS
`endpoint()` built mDNS (swarm-discovery: UDP 5353 sockets, multicast group joins) and returned `Err("mDNS: …")` if
that failed, which killed the whole online session: `run()` sends `Event::Failed` and never gets to Host, Find or
Join. An iPhone app may only use multicast with Apple's `com.apple.developer.networking.multicast` entitlement
(a sideloaded IPA doesn't have it); without it swarm-discovery fails to bind/join both its IPv4 and IPv6 sockets and
returns `CannotBind`, so everything online failed, matching the report (nothing to find, no hosting).

## Change
`with_mdns()`: a failed mDNS is logged (`mDNS: …: no LAN lobbies on this device`, lands in the iOS log file) and the
endpoint goes on without it: relays + DHT as on any machine, just no LAN list (`watch_lan(None)` is empty).
Test `no_mdns_still_goes_online` (hst-base); `lan_lobby_by_mdns` still passes. `tools/ios-build.sh check -p hst-base`
and the wasm32 check build.

## Not verified / not 1:1
- Not run on an iPhone (no device here), so it's unconfirmed that mDNS was what failed on the user's phone. If
  online still fails there, `hst-log.txt` will now hold the mDNS warning and iroh's `no home relay yet` line, which
  point to the next cause.
- LAN lobbies (relay Off) still won't work in the iOS app without the multicast entitlement; that needs an Apple
  developer request, not code.
- Not 1:1 with the original: it has no online play (n/a).
