# B267 — Find: LAN and public lobbies at the same time

Before: `Cmd::Find` slept `LAN_WAIT` (3 s), sent the LAN list, then ran `find_all` (DHT board + short-code slots,
zipped), so the public lobbies came at 3 s + the slower of the two lookups. `Cmd::Quick` did the same wait and one
`find_all`.

Now (`net.rs` `Finding`): the LAN's wait, the DHT board lookup and the slots lookup run at once
(`FuturesUnordered`); each answer sends `Event::Found` with the merged list so far (LAN copy first, then board, then
slots, de-duplicated by ID as `find_all` did; the LAN part is mDNS's current map, so it rides on every list). Native
sends `FINDS` = 3 lists, the browser 1 (slots only; it has no LAN or board, and no longer waits 3 s for nothing).
The menu's spinner counts `net::FINDS`; the empty-but-still-looking line is now "None found yet: still looking...".
Quick tries the fitting lobbies of each list as it comes (each ID once) and hosts only once all lookups answered
with nothing joinable. `find_all` is gone.

The relay bind wasn't touched: since B224 it no longer waits for the home relay, and Find showed `Online.` at 0.05 s
in every run, so starting the DHT before the bind would gain nothing.

## Timing (`hst net find`, same machine as a `hst net host 1`, 3 other lobbies of other agents up)

| build | first list | DHT lists |
|---|---|---|
| old | 3.01 s (LAN) | 9.02 s |
| old | 3.00 s | 11.01 s |
| new | 2.45 s (a slot's: the host's short code `5RBC`, before mDNS had it) | 2.45 s, 5.06 s; LAN 3.05 s |
| new | 1.80 s | 1.80 s, 6.06 s; LAN 3.05 s |
| new | 2.05 s | 2.05 s, 6.01 s; LAN 3.05 s |

Last lobby shown: old 9–11 s, new 5–6 s (the slower DHT lookup). The LAN list's time doesn't depend on the DHT.

## Not verified / not 1:1

- No-internet run not done (the machine stays online for the other agents); the LAN list is its own timer future
  in the set, independent of the DHT lookups, and is sent at `LAN_WAIT` as before.
- Quick not run live: joining the first fitting lobby would have sat in another agent's test lobby. The menu test
  and the shared `Finding` path (same as Find) cover it.
- Front-end only, not in the original: no 1:1 needed.
