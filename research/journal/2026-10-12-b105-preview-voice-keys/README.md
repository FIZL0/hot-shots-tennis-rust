# B105 — character-select preview voices: R2/L2 keys swapped

## Finding
The inspect handler of Data → Items → Characters (menu overlay, the function that also cycles costumes on 0x400/0x800)
plays bank slot 0x16 program 0 with key 1 on pad bit **0x100** and key 0 on **0x200**. In the game's pad word
(journal 2026-10-06-p0-input-replay) 0x100 is **L2** and 0x200 is **R2**; the same handler's 0x400 / 0x800 = L1 / R1
costume step confirms the layout. B86a had read 0x100 as R2, so R2 played key 1 (the upset take) and L2 key 0 (the
happy one), as the user heard.

## Port (`play/main_menu/previews.rs` `voice`)
- Disc characters: `key = (act == Act::Loss)` → R2 key 0, L2 key 1. Mods with no voice use the donor's bank through
  the same path, so they follow.
- A mod that ships a voice but lacks the program for a button (9 = R2, 10 = L2) plays nothing for it, instead of the
  donor's take (another person's voice). M8e packages the missing loss takes.
- Doc comment fixed. `presses_start_voices` pins disc Win = key 0, Loss = key 1; mod with voice and no L2 take → None;
  mod without voice → donor's key 0 / 1.

## Not verified / not 1:1
- The key ↔ button mapping is from the decompile (handler bits + the known pad layout), not heard live on the
  original's Data → Items screen: driving PCSX2 into that screen wasn't needed once the bits matched the documented
  pad word and the user's report.
- Which motion each button starts (handler's motion arg 2 on L2, 1 on R2) wasn't re-checked; the user only reported the
  voices swapped, motions untouched.
