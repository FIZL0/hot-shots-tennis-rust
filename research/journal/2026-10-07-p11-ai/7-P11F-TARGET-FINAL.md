# P11f — target choice (FINAL)

Port: `crates/hst-sim/src/aim.rs` (`Aim`, `Look`, `Pair`; `AiParams::pair_aim`, `aim_at`, `receive_aim`,
`rally_aim`, `aim`, `lets_go`; `fine_zone`, `out_by`) and `position::zone_in`. Tests in `crates/hst-sim/tests/ai.rs`:

- `aims_match_the_game` — `context/fixtures/ai_aim_s05.bin` (slot 5, doubles, 10029 frames): all 50 calls of the
  doubles chooser bit-exact (stick bits, plan byte, generator index after), by contact kind 0..4: 15/8/14/4/9.
- `singles_aims_match_the_game` — `context/fixtures/ai_aim_singles.bin` (bot-only singles, 8043 frames): all 51
  rally-chooser calls bit-exact, plus the short-ball flag; kinds 0/1/3/4: 15/31/4/1.

## The original

- Doubles (the default play mode, and slot 5) has a single dispatcher, called by both the receive and the rally
  state, that branches on the AI's contact kind: 2 volley, 1 low ball, else ground stroke (4 dive; 3 smash re-aims
  with plan 0x11). Its picks are lanes (0/1/2) from where the two opponents and the partner stand (`zone_in`,
  blurred for the opponents), the formation byte (2 = both up) and the smash-third flag, then
  `aim_at`/`aim` turn the lane and depth into a stick.
- `aim_at` (aim at a point): the direction from the AI to the point, carried on to the baseline
  ((11.885 + |me.z|)/|dz|) or cut at the side line; x/w and (depth − 4.4425)/4.4425, normalised; length band from
  the row's `key_level` mix (0.2/0.4/0.6/0.8/1.0 edges); angle ±`angle_width` degrees, wrapped, turned on VU0
  (rot_y). Only z takes the side sign.
- Singles: separate return-of-serve and rally choosers (volley and low-ball sub-choosers shared). Rally reads the
  opponent's zone and finer sixth (`fine_zone`, blurred), the AI's kept spot (AI+0x130/0x138 in the original), the
  contact height against the player's reach heights, the opponent's last shot kind and hand byte, and a short-ball
  flag it clears and maybe sets again. Depth tables by base level are in `RECEIVE_DEPTH` / `RALLY_DEPTH`.
- Row fields: line margin, angle width, base level mix, special return rate, key level mix.
- Line margin (the let-go): when the ball predictor says the first bounce is out, with less than 9 frames to the
  contact, the AI lets it go if margin ≤ the signed distance to the nearer line (x: |x| − half width 4.115/5.485;
  z: |z| − 11.885; z when |dz| ≤ |dx|). A serve uses the service box (4.115, 6.4).
- Body and low shots: the body-shot flag and the low flag on the AI feed the contact searches (P11e4's body-shot
  search), not the target choice. The singles smash's body shot (aim at the opponent, `body`) is in `receive_aim`.

## Recording

`tools/record_ai_aim.py <slot> <frames> <out> [singles]` patches jumps onto the chooser(s) (doubles dispatcher; or
the two singles choosers) like record_ai_side.py; 0xc90-byte entry/exit records (format in the docstring). Runs
over ~10000 frames have failed with "PINE batch failed": keep them shorter.

Singles save state (copy 3's scratch slot 9): slot 1 on copy 3 is the doubles controller screen and its pad has
circle = back, cross = confirm (swapped against 3-P11E2's note). Circle → singles/doubles → left, cross → 1P down
×5 cross (COM) → 2P down ×4 cross (COM) → right ×3 cross (JJ) → right ×2 cross (Jun) → cross until the match loads.

## In the app

`play.rs`: `ai_aim` (the bot's stroke stick: `pair_aim` in doubles, `rally_aim` in singles) and `ai_lets_go`
(steps the stand-in flight to its first bounce; a bot doesn't swing at a ball out by its margin).

Open / ponytails: bot-only singles never called the return-of-serve chooser in 8000 frames, so `receive_aim` and
the singles volley branch are hand-checked against the decompile only. The app passes level 3, volley level 0 and
no smash-third; the real level picks are P11g. The app's let-go has no predictor timing (it decides at the press).
