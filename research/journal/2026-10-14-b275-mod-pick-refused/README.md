# B275: a guest left with "bad params override" over a host's COM mod pick

## Cause
`Wire::check` allowed override values made only of digits and ` ./-+`, while `mods::read` took any string. The
mods repo's Get a Grip / Out of Bounds mods (e.g. `getagrip_pc00_emi`) override `タイプ` (play style) with text:
`オール`, `ネット`, `ベース`, `ビッグ`. They load locally, so the host could seat one, but every guest refused the room
and left.

## Fix
- `mods::override_ok(k, v)` is the one rule, used by `read` and `Wire::check`: a TParam.csv column, value up to
  64 characters, no comma or control character (the row is re-joined with commas for `ReachStats::from_tparam`).
  A mod that breaks the rule is refused at read time. All 1,944 overrides in `../opentennis-mods/out/mods` pass.
  Column names come from one `COLUMNS` constant shared by the web and native builds, and peers already have to
  run the same `BUILD` to join, so builds agree on them.
- The error now names the key and the value (`mod "x": params.override `nope` is not a TParam.csv column`).
- A guest no longer leaves over a bad pick in a lobby `Room`. It shows "A pick this game can't play: …" and stays
  seated. Only a bad `Start` still ends the link, because that match can't be played. The host checks its own
  picks and COM picks (`Cmd::Picks`, `Cmd::Bot`) before seating them ("Pick refused: …"). A guest's refused pick
  gives the host "<name>'s pick was refused: …" and sends the guest a chat line "Your pick was refused: …".
- Test: `net.rs a_refused_mod_pick_keeps_the_guest` (loopback). `mods.rs checks_the_manifest` and
  `mods_online.rs wire_round_trips…` cover the text cell and the comma.

## Rejoin by code: "No open lobby has the code"
The host republishes its code every 30 s for as long as the lobby is open. Nothing removes the record when a guest
leaves. Two problems found:
1. `relay.pkarr.org` answers GETs with `cache-control: public, max-age=300` (checked with curl). The browser's
   `relay_get` goes through fetch, so a second lookup within 5 min (a rejoin) could get the cached copy. That copy is
   older than `STALE` (90 s) by then, so `fresh()` rejects it and the lookup reports no lobby. Now every lookup adds
   `?t=<µs>`, which the relay accepts (200, same payload) and which needs no CORS preflight.
2. The relay sends `x-ratelimit-limit: 10`, and the host was re-putting the code (DHT and relay) on every lobby
   change (join, pick, ready…). Now the code goes out only on the 30 s tick. The listing and board slot still go out
   on every change.

## Not verified / not 1:1
- The user's rejoin wasn't reproduced end to end across two machines (needs a browser and a native peer on the live
  relay). The cause is inferred from the relay's headers. If a native joiner (DHT, not the relay) also misses the
  code, the cause is something else: still open.
- An older build (before this fix) still refuses text overrides and drops out, but `BUILD` must match to join, so the
  two never meet in one lobby.
- Gameplay isn't affected (lobby/network only). Nothing to check against the original.
