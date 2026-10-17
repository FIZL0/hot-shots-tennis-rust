# B556 — online desync at tick 3960, markers gone after the resync

Report: `context/reports/2026-10-17/desync-1792209661-host.json` (4-player browser match, rollback, host side).
Line-up: Anya (mod `oob_pc12_anya`) on seat 0, three disc characters, court 3.

## What diverged
A peer's screenshot in the report folder shows that peer's download of Anya's looks failing (size mismatch), so it played
her as the M2i stand-in (her gameplay wire over the donor's disc clips). The host had her own clips (`visual`).
Two things in the hash depended on those looks:

- **Cut-away / replay cameras** (B502/B366) frame the *drawn* clip's bones (`cutaway::bone`, `drawn_time`) and write
  `g.cam`, which is hashed. The camera also chooses who is in view, and `reaction_voices` draws `g.rng.shared` once per player
  in view. So this was a real divergence in gameplay RNG, not just a hash mismatch.
- **`Motion::drawn` / `drawn_prev`** tick by the drawn clip's length. They are hashed but are used only for drawing.

Fix:
- An online mod (`mod_match::load`, wire skeleton present) sets `CharacterData::frame_gameplay`. Cameras then frame
  the gameplay clips (`CharacterData::framed`, `drawn_time` on the gameplay clock), which every peer has.
- `snapshot::state_text` hashes each motion with its drawn clock set to the gameplay clock.
- Offline nothing changes (`frame_gameplay` is false, and the drawn clock was already the gameplay one for disc characters).

Test: `netplay::tests::a_mod_plays_alike_with_or_without_its_looks` (`app_in` with the report's line-up, court and
mod). It runs the same mashed match twice, once with the looks and once with the stand-in, and compares `state_text`
every tick for 4000 ticks (`HST_LOOKS_TICKS` sets the count). Before the fix it failed at tick 591 on `g.cam.view`; it now passes in about 5 s.

## Markers after the resync
`effects::LandingMarks` (red hit mark, yellow lob/smash mark) and `popups::Calls` (call models) carry
entity/material handles, but they had no `Wire::local` hook. A resync then wrote the host's entity ids and an empty
material list over the peer's own, so nothing was drawn. Both now keep this machine's views on decode. Test:
`snapshot::tests::a_resync_keeps_the_local_marker_views` fails without the fix (materials 5/7 → 0).

## Not verified / not 1:1
- No peer-side state text: the report is the host's only, so I couldn't diff the two machines at 3960. The cause is inferred
  from the failed looks download in the screenshot plus the looks-vs-stand-in test, which reproduces a divergence of
  that kind. It may not be the exact first differing field at 3960.
- `replays_a_desync` doesn't reproduce the host's hash: the test app doesn't replay main.rs's seed/weather draw, and the
  hash's content changed with this fix anyway.
- Online mod characters' cut-away / replay framing now follows the gameplay clip. If a mod's own clip moves
  differently from the donor's, the camera may sit slightly off its body (presentation only, offline unchanged). Not
  checked by eye.
- Not tested in a real browser match with a failed looks download. The marker fix is tested with an encode/decode on one
  world, not with a live rollback resync between two machines.
- PCSX2 not used (netplay only, nothing to compare with the original).
