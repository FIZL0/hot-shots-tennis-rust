# B11 — ✕/○ hit effect and feel

## Effect choice: already right

The impact model follows the button's shot kind, the same in the game and the port:
- The stroke launch `0x3467b0` ends with effect event 1 carrying the button's kind (`s4`). The manager stores that kind at fx+0xd0, and the impact, sparks, glow and ribbon all go by it.
- In `effects_s05`, all 22 recorded impacts follow that rule:
  - The model is the shot kind, or model 5 on a smash.
  - fx+0xd0 equals the ball's kind (ball+0x5c), with one exception.
- The exception is a framed mis-hit (player+0x3f06). The game then launches a class-2 lob (`0x37b0d0` with kind 3) but keeps the button's impact (flat in the capture). The port has no mis-hits yet (P3).
- `tests/effect.rs` now asserts this rule.

So ✕ shows the topspin starburst and ○ shows the slice impact. The glow disc (`impactef_*`, P9e) is the circle, and it plays on every non-smash hit in the original too.

## Feel: the spins were placeholders

`strike` gave strokes and volleys a fixed `KIND_SPIN` per kind, and no first-bounce spin or restitution. The game takes all three from the hitter's shot record instead. The launch chain is `0x37b0b0`/`0x37b0d0` → `0x37b110` → `0x37e420` → `0x37e950` → `0x379080` → `0x375da0`. Record fields map like this:

| Field | Meaning | Ball offset |
|---|---|---|
| 7 | spin° | ball+0x1a4 |
| 9 | first-bounce spin° | +0x1a8 |
| 11 | first-bounce restitution | +0x1ac |
| 12 | second restitution | (blend target) |

`0x37e950` then adjusts these for strokes (class 1). The constants are at 0x4108a8–0x4108d4. The clamp bounds 0x410aa0–0x410b18 are int64 0/1.

| Kind | Adjustment |
|---|---|
| flat (2) | spin × (1 + 1.5·low) × (1 + 2·near²), where low = 1 − min(−y/2, 1) and near = clamp((11.885 − \|z\|)/8, 0, 1). If low + near² > 1, both are first normalized by their sum, done in sequence. The `0x1161c0` call is fdlibm powf(near, 2) = near·near. |
| lob (3) | spin += 200°·(1 − clamp((\|Δz\| − 4.5)/5.5, 0, 1)) |
| drop (4) | first-bounce spin × (1 − 0.6·near) |

For any record with field 12 ≠ 0, the restitution becomes the following, for all classes:

r + clamp(−y/1.5, 0, 1)·(1 − near)·(field12 − r)

Field 10 (side angle) is 0 in every stroke and volley record, including the three archetypes. The velocity side-rotation and the left-hander spin flip (`0x3551c0`) both hang on it, so they never apply to these shots.

Size of the error:
- The port's topspin was 2.97 rad/frame. The game's stroke records give 4.0–6.7, so ✕ strokes dipped far less than they should: floaty.
- The port's slice was −2.09. The game's records give −1.3 to −1.9 for strokes and −0.45 to −0.81 for volleys.
- Lobs had no first-bounce kick.

## Port

- `hst_sim::params::rally_spin(record, class, kind, hit, target)` returns (spin, first-bounce spin, restitution) in ps2 order.
- `play.rs`:
  - Each player keeps their character's class 1/2 records (`rally_records`).
  - `rally_spin` replaces `KIND_SPIN` in `strike` and sets the shot's first-bounce fields.
- The port uses the base record. In the recordings, the game used an up1/dw1/dw2 variant record on about half the launches. That choice goes with the trajectory-table mode, which is still P3, as are the stroke speeds that don't match the base table exactly.

## Check

`shot_tables.rs` `rally_spins_like_the_game` covers 210 strokes and volleys from match_s05, new_recording, lob_smash_s05 and human_smash_s04:
- Every launch's spin, first-bounce spin and restitution match bit for bit for one of the hitting players' records (base or variant).
- 97 of them match on the base record.
- By kind, class 1 had 36 top, 20 slice, 47 flat, 32 lob and 2 drop; class 2 had 18, 6, 38, 10 and 1.
- The hitter is matched against any player's record, because the nearest player is sometimes not the hitter (one flat in lob_smash_s05).

Not checked:
- The feel was not checked in game or side by side in PCSX2.
- Trajectory differences that come from the mode/variant choice (P3) remain.
