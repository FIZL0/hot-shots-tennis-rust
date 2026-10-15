# B386 — match celebration music silent (FINAL)

## Cause
The ceremony loaded the result music (`bgmm_10`, B53) only with `--music`, the match-music switch. That switch
defaults to off (user 2026-10-12) and only means the court's BGM, so by default the ceremony and result page were
silent apart from the jingle, voice and gallery.

## Fix
`play/match_stats/ceremony.rs` `begin` always loads `bgmm_10`; `tick` starts it at ceremony tick 44 as before (logs
"result music starts"). It is in the Music volume group, so the Music slider still scales it. The court BGM switch is
untouched.

## Loss / annihilation sounds
The banner row (annihilation/victory/narrow) changes no sound in the original or the port; a loss differs by the
jingle (key 4 vs 3), the prize voice's program (1 vs 0) and the gallery's groan (event 4 → programs 4, 8) instead of
the ovation (3 → 3, 7, 7/1) and the four cheers. New test `audio::tests::match_end_sounds_are_heard` renders each of
these from the disc and asserts it is audible: `bgmm_10`, jingles 3 and 4, every character's prize voice (A and B
sets, programs 0/1, both lines) and every court's gallery banks' ovation/groan programs. All pass.

Run: `HST_STATS=6 target/release/hst <iso> --play --stage 10 --pads -,-,-,-` (no `--music`): the log shows the
ceremony, "result music starts", the voice, then the result screen (`context/b386/run1/log.txt`).

## Not verified / not 1:1
- Not listened to: no audio capture here; checked by the render test (peaks) and the run's log.
- A live loss ceremony wasn't run (needs a human side beaten; the unattended `HST_STATS` run ends a bot match, which
  always shows the winner). The loss path's choice of jingle 4 / program 1 / groan is the existing P26c port; this task
  only checked those sounds load and are heard.
- Playing `bgmm_10` with the match music off is ours (user request); the original has no such switch.
