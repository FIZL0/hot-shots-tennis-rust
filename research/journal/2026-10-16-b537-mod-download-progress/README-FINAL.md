# B537 — online lobby: mod download progress

**Problem.** A seat whose mod this machine was fetching said "Emi mod: downloading its looks..." from the ask until
the pack had fully arrived, with nothing moving in between: the pack stream was read with one `read_to_end` and its
length was never sent, so there was no progress to show. With a big mod (up to 64 MB through the host) it looked stuck.

**Change.**
- `net.rs`: a pack stream is now hash (u64 LE), **length (u64 LE)**, pack. `read_packs` reads it in 64 KiB chunks and
  reports `(hash, bytes so far, length)` once per whole percent; a length over `mods_online::CAP` is stopped, a short
  stream is dropped as before. Guests get `Event::Progress`; the host's per-guest readers send `Guest::Progress`,
  which the host task passes on as `Event::Progress` only when the host itself waits for that mod. Peers must be the
  same build (handshake), so the wire change is safe.
- `main_menu/online.rs`: `NetState::progress` (per wanted hash), filled by `download_progress` from `poll` while the
  mod is still wanted, cleared on arrival and on a new lobby. `remote_mod_note` shows "downloading its looks (42%)"
  once bytes come (lobby seat, select card and confirm all use it).
- Tests: `net::tests::mod_looks_peer_to_peer_and_the_ready_gate` checks the last Progress (whole length) comes before
  the Pack; `remote_mod_asked_on_the_select` checks the note's percentages, an unwanted hash ignored, and the progress
  cleared when the pack arrives (refused here).

## Not verified / not 1:1
- Not compared with the original: online play is ours (no PS2 counterpart); checked by unit/network tests only, not
  in a live two-machine lobby.
- A guest waiting for another guest's mod sees "downloading its looks..." with no percentage while the host is still
  collecting it from the owner (the host only forwards whole packs); the percentage then covers the host → guest hop.
- A download cut off part way still leaves the note at its last percentage (as before it stayed on "..."); no retry.
- The host's court mod download has no seat note, so it shows no progress (unchanged).
