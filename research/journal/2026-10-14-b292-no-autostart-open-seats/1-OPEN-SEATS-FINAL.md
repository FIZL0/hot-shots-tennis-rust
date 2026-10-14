# B292 — no lobby autostart with open seats

`Room::all_set` (net.rs), which starts B215's self-running countdown once every person in is ready, now also needs
`open() == 0`. With seats open, the match starts only on the host's Start (the open seats go to the computer as
before); a full lobby with everyone ready still counts down by itself. The menu hint for guests says so.

Tests: `no_autostart_with_open_seats` (Room unit test); `short_code_on_a_testnet` (ignored, doubles with two open
seats) now checks that the guest's ready starts no countdown and that the host's Start does; `ready_up_countdown`
(singles, full) unchanged.

Headless `hst net` (no menus) keeps its own rule (host starts when full, or with `bots` once a guest is in).

## Not verified / not 1:1
- None against the original: the online lobby is ours (front-end, no original counterpart).
