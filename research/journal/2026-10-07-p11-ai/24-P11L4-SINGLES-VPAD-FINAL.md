# 24 — P11l4: the singles receive's last branches, from P1's serves (vpad)

Same method as P11k4 (journal 23), but on a singles match with a human P1. There was no such save, so I made one.

## The save

Copy 1's slot 1 (doubles Game Selection) → ○ ×3 backs out to the Main Menu (on this pad ✕ confirms, ○ backs out;
journal 3's "○ confirms" was another pad mapping) → ✕ Fun Time Tennis → left, ✕ Singles → ✕ 1P = 1-A → 2P: its
cursor already sits on COM, ✕ → character select (saved: `context/recordings/singles_1p_select.p2s`) → ✕ Ashley,
✕ the COM where its cursor starts (character 1, row 1, level 3) → ✕ "Start the match!" → wait for P1's first serve
(gm+0x55 = 2, P1 +0x3fa4 = 1) → `context/recordings/singles_1p_serve.p2s`, loaded in slot 8 for every run below.

## Recorder

`HST_SPEED=k:scale` (new in `tools/record_ai_rally.py`) scales one player's speed stat. Only P1's position reaches
the CPU's receive (the opponent region 0x500), so a fast P1 changes no AI state the replay doesn't read.

## Fixtures (`ai_rally.rs singles_vpad_serves_receive_matches_the_game`)

All are slot 8 = `singles_1p_serve.p2s`, tag 1, `HST_SINGLES=1 HST_DRIVE=1`. Every call replays bit for bit.

| Fixture | Settings | Frames | Calls | Reaches |
|---|---|---|---|---|
| `ai_rally_singles_vpad_wide.bin` | `HST_AI=1:29:3 HST_SERVE=cross HST_SERVE_Y=0.5 HST_SERVE_WALK=1`, 6000 | 6032 | 2444 | out-of-court plan 11 (x −5.92) |
| `ai_rally_singles_vpad_wide_b.bin` | same + `HST_SERVE_DELAY=7` | 6032 | 2404 | out-of-court plan 10 (x −5.87) |
| `ai_rally_singles_vpad_upgrade.bin` | `HST_SPEED=0:2.5 HST_AI=1:29:3 HST_SERVE=cross HST_SERVE_Y=1 HST_SERVE_WALK=1`, 4000 | 4033 | 1251 | level-3 plan-10 upgrade (P1 at z 3.83) |
| `ai_rally_singles_vpad_last.bin` | `HST_PATH_STOP=9 HST_SERVE=cross HST_SERVE_Y=1 HST_SERVE_WALK=0`, speed 0.3, 2000 | 2033 | 1140 | last-entry pick ×22 |
| `ai_rally_singles_vpad_smash.bin` | `HST_CHAR=1:0xe0:0.01,1:0xe8:1.8,1:0xec:1.2,1:0xf0:0.8 HST_SERVE=cross,circle HST_SERVE_Y=1 HST_SERVE_WALK=0`, 3000 | 3029 | 1061 | smash find ×4 |

The branches were checked two ways: temporary eprintln markers in `rally.rs` `singles_aim` (out10, out11, upgrade,
special; since removed), and the test's own reading of the exits. The test counts a plan 10/11 aim with |x| > 5.485
as out-of-court. It counts a plan 10 inside the court, on a row with special-return rate 0 and with the opponent
inside 7.4 m, as the upgrade.

Row 29 (ALL, return levels [0, 0, 100], special return 0) gives the upgrade its best odds: return level 2 every
time, 60 % × 20 %. It also keeps the special return from taking plan 10.

What it took:
- **Out of court.** The chance is (|x| − 5.485) × 100 / 2.5 %, capped at 20 %. Singles receivers aimed at |x| ≤ 5.5
  with the doubles fixtures' serve (✕, Y = 1). I swept ○/□/△, Y −1..1, the walk on and off, and a slowed receiver.
  Only ✕ at Y = 0.5 with the walk on put the receiver at 5.9–7.2. Six 6000-frame runs with different
  `HST_SERVE_DELAY` gave two plan 11s and one plan 10.
- **Upgrade.** At normal speed P1 (driven to 4 m off the net) was still 7.4–10 m deep when the CPU aimed: 0 of 25
  aims qualified. At ×2.5 it was at 3.8–6 m, and the first run hit it.
- **Last-entry pick and smash find.** Unreachable unpoked, for the reasons in journal 23. I used the same pokes on
  the singles receiver (player 1).

## Not verified / not 1:1

- **Last-entry pick and smash find:** proven only on poked states, as in doubles (journal 23): a stopped path at
  0.3 speed, and the receiver's character heights. The branch code runs bit-exact on recorded inputs, but no unpoked
  recording reaches it.
- **Upgrade:** reached with P1's speed stat ×2.5. That is a state no real character has, but only P1's position
  feeds the CPU's decision.
- **Out-of-court aims and upgrade:** rely on row 29 forced on the COM (`HST_AI`), not the row its character
  select gave (row 1).
- **Pad timing is wall-clock,** so re-recording won't reproduce these points. The fixtures are what's checked.
