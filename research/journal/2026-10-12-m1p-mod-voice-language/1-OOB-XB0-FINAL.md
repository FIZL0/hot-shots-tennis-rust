# M1p — modded characters speaking Japanese

**Cause:** Out of Bounds (PS3) ships each in-game voice archive twice: `sound/pc/ga_pc_NN.xb0` (English, all 17 pcs) and
`ga_pc_NN.xb` (pc00-14; Japanese cast for pc01-06 and 08-14, English for pc00 and 07). open-tennis-mods
`tools/rerig/oob_data.py _bank` globbed `ga_pc_NN.xb*` and took the first hit, so 13 OOB mods played Japanese lines.
Not the loader: `mods.rs` only plays the mod's own `voice/` folder (no fallback set).

**How found:** faster-whisper (small, CPU) language ID over the four longest voice wavs of each of the 87 packaged mods.
Before: OOB 42 of 69 lines `ja` at 0.7-1.0 ("僕が一番うまいんだ!", "やったー!"); every other game only a handful of
low-confidence grunts (えぇ?, おいしょ, 做). `.xb` vs `.xb0` of pc01: ja 0.98-1.00 vs en 0.98-0.99.

**Fix (open-tennis-mods 8a83653):** `_bank` reads `.xb0`. Its cue names follow the script numbering the slot table was written
from (Victory 49A/49B/50A/51A, Reaction 30/48/60A/39), so the slot mapping is unchanged. Repackaged
(`package.py oob`); recheck: 64 of 68 `en`, the other 4 wordless (la-la, grunts) at ≤0.8. `notes/audio.md` *Language*
lists each game's English copy; `tools/voice_lang.py` reruns the check.

**Other games:** Open Tee 2 already used the English `.xb1` (`_US` wave names). Fore, Get a Grip and Open Tee have one
voice copy each, English. No game is Japanese-only for the packaged lines, so no mod note was needed.

## Not verified / not 1:1
- Not heard in HST itself: checked on the packaged wavs by speech recognition, not by ear in a match (no human in the run).
- Only each mod's four longest lines were language-checked; short shouts can't be classified by language ID.
- `tools/voices.py`'s `out/voices/oob/pcNN/` alias folders still mix both copies (not used for packaging).
