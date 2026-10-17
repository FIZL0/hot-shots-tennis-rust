# M126 — online rejoin after a browser refresh or crash

## What changed
- `crates/hst-base/src/net/rejoin.rs` (new): a guest's match record `code seat unix-seconds` — the tab's
  sessionStorage (`hst-rejoin`) in the browser, `rejoin.txt` beside settings.txt natively. Read once (`take`), ignored
  after `FRESH` = 10 min, `forget` when the match is over.
- `main_menu/online.rs`: `remember` on every `Event::Start` (a guest's code = the joined code `paste`, else the
  host's short code; the host and browser-LAN offers write none, they forget). `linked` (fresh menu) now also takes a
  remembered record when no `?code=`/`HST_CODE` is given and goes straight to joining it (one press on the Assign
  screen, as a lobby link already did) — the "rejoin straight away" option; status says "Rejoining your match".
  Not taken in `--shot` runs. Failed / Dropped / Lobby events forget the record.
- `net.rs`: `Msg::Hello` carries `seat: Option<u8>` (serde-default, skipped when None), set by `Cmd::Seat` before the
  Join. The relay's rejoiner choice is `back_for`: the away peer in the claimed seat, then by name (two players can
  both be "Sam"), then the first away. Unit test `a_rejoiner_gets_its_own_seat_back`.
- `recover.rs`: `WAIT` 20 → 120 s (tests keep 20 s under `cfg(test)` so the netplay match tests don't play 100 s
  more of frames); the countdown reads "Waiting for X to come back or rejoin... 1:58" (m:ss). The match's end
  (result screen or any `finish`) forgets the record once.
- A refreshed host still ends the match: the guests' connections and the relay are the host's; no host migration
  (noted in recover.rs's module doc). `web/index.html` needed no change: a reload in the same tab already goes
  straight back into the game (`hst-playing` in sessionStorage), and the disc pack loads from OPFS.

## Tests
- hst-base `rejoin` (3), hst-play `the_wait_says_minutes`, `HST_NETPLAY=1` hst-play netplay (30),
  `a_silent_peer_rejoins_and_a_silent_host_ends_it` (ignored one), hst `online` (39), wasm32 `cargo check -p hst`.

## Not verified / not 1:1
- No end-to-end run of a real browser refresh (or a killed native guest) rejoining a live host: needs two machines /
  browsers and a relay; the pieces (record, seat claim, relay choice, existing rejoin restart test) are tested apart.
- The 120 s wait and the "or rejoin" wording are ours (the original has no online play); menu/presentation, not
  gameplay, so nothing to match.
- One press is still needed on the Assign screen before rejoining (it picks the device that plays); not skipped
  because which device it should be isn't known after a restart.
