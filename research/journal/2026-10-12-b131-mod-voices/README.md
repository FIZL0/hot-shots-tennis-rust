# B131 — mod characters fall back to stock player sounds

## Findings
- In a match the fallback was already there (M1g, `play/mod_match.rs apply`): the player's bank is the donor's disc
  voice bank (`audio::voice_bank` by donor number), and a mod's `voice/` wavs go on top of it with the donor's
  `hd`/`bd` kept, so any program the mod ships no wav for plays the donor's take (`audio::Sound::play`).
  A mod with no `voice/` at all (Get a Grip's Helghast is the only packaged one) keeps the donor bank as is.
- New test `mod_voices_fall_back_to_the_donor` (mod_match.rs): sets up a bot singles match with Helghast (no voice),
  Wendy (full set) and Chika (umpire, 6 takes) and checks every program/key the singles sim can ask for (shouts 1–4,
  hit cry 5, serve voice 6 keys 0–2, reactions 7–10) resolves to a mod wav or a donor disc sequence. Passes.
  (Program 6 keys 3–4 are the doubles CPU call-out, `sound::call_out`; from the doubles bank, same path.)
- The silent case was the character-select card's R2/L2 voice (`main_menu/previews.rs voice`): B105 made a mod that
  has a voice but lacks the R2/L2 take (program 9/10) play nothing. Now it plays the donor's costume-0 menu take,
  as for a mod with no voice at all. `presses_start_voices` updated (L2 on a mod with only 9_0.wav → donor's key 1).

## Not verified / not 1:1
- Mods have no counterpart in the original, so nothing to check against PCSX2; the fallback is the user's call.
- Not listened to in the running app (unattended run): checked by the bank lookups the mixer uses, not by ear.
- The prize-ceremony voice already loads the donor's (`g.chars` is the donor number); no mod wavs there until M8e.
