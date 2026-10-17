# M125 — online mods' looks from the mod store

**Goal.** A guest without a lobby mod's looks used to get them streamed from the owner's device through the host (a
phone sending ~24 MB over the relay: slow, and its frame rate drops). Now the wanting machine tries the store first.

## What changed
- `mods_online::pack` leaves out a store install's `store.json`. Before, a mod installed from the store packed with its
  stamp, so its looks hash could never match a fresh store copy. (This changes the looks hash of store-installed mods;
  peers on the same build agree, and an older peer's hash still works through the owner as before.)
- `modstore::Meta::looks` (16 hex digits, set by `package` for characters and courts): uploads from now on say the
  looks a lobby will name. `meta_of` only accepts 16 hex digits. The index builder copies meta.json verbatim, so it
  carries the field without changes.
- `modstore::fetch_looks(id, looks, stage, tell, done)`: lists the store unmerged (`lists`, 3 requests with the index;
  15 s timeout so a LAN lobby with no internet falls back fast), tries the candidates (`looks_candidates`: the
  entries whose meta names these looks, then at most two old uploads without `looks`, `main` first; never ones naming
  other looks or texture packs), downloads each with `ranged_with` (no upscaled set) into `stage`, packs and hashes
  it, clears `stage`, and hands over the pack if it hashes to `looks`.
- `Net::want(h, id, cache)`: store first (stage `mods-online/.store-<h>/`), its progress as `Event::Progress`, the
  pack as `Event::Pack` (so `got_pack` checks and unpacks it exactly as a peer's: hash, §7 checks); any failure sends
  `Cmd::Want(h)` and the owner streams it as before. The test `Net::wired` never asks the store.
- online.rs: the two places that asked for looks (Always, and the prompt's answer through `Fx::Net(Cmd::Want)`) go
  through `Net::want` with the mod's id (`looks_id`: a pick's wire or the host's court mod).

## Verified
- `looks_candidates_say_so_first`, `pack_unpack_checks_what_arrives` (store.json left out), `package_then_install`
  (meta's looks = pack hash of the source folder = pack hash of the installed copy with its stamp).
- `live_looks` against the real store (`HST_LIVE_LOOKS=opentee2_pc18_lee`): installed as a player would (stamp
  written), hashed 6f8211048edb0e25; `fetch_looks` for that hash returned the same pack from the store (legacy entry
  without `looks`), for another hash an error; stage left empty. 33 s.
- hst `online` tests (39) and hst-base `net::` tests (38) pass.

## Not verified / not 1:1
- No two-machine lobby run with a published mod (needs two devices on the internet); the path is the same
  `Event::Pack` a peer's pack takes, and the store half ran live above.
- Front-end/online feature, not in the original game: nothing to match.
- Uploads before this have no `looks`: for those a guest may download up to two store copies that turn out to be other
  versions before asking the owner (a delay, never a wrong mod: the pack hash decides).
