# P11k5 — doubles NET/BASE branches from pad-driven rallies (2026-10-16)

Slot 3 (P1 human, partner CPU 2 on team 2, CPUs 1/3 opposite), P1 driven by `record_ai_rally.py`'s `HST_DRIVE=1`
(vpad, run to the ball, one button). Every fixture is recorded with `tools/record_ai_rally.py 3 <frames> <out> 1 1,2,3`
and replays bit for bit (receive, NET and BASE, every call).

## Fixtures (context/fixtures/, git-ignored)

| fixture | env | what it brings |
|---|---|---|
| `ai_rally_vpad_lob.bin` | `HST_DRIVE=1 HST_BTN=triangle`, 3000 frames | give-way beside a human, defer, substate-3 wait (partner), human searches |
| `ai_rally_vpad_fast.bin` | + `HST_AI=1:119:3,2:119:3,3:119:3` | tail repick beside the human |
| `ai_rally_vpad_base_lob.bin` | triangle, `HST_AI=*:121:3`, 3500 frames | tail repick between bots, more give-way branches |
| `ai_rally_vpad_slice.bin` | `HST_BTN=cross HST_AI=1:119:3,2:121:3,3:119:3`, 3500 frames | the one unpoked short-landing walk (landing + 0.3, by the human's partner) |
| `ai_rally_vpad_window10.bin` | `HST_DRIVE=1 HST_WINDOW=10`, 3000 frames | poked: short walks to landing, after-bounce descent, last entry |
| `ai_rally_vpad_window14.bin` | `HST_DRIVE=1 HST_WINDOW=14`, 3000 frames | poked: short walks to entry 11 as well |

`HST_WINDOW=<n>` (new) sets every CPU's path window (AI +0x34) to n once the shot count is ≥ 2 (rallies). Serves and
returns keep the game's 180, otherwise nobody returns the serve.

## Findings

- The short-landing walk is almost unreachable without a poke. The path object adds 15 entries a frame until its
  last entry has two bounces. The reaction wait (fast_ball + rand + base, doubles base beside a human) normally covers
  those build frames, so the copy already reaches the second bounce. Only the quickest rows (119/121: wait 2..4
  frames) catch it short, and then only before the first bounce (target: landing + 0.3). Once the ball has bounced,
  the copy is ≥ 15 entries and reaches bounce 2, so after-bounce, entry 11 and last need the window poke.
- Substate-2 hold (path mode 2, gap ≥ line margin): no rally ball in any recording ends up more than ~0.15 m out,
  so no margin is ever crossed unpoked. Tried: `HST_WIDE` with reach 2.5/3.0/3.5/4.0 and circle/cross/triangle,
  NET rows 115/129 (0.2 m margin), and `HST_LINE=-2 HST_MARGIN=-3` on slot 3. None held. It stays covered by the
  poked bot fixture `ai_let_go_s05.bin` (`cpus_let_out_balls_go`).
- Give-way beside a human runs only on the frame the substate reaches 1. When it says no, it sets the defer flag,
  and every later frame returns at once. In the unpoked fixtures it never said yes (all 25 calls deferred).
- Stash: the not-found tail ends by storing its target quad. Where the game's quad comes from its uninitialised
  stack, the replay feeds the recorded value (`Rally::stash`, record 0x730). It was reached on every tail call above.

## Test

`vpad_rallies_net_and_base_match_the_game` (ai_rally.rs) replays the new fixtures. It then reads, from the game's own
entry/exit records across them and the older `ai_human_*` pad fixtures, which branch each NET/BASE call ran.
Counts (unpoked / all):

- short walk landing 1/972, after-bounce 0/202, entry 11 0/148, last 0/230, chase kept 1/1552
- repick bots 5, beside human 17
- defer beside human 25/31
- substate-3 wait bots 7, beside human 10 (unpoked)
- give-way beside human by deciding test: own run < 2.5 and ahead 7, partner run < 2 0/1, own run < 2.5 4,
  frames within 30 2, frames/no shot 12

It asserts that each of these is non-zero (the poked counts where the unpoked are 0).

## Not verified / not 1:1

- Substate-2 hold: only on a poked state (`ai_let_go_s05`, line/margin poke). No natural rally ball lands past
  the margin (reason above).
- Short-landing walk's after-bounce, entry 11 and last targets: only with `HST_WINDOW` (window 10/14 instead of the
  game's 180). The game's own window never leaves a copy that short after the bounce.
- The unpoked short walk came up once (landing + 0.3, `_slice`).
- Give-way's "partner run under 2 m" branch came up once, in a window-poked match. The window doesn't feed that
  comparison, but the match is still a poked one.
- The test classifies branches from the records. It re-derives give-way's comparisons and the walk's targets in the
  test (same ps2 ops) rather than instrumenting `rally.rs`. Every call still replays bit for bit, so the port's
  outputs match whatever it ran.
- Pad timing is wall-clock (vpad over PINE), so re-recording gives different rallies. The fixtures are what's verified.
- The stash quad is the game's recorded stack value, not something the port computes (by design: the game's is garbage).
