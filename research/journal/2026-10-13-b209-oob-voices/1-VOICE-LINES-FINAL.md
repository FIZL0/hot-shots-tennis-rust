# B209 — Out of Bounds mods' voice lines sorted by what they say

**Cause of Anya's bad celebration.** `oob_data.py` filled program 7 (point won) with the profile's "Victory Pose" set
and 9 (set won) with the second takes of the same six hole-result cues. Those six are hole *results*, not all wins:
sg slot (0,29) / (1,9) is the disaster hole ("Hmm, useless rubbish." / "No, plastic, useless rubbish!" for Anya,
"No! That was awful!" for Jasmine). Every OOB mod had it in its celebrations.

**Method.** I can't listen, so every sg cue of all 17 characters (956 cues: the longest note's wave and, for
multi-part cues, all parts joined) was transcribed with faster-whisper small, English (the model was already cached
for `tools/voice_lang.py`; it ran in a scratch venv). The transcripts were laid out per slot across the 17 actors:
each slot says the same thing in every actor's words, so the slot's event can be read off. Transcripts are game
data and stay out of git; the slot → meaning table and the per-character skips are in ../opentennis-mods `notes/oob.md`
"Voice lines".

**Fix (../opentennis-mods c454aea, `tools/rerig/oob_data.py voices()`).**

| program | now | was |
|---|---|---|
| 0, 3 | (0,6) (1,3) grunts | + (0,2) "Okay." |
| 1 good stroke | (0,8) (0,7) (0,9) (0,10) "Hya!", "Go!", "Fly!" | profile Reaction set incl. (0,40) "Aw, shoot!", (0,19) "I messed up." |
| 2 mis-hit | (0,12) (0,14) (1,15) "Oops!", "Whoops!" | empty (donor's disc takes) |
| 4 whiff | (0,38) (0,13) "Huh?" | same |
| 7 point won | (0,28) (0,37) (1,8) (0,27) "Yes!", "It's in!" | Victory Pose set incl. the disaster (0,29) |
| 8 point lost | (0,29) disaster + (0,40) (0,22) (0,18) (0,23) (0,16) "Oh no!" | empty |
| 9 set won | (0,31) (0,33) (1,11) (1,13) (0,30) (1,10) | incl. the disaster (1,9) and the stumbling win (0,32) |
| 10 set lost | (1,9) (0,29) (0,25) (0,24) "Next time!" | empty |

Plus a per-actor `SKIP` for takes that say something else in that actor's script (e.g. Kate's par "It's not good
enough!" out of set won, Anya's "Don't get excited." out of point won); a wave two slots share is used once.
The 17 `out/mods/oob_*` (= `mods/hst-mods/`, a symlink) got their `voice/` and `mod.json` `voice_counts` /
`voice_why` rewritten with `package.voices()` + `oob_data.voices()` (the voice part of `package.py golf()` only, so
the costumes other tasks are rebuilding weren't touched); a full `./build.sh oob` gives the same.

Anya now: 7 "It's in!", "It is nothing.", "That's it!"; 9 "Hmm, it will do.", "Ah, how pleasant.", "Walk softly
and carry a big club. Hehehe." …; her miss line "Hmm, useless rubbish." is program 8 key 0 (point lost) and 10.

## Not verified / not 1:1
- Not listened to by ear: Whisper gives words, not tone. One-word lines and grunts ("Oh!", "Huh?", "Ha!") were
  taken by slot meaning; a sarcastic or pained reading of a happy word wouldn't show. Anya's set should get a human
  listen in a match (point won/lost reactions).
- Out of Bounds has no event table (encrypted program), so which golf event each slot belonged to is inferred from
  the words; which HST program a line fits is a judgement, not a port. Program 6 (partner calls) stays empty: no
  golf line is a call to a partner.
- Not checked in-game (no PCSX2 needed: it's mod data only); the game picks the wavs up by file name, `audio.rs`
  unchanged.
