# B332 — our own lobby never in Find, the Online card or Quickplay

Every list Find sends (`Event::Found`, which also fills the Online screen's open-lobbies card) and every list Quickplay
picks from comes out of `Finding::next` (`crates/hst/src/net.rs`). It now drops the listing whose id is this machine's
endpoint ID (`not_mine`). After hosting and leaving, our DHT board listing, slot listing and mDNS record can linger
(up to `STALE`). Without the filter, Find showed them and Quickplay could dial itself. The browser with relays off
(no endpoint) has no listing of its own, so there is nothing to filter.

Test: `net::tests::find_never_lists_our_own_lobby`.

## Not verified / not 1:1
- Not tried live over the DHT with two machines (needs a second peer); unit test only. No original to compare
  (online play is our addition).
- A lobby left over from an earlier run of the app (different random endpoint key) can't be told apart from anyone
  else's and still shows until it goes stale.
