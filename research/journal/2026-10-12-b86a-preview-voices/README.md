# B86a — voices for the character-select preview presses

The original's character select draws 2D portraits only, so it has no R2/L2. Our 3D cards copy the inspect on
**Data → Items → Characters / Costumes**, so the voices come from that screen's handler in the menu overlay. It can be
read with `research/fn.sh <addr> context/decomp/hst_menu.c`. `xref.py` only knows hst_game.c.

## What the original does (decompile, confirmed live where marked)
- **R2 (win motion 0x2e):** loads the character's interview bank into bank slot 0x16: `SND/VOICE/MENU/ITGnn_N.XB0`
  (hd `itgnn_n.hd`), or `ITGnn_C` on the Costumes tab (costume index ≥ 5). Then it plays program 0 key 1 at
  volume 0x80, in the same frame the motion is set. It reloads only when the bank id (char + 0x10·Costumes tab)
  changes.
  - **Live, PCSX2 copy 1:** the Characters tab on Ashley loaded `itg00_n.hd`. The Costumes tab on Ashley Retro
    Americana loaded `itg00_c.hd`, and the screen's bank id was 0x10. The strings were found in an EE RAM dump from a
    save state; the IOP doesn't keep the name.
- **L2 (loss motion 0x2f):** the same, with key 0.
- **The gate:** R2, L2 and △ are all ignored while the slot-0x16 voice is still sounding. They are also ignored until
  the model is ready.
- **△ (the "Voice Sample" pill):**
  - Plays key = character index of `mvc_it0` (slot 0x13, Characters tab) or `mvc_it1` (slot 0x14, Costumes tab),
    program 0, volume 0x80.
  - The sample is skipped if it is still playing. The pose motion resets either way.
  - The banks are preloaded with the screen. **Live:** the screen's three sample structs map to slots 0x13, 0x14 and
    0x15, all loaded. `mvc_it0` and `mvc_it1` have 14 keys each.
  - `mvc_it2` (5 keys) is the Umpires tab's bank, which our cards don't use.

## Port
`previews.rs`:
- `voice()` picks the bank and play. Banks load lazily from the ISO and stay cached, as the original never unloads
  slot 0x16.
- `buttons` drops a press while the card's R2/L2 voice sounds, then starts the voice with the motion.
- Mods (standard §5):
  - R2 and L2 use the mod's own `voice/` program 9 and 10, key 0. These are the match's reaction voices for the same
    motions 0x2e and 0x2f.
  - If the mod has no such take, R2 and L2 use the donor's `itgNN_n`.
  - △ uses the donor's sample only when the mod ships no voice at all.
- Test: `previews::tests::presses_start_voices` covers:
  - a disc character's R2, L2 and △ on the Characters and Costumes tabs;
  - a mod with its own R2 take, falling back to the donor for L2, with no sample;
  - a mod without a voice, using the donor for everything.

  Each case asserts the bank, slot, program and key, and that `Sound::playing` is true.

## Not verified / not 1:1
- **The keys** (R2 key 1, L2 key 0, △ key = character) are from the decompile only. Our SPU/IOP side wasn't probed,
  so the take that sounded wasn't matched by ear or by RAM.
- **Bank load latency:** the original loads the ITG bank from disc on the first press, and how long that takes wasn't
  measured. We start the voice in the same frame, with a synchronous ISO read on first use.
- **The gate is per card, not per screen:** the original has one inspect per screen. We have up to four cards, so
  each card gates on its own voice.
- **Mods:**
  - The original has nothing for mods, so the program 9/10 mapping and the donor fallback are our choice.
  - A mod with its own voice gets no △ sample, because the standard has no sample take. A `voice/` program for it
    would need an addition to the standard.
- **Volume:** the playback level uses `play_centre` (bank volume of slot 0x16 / 0x13 / 0x14 from the mixer's table).
  That level wasn't compared with the original's output.
