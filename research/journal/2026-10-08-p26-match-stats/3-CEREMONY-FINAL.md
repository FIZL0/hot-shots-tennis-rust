# P26c — the winners' ceremony, left stick, COM rank, the ceremony's sound bits

## What the original does (result object, state 0)

Recorded with `research/p26c_ceremony.py 5 context/p26c/run2 …` (slot 5, the bot-only doubles match on court 10,
team 0's games poked to one short of the set): result state, shown team, banner rows, every player's mode and
the sound generator's index each vsync, a screenshot every 12 frames (`context/p26c/run2/`, sheet `run2_sheet.png`).
State 0 lasted ~318 vsyncs (the hand-slap camera clip, 319 frames), rows 4/5 (NARROW WIN), team 0 shown.

From the decompile of the match-end setup and the result object:

- **Shown side**: the human side if exactly one side has a human, else the winner. Lost = shown ≠ winner.
- **Camera**: `AZUMA/C_EFF/EFFCT.XB0` `prize/CAM/{win_99, lost_99, hand_99}.CAM` (lost → lost_99, else doubles →
  hand_99, singles → win_99). The `.CAM` is an ANI2 with root tracks only and a fourth key list (4-byte FOV
  values): `hst_data::ani::parse_cam`. Every frame is keyed. Turned π about y (x, z negated) when shown == 0 ?
  ends swapped : not swapped. Offset (0, y, z of the shown player's mark); y = −1.3 when lost, −0.4 in doubles
  when one of the winners is character 7 or 11 (only when both or neither side has a human).
- **Marks**: (0, 0, −6.4) singles, (0.75, 0, −6.4) doubles; the far side turned with z negated, one of each pair's
  x mirrored. Lefties mirrored as in the match.
- **Motions**: `AZUMA/PRIZE/ANI/WINANI.XB` / `LOSTANI.XB`: win_NN (loops back to frame 10), hand_XX{l,r}… (doubles
  winners first, then win), lost_NN{a,b} (holds the last frame); per-character table of hand files and a/b.
  The recording's players 0/2 switch hand → win ~89 vsyncs in, when the sound generator's index ticks once.
- **Banner**: rating = winner's stats row 8; < 65 → rows 4/5, < 80 → 2/3, else 0/1 (winner even, loser odd);
  `result_gameset05` at (192, 360). Doubles: the `inpane_team` plate at y 320 in the team colour with "A/B Team".
- **Sounds**: the shown character's prize voice bank (`SND/VOICE/PRIZE/PC_HVnn[B].XB0`), program = lost, key =
  one sound-generator bit — at the start for singles or a beaten side, at the end of the hand slap for doubles
  winners (the second bit). The gallery gets point events 3 (ovation) or 4 (groan), which chain 3→7(→7 key 1),
  4→9 (`hst_sim::sound::Gallery`, test `gallery_match_end`). A winner also gets program 10 key 1 at volume 0x20 from
  bearings 45/135/225/325 and the BGM fading to 0 over 270 ticks.
- **Match-end jingle**: 3 when shown == winner, else 4. The port gave 4 for a bot-only match; fixed in play.rs.

## The port

`crates/hst/src/play/match_stats/ceremony.rs`: `plan()` (pure, unit-tested: the recorded doubles case, a beaten
human in singles, the ends swapped, the banner and plate quads), `begin` (loads the camera clip, motions, voice
bank, spawns the confetti under the match camera), `tick` (sounds, motions, confetti, run from the result
screen's 60 Hz loop while the fixed clock is held), `show` (camera, as `intro::show`), `pose` (marks and motions over
`character::animate`), `hide` (ball, panel, balloons, markers). The result page opens after the clip; the
figures, confetti and the clip's last view stay behind it. `match_over` copies the ends and the sound generator
(the next match reseeds it), so nothing touches the sim's generators or timing.

Left stick: a push past ±0.5 on `LeftStickX` acts as → / ← on the result page (edge-detected).

COM rank label: already done by B25a (the stats page shows the AI row's rank for COM players); nothing to change.

Checked side by side (`tools/shot.sh` with `HST_STATS=18 --stage 10 --play --pads -,-,-,-` via an `HST_BIN` wrapper,
shots in `context/p26c/port/`) against `context/p26c/run2/s0_144.png` / `s0_228.png`: the camera's framing and
path, banner, plate and confetti look the same; the result page then opens over the held view.

## Not verified / not 1:1

- **Characters differ in the side-by-side**: the port's default doubles line-up isn't slot 5's, so the motions were
  checked for playing and switching (hand → win), not frame against frame.
- **Confetti**: count, size, colours, sway and spin are by eye (the original draws from its global rand, not the
  sim generators); no recording of its particle code.
- **Lost / singles cases**: only the bot-only doubles win was recorded; the lost camera, its −1.3 offset, the groan
  and the singles win clip come from the decompile and a unit test, not a capture.
- **After state 0**: the original's result object may move the camera on (orbit); the port holds the clip's last
  frame behind the result page.
- **Mod characters (> 13)** play stock character 13's ceremony files (no own ceremony motions exist); the voice bank
  is character 13's too.
- **Racket02 track and faces**: the ceremony motions' extra tracks (second racket, face) aren't applied; the face
  keeps its last expression.
- **No interpolation** between ceremony ticks (motions and camera step at 60 Hz).
- **Jingle timing**: still played at the verdict (as before), not at the director's later cue.
- **Other aspect ratios**: the 720×720 shot came out at 1280×720 (the desktop rule sizes the shot window); the
  camera's FOV and the overlay use the intro's/result page's already-checked aspect handling.
- **Sounds**: heard-by-ear only in code review; no audio capture compared.
