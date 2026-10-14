# B181 — mod characters never use their donor's voice

Every character voice the remaster plays goes through one of three banks:

- **Match** (`VoiceBanks`, slots 1..4: stroke/whiff/dive shouts, serve, point reactions, mate calls, emotes,
  `reaction_voices.rs`): `mod_match::apply` now sets a mod player's bank to the mod's own `voice/` takes only
  (`mods::voice`), `None` if it ships none. No donor `.hd/.bd` is attached any more, so a program it lacks falls
  through `audio::play_as` to an empty header → silence; a `None` bank is skipped in `play.rs`'s sound system.
  The sim side (`g.voices[i].shout/react`, RNG draws) is untouched, so matches stay bit-exact.
- **Select-screen R2/L2** (`main_menu/previews.rs::voice`): a mod's own program 9/10 take, else `None`.
- **Prize ceremony** (`match_stats/ceremony.rs::begin`): the shown player's disc prize bank isn't loaded when it is a
  mod character.

Costume packs (`costume_of`) *are* their disc character and keep its voice everywhere.

Tests: `mod_match::mod_voices_never_fall_back_to_the_donor` (Helghast, no voice → no bank; Wendy, Chika → only
their own programs, no donor bank), `previews::presses_start_voices` (mod without a take → silent; costume pack →
disc character's take).

## Not verified / not 1:1
- Not checked by ear in a running match (unattended run); the test checks the banks the match plays from.
- Nothing to compare in the original: mods don't exist there.
