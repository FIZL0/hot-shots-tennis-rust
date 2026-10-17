# B551 — rejoining a lobby you just left says "lobby full"

## Cause
"Full" is the host's admit in `host()` (`Guest::In` → `room.take` → "not enough open seats (0 open, you bring 1)").
A seat is freed only on `Guest::Gone(peer)`, sent when that guest's connection's stream ends. Two gaps:

1. If the host hadn't yet seen the old connection end (the guest's QUIC close lost or late, e.g. over a relay; the
   host then waits for the idle timeout), the same machine's new connection found its own old seat still taken and
   was refused as full.
2. `Gone` carried only the peer ID, so an old connection's late end also took a rejoined peer's new seat away (the
   same in the match relay's B196 rejoin path).

## Fix (`crates/hst-base/src/net.rs`)
- `Guest::Gone(peer, Link)`: a connection's end frees seats only if it is still that peer's current connection
  (`Link::same`, the link's channel identity). Same in `relay`.
- `Guest::In` from a peer already joined (iroh endpoint IDs are authenticated, so only the same machine): the old
  connection is closed, `Room::reclaim` frees its seats (everything of it, as `leave`) and the admit takes those
  seats back first (`Room::take_from`), with the new Hello's picks.
- The listing follows the room (`dirty`) as before; no change in online.rs was needed.

## Test
`net::tests::a_guest_who_left_joins_again` (loopback, ~1 s): join, Leave, join again → seated; a third connection
while the host still holds the second → seated on the same seat with its new pick; the second's end afterwards frees
nothing. Without the `Guest::In` change it fails with `Refused: not enough open seats (0 open, you bring 1)`.
Also ran `net::tests` (36) and the ignored `a_silent_peer_rejoins_and_a_silent_host_ends_it`.

## Not verified / not 1:1
- The user's actual session had no log, so which of the two (close lost vs. a fast rejoin) hit them isn't known;
  the fix covers both. Not tried across two real machines over a relay.
- Online lobbies are ours (not in the original game): nothing to compare with PCSX2.
