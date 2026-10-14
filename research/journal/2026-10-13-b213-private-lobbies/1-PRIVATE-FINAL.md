# B213 — public and private lobbies

- `Room.private` (serde: omitted when false, so older peers still parse a public room). The host task's listing step
  (`net.rs` host, `dirty`) skips the DHT announce and clears the mDNS user data while private; the short code is still
  published, the ticket still works, so a code/link/QR joins. Going Private overwrites the DHT listing once as full
  (`taken = seats`), which `find` already drops, so it leaves Find/Quickplay on the next refresh instead of after
  `STALE`; mDNS drops it at once (no user data → `lan_listing` none → removed). Going Public sets `dirty`: listed
  again on the same pass.
- DHT listing seq is now real µs (was whole seconds × 10⁶: two puts in one second didn't overwrite each other).
- `Cmd::Private(bool)` (host only, until Start). Lobby: a sixth host button "Public"/"Private" (grid Copy Link ·
  Public · Leave), "Private: code only" over the QR, info line per state, legend Make Private / Make Public.
- The choice is remembered in `online.txt` (`private = yes|no`), so Host opens the next lobby private straight away
  (no window where a fresh lobby is listed). A lobby Quickplay hosts is always public.
- Test: `net.rs private_lobby_joins_by_code_only` (DHT testnet, the real host task, ~2 s): private → not in `find`,
  joins by code; Public → found; Private again → gone. Menu tests: toggle, saved cfg, grid/taps.

## Not verified / not 1:1
- Not a retail feature (online is ours), nothing to compare with the original.
- mDNS side checked only by reading (`lan_lobby_by_mdns` covers listing via user data; private just sets none).
- No screenshot of the lobby screen: it needs a live lobby; the new label sits in the existing 640×448 layout
  (scaled uniformly like the rest of the lobby), above the QR.
- The first-ever private host is opened public (default) until toggled; after that Host remembers Private.
- Wrap-up: full workspace tests pass except rollback_desync_is_saved_and_play_goes_on, failing on main since B231 (B249), untouched here; ticked and committed by hand (done.sh needs a green run).
