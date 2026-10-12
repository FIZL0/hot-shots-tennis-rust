# M2j — LAN play with no internet

## What landed
- `net.rs`: every endpoint adds iroh's mDNS address lookup (`iroh-mdns-address-lookup` 0.6, service `_hst._udp`),
  so a bare endpoint ID resolves on a shared network with no DHT and no relay.
- A host puts its listing (`hst1` + Listing JSON without ID/time, ≤ 245 bytes even for a 24×4-byte name) in the
  mDNS user data (`set_user_data_for_address_lookup`) on every room change, and clears it at the start or on leave.
  A watcher task keeps the LAN lobbies current (Discovered / Expired).
- Find: waits 3 s for mDNS, sends `Found(LAN list)`, then `Found(LAN + DHT)` (DHT entries for a host already heard
  on the LAN are dropped). Quickplay picks from the same merged list. The menu already replaces its list on each
  `Found`. `hst net find` exits after the second one.
- The code/QR (ticket) already held the host's direct (LAN) addresses; a test checks that with relays off it has an
  IP address and that an endpoint with no address lookup at all connects with it.
- Menu texts (Host, Find) and README *Online play* mention LAN play.

## Checked
- `net::tests::lan_lobby_by_mdns`: host listing heard by mDNS, joined by bare ID (mDNS lookup), joined by ticket
  from an endpoint without lookups. `lan_listing_fits_mdns`: size and foreign user data ignored.
- Three `hst net` processes on this machine, `--relay off`: `host doubles 2`, `find` (listing in the first Found),
  two `join <id> 1` → all three got `Start`, round trip 1 ms, input delay 2 ticks.

## Not verified / not 1:1
- Phones: the acceptance (two Android phones, one the hotspot, and a laptop) needs the Android build, M9b, which
  the user dropped on 2026-10-12. So no `MulticastLock` (there's no Android target to hold it in) and no iOS Local
  Network permission/entitlement handling. Unblocks if M9b comes back.
- Two separate machines on one LAN/hotspot: only tested on one machine (multicast over loopback/the host's own
  interface). Needs a second machine.
- Fully offline: not tested with no internet route. The DHT client still starts and its Find query runs up to its
  30 s cap, but the LAN list is shown after 3 s regardless; `Dht::client()` failing to bind would still fail the
  whole network start (not seen).
- No separate "short code": the code is the ticket (long text, or QR), which carries the LAN address; on a LAN the
  Find list (or a bare ID) is the no-typing path.
- Not a gameplay port: nothing here touches the sim; the match itself is M2g's lockstep, unchanged.
