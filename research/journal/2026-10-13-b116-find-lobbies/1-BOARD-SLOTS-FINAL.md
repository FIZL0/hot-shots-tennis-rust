# B116 — Find shows no lobbies

## What was wrong
- Native Find works on this machine. With a `hst net host` running:
  - the LAN (mDNS) list came at 3 s;
  - a DHT-only Find (LAN skipped by a temporary patch) found the lobby on the public mainline DHT in 4.0 s, twice.
- The browser build (opentennis.dev) had no lobby list at all:
  - `find_all` returned `[]` on wasm, so Find in the browser always said "No open lobbies found";
  - a lobby hosted in the browser was never listed anywhere, so native Find couldn't see it either.
- The browser can't use the DHT board. Find there is `get_peers(TOPIC)`, which needs UDP. The pkarr relay only
  gets and puts single BEP 44 items by key.

## Fix: board slots (`net.rs`)
- There are `SLOTS` = 8 well-known keys, `slot0`…`slot7`. Their seeds are public, made the same way as the short
  codes' (`code_key`).
- Each slot holds one listing: the `Listing` JSON plus the new `code` field, the lobby's short code, as a TXT
  record. Reads and writes use the existing short-code `lookup`/`publish`:
  - native: the DHT, plus a copy put on the relay;
  - browser: the pkarr relay over HTTP.
- A host lists itself once its short code is rolled, then on every change and every 30 s (`list_in_slot`).
  - It keeps the slot it remembers if that slot is still its own.
  - Otherwise it takes the first slot that is free, stale (more than 90 s old) or its own, starting from the slot
    its ID picks.
- Find reads every slot (`find_slots`).
  - Native: LAN, then the DHT board, then the slots. A lobby already in an earlier list is skipped.
  - Browser: the slots only.
- A listing that came only from a slot (a browser host has no DHT or mDNS address) is joined by its short code.
  This is `Listing::dial`, used by the Lobbies screen and Quickplay.
- Why 8: the relay rate-limits at about 10 requests at once. Probed: 16 parallel GETs gave six 429s. A slot nobody
  wrote takes about 7 s to answer (404).
- The tests write to `tslot*` names. Before that change, a testnet run put fake listings on the real board for
  90 s, because a testnet's items reached the real DHT and relay.

## Verified (`context/b116/`)
- `slots_on_a_testnet`:
  - two hosts take different slots, and one keeps its slot on republish;
  - a third node finds both codes;
  - with the board full, a ninth host isn't listed.
- `slot_through_the_pkarr_relay` (ignored, needs the internet): a listing with the longest name goes through
  `relay.pkarr.org` and back the way the browser reads and writes it. Passed.
- A native `hst net host` listed in a real slot was read back through the relay (the browser's read path) in
  0.39 s.
- run2:
  - `hst net host singles 1 --bots`;
  - `hst net find` listed it at 3 s (LAN) and 11 s (DHT + slots);
  - `hst net join <its short code>` reached Room and then Start on both sides.
- `wasm32-unknown-unknown` `cargo check` is clean (only warnings that were already there).
- `tools/check.sh -p hst --bin hst -- net:: main_menu`: 68 passed.

## Not verified / not 1:1
- **Not tested on two machines or networks, and not tested in a real browser tab.** No second machine or browser
  automation was available here. The browser's read and write path (`relay_get`/`relay_put`) was run natively
  against the real relay instead. PLAN *Needs the human* already has the two-network and native/browser crossplay
  checks (M2f, B195/B201); they now cover Find too.
- **Browser hosts appear late.** A browser host shows up only after its relay put lands, which takes 5–20 s
  (B151). That can be just over the ~15 s target.
- **Only 8 lobbies in the board slots.** Native Find still lists any number through the DHT board.
- **Slot races.** A slot that didn't answer (timeout or 429) counts as free, and two hosts claiming the same slot
  at once both write it. The next republish's check moves the loser to another slot. Both have `ponytail:` notes.
- **Unsigned slot listings.** Unlike the DHT board's, slot listings are not signed by the endpoint. A fake one only
  leads to whatever lobby its code names, and the code's ticket is still dialled and authenticated by QUIC.
- **Started lobbies stay listed briefly.** A started or left lobby stays listed with open seats until it goes
  stale (90 s), the same as on the DHT board.
- The original has no online mode, so there is nothing to compare 1:1.
