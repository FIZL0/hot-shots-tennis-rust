# P12a — post-point sequence (port: crates/hst-sim/src/flow.rs, wired in crates/hst/src/play.rs)

## What was ported
- Umpire call sprite (scoreboard show kind 5) before the point's pause: `PostPoint::called(event, call, timing)`. Judge calls 1 out, 2 fault, 3 double fault, 4 let, 5 out after net show it; 0/6 don't. It settles when her call line ends (`voice_idle`) or its countdown runs out (GAME.BIN table at 0x410f0c, row = language·0x14 + umpire·0x28 bytes; read by `exe::Game::scoreboard_timing(language, umpire)`, field `call_wait`), then holds 29 ticks past settle (hold counter > 0x1d), fades 6. Out, out after net and double fault then chain into the score show with a 1-tick pause (scoreboard +0x188) — faults are cleared when that pause ends; fault and let end the phase (no point) → serve again. This replaces the app's fixed 90-tick no-point delay (Phase::Over removed).
- Human skip: once the new score has settled (show +0x158 / +0x17f busy flag cleared by the show), a face-button press by a human ends the point-over phase on that tick. Earlier presses do nothing. Found from new_recording.bin (presses at 55/67 ignored, 125 ends) and the phase-4 handler (0x326270: the gm+0xd4 object's +0x52/+0x53 path, active with humans). App: `post_press` set in `control`, consumed in `simulate`.
- HUD score moment: the HUD keeps the score from before the point until the score show starts rolling (`PostPoint::shown`, `before`); a game/set shows the new score from the show's start. PCSX2 screenshots (slot 5, plain point, context/p12a/point_t*.png and sheet.png; ticks are gm+0x58 read right after F8, hyprctl adds ~9 ticks per shot so they are not one-frame steps): t31 point-end camera with "Smash Ace", t40 cut-away fade, t52 cut-away, t61 scoreboard rolling 0→15, t70+ settled 15/0, held through t140. The "□ Replay" prompt appears from t61 (instant replay offer, P0b4d).

## Verified (tests/score.rs)
- `match_s05_post_point`: all 35 point-over phases of the slot-5 bot match (except the match-over point and its replay) incl. 10 called ones; faults, points, games equal every game tick; next phase (serve/change ends) asked on the game's tick; 2 change-ends phases of 80. The call line's end isn't recorded, so for called phases the test searches the tick it ends (1..countdown) and requires one to reproduce the whole timeline: found fault 43/44, out after net 36/37.
- `human_post_point`: all 8 point-over phases of new_recording.bin (P1 human), presses as recorded; one called out (line end 36). Mutation check: letting a press end the phase as soon as the show starts fails at the first phase.
- App smoke run (HST_AUTOPLAY, stage 1): faults return to the serve via the call show, called/uncalled points continue.

## Not ported (ponytail)
- Player-reaction-dependent wait (scoreboard +0x468 = min of table 0x410ed0 [7,20,7,12,32] over players already in a team reaction): never differs from 20 in any recording (players are idle when told to react).
- The call sprite's own animation length (assumed shorter than the voice line). Without audio, the call settles on its first tick.
- Match-over phase, instant replay (P0b4d), point-end cut-away cameras (P16a), round1.bin (no rally block recorded; its called phases can't be judged).
- Reaction choice/root were already ported and tested (tests/motion.rs).
