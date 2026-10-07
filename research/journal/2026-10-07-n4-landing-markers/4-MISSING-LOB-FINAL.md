# N4b — missing lob (smash) marker

## Cause
The port searched every receiving human against **character 0's** smash heights (TParam col 64: top 2.65, middle
2.25), but players are seated as other characters (default line-up 6, 1, 2, 3; `--chars`). The original points each
candidate at its own character's table entry: the match manager's marker object keeps a per-slot character id
(`-1` for CPU slots, set from the player's controller index < 0x20) and the search reads that entry's
`+0xe8`/`+0xec` (N4a's capture: P1 = character 6, 2.6/2.2 — TParam's 260/220/180). So a lob peaking between the
character's own top and character 0's (P1 Carol: 2.60–2.65 m; much wider for 3, 7, 11 at 3.0 m+ or 5 at 2.5 m)
never armed the search and lost its marker; the placed point also moved with the middle height.

## Fixed
- `play.rs`: `smash_heights(iso, c)` per seated player (TParam col 64 top / middle) replaces the character-0
  `smash_mid`; `strike` uses each human's own.
- The original sets the search up only `players == 1 || shots > 1` (same gate as the red marker's match case;
  the strike message clears it first), so the serve no longer searches.
- `effects.rs`/`effect.rs`: `Effect::hold`. The original's clocks clamp at the end and the marker is drawn until
  the live ball bounces; `smash_p`'s morph is 105 frames but its blob alpha fades over 120, so the port cut it at
  ~0.47 alpha 15 frames early on long lobs. Test `tests/effect.rs smash_mark_holds`.

## Checked
- `tools/check.sh`: 113 tests pass. App launches (`--play --stage 1 --shot`), setup loads per-character heights.
- Not re-captured on PCSX2: the per-character read is direct in the decompile and N4a's log already showed the
  table entry for character 6.
