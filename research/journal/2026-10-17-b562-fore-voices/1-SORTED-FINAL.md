# B562 — Fore! mods' voice lines sorted by what they say

**Cause.** `fore_data.py voices()` filled the stroke shouts 0/1/3 with `jy3`/`jy4` and the bad-shot programs 2/4
with `jy0-2`, by the order the five shot reactions sit in the swing archive (ga/os/bw upset, lp/tp happy). The
transcripts show that order means nothing: `jy0/jy1` are the power / good-shot exclamations, `jy2` the groans,
`jy3` pleas to a rolling ball, many of them bad ("Failure is imminent.", "It's tanking!", "This is no good!"), `jy4`
a mixed verdict ("I don't like golf!"). Point lost / set lost 8/10 were the bogey bank, which is often a joke or a
laugh (Hubert's "Ha ha ha! What a total mind blow!").

**Method.** As B209: every Fore! cue (1778 waves, `out/voices/fore/`) transcribed with faster-whisper small,
English, beam 5 (scratch venv; the model was cached). Output in `context/fore-voices/`: `fore.txt` (the jy/g banks the
old packager used), `fore_all.txt` (all banks), `picks.txt` (each mod's programs with the words and seconds of each
take). The banks' meanings, read across the 24 actors: `co` comments on a shot, `ya` heckles/praise for the
opponent, `sc` one line per opponent, `vs` versus results (roughly 00-03 a win, 04-07 a loss), `jy5/6`
encouragement for the other player after a wait.

**Fix (../opentennis-mods bc47dc1).** `fore_data.py PICKS`, per character, by meaning:

| program | now |
|---|---|
| 0 / 1 / 3 stroke shouts | short exclamations only, ≤ ~1 s: jy0/jy1 and one-word co/ya/sc lines ("Booyah!", "Count it!", "BALL CRUSHER!", "Ha!") |
| 2 mis-hit, 4 whiff | short bad-shot / surprised lines (jy2, "Oops.", "What?", "NUTS!") |
| 7 point won | happy par/birdie g lines and boasts |
| 8 point lost | laments only (groans, "Not my day!", the bogey line only when it is a lament) |
| 9 set won | happy eagle/birdie g lines, winning vs lines |
| 10 set lost | losing vs lines and laments |

5 and 6 stay empty (notes/fore.md). Hubert now: strokes "Tinker!"/"Blankers!" (0.4/0.7 s), mis-hit "UNFORTUNATE",
"Failure is imminent." is a point-lost line; his laughing bogey is in nothing. The 24 `out/mods/fore_*` got their
`voice/` and `mod.json` `voice_counts`/`voice_why` rewritten with `package.voices()` + the new `fore_data.voices()`
(the voice part of `package.py golf()` only, as B209), and `version` bumped (2 → 3; Z had none → 2).
`notes/fore.md` *Voices* rewritten.

## Not verified / not 1:1
- Not listened to: Whisper gives words, not tone. Several short jy0/jy1 takes are nonsense words to Whisper
  (Hubert "Tinker!"/"Blankers!", Sam "SUMMER EYE!", Louise "Washing machine!", Mel's food words); they were taken as
  exclamations by their slot and length, a pained reading wouldn't show. Worth a listen in a match.
- Which HST program a golf line fits is a judgement, not a port: Fore! has no tennis events and no effort shouts.
- Not re-uploaded to the mod store here (unattended run, outward-facing): the bumped versions go up with the next
  upload, and installed copies refresh then.
- Not checked in-game (mod data only; audio.rs unchanged, file names as before).
