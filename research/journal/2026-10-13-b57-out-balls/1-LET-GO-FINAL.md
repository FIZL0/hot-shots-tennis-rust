# B57 let-go: findings and port (FINAL)

## What the original does

- **Where the check is.** It sits in the NET and BASE rally routines' substate 2, which was already ported bit-exact
  in `hst_sim::rally`. When `swing + lead < 9`, the routine reads the path object's mode byte (+0x5d) and line gap
  (+0x60). If mode == 2 (out) and !(gap < row.line_margin), the AI sets its hold (+0xbc) and drops to substate 0.
  - Substate 0 returns early while held, so the AI neither presses nor runs on.
  - There is no let-go motion: the player just stands.
  - The receive routine (return of serve) has no such check.
- **Which CPUs.** All of them, through their AIParam row's line margin (CSV column 33):
  - rank 0: 1 m
  - ranks 1–5: 0.5 or 1 m
  - ranks 6–11: 0.3 or 0.5 m
  - ranks 12–15: mostly 0.2 m

  There is no random chance. Singles and doubles use the same check, with the lines of the current call.
- **Mode and gap, built per frame by the path builder.**
  - On a path reset, mode = 0.
  - Each path step sets mode = the predictor ball's line call (+0xa5). The predictor ball is a copy of the ball,
    stepped with no net.
  - The gap is written only on a step where the previous mode was 0 and the new call is 2. It then takes the
    predictor's distance to the nearest *true* line (+0x230, no margin).
  - Otherwise the gap keeps its old value, across paths and points.
- **Evidence from the fixtures.**
  - Existing fixtures contain mode-2 entries with negative and stale gaps (`ai_human_vpad_b`, `singles_base`,
    `singles_spec`), which matches the stale-gap rule.
  - None of them had a CPU in substate 2 facing an out ball.

## Port

- `play/doubles_ai.rs`:
  - The path build is now `predict`. It returns the path, the mode (the far end's call) and the gap.
  - Gap rule: the gap is taken when a step goes from `Call::None` to `Call::Out`.
  - `Shared::gap` keeps the gap across paths, as the original does.
  - `step` feeds both into `World::path_mode` / `path_gap`. Before this change they were always 0, so CPUs never
    let a ball go.
- Unit test `path_mode_and_gap`: a rally ball landing past the baseline gives mode 2 and a gap equal to its distance
  past the line. A ball landing in gives mode 1 and no gap.

## Recording a let-go

Ways that did not produce an out ball in slot 4 with P1 driven (`HST_DRIVE`):
- wide/long stick at contact (`HST_WIDE`)
- early presses (`HST_REACH=5`)
- lobs (`HST_BTN=triangle`)

All three gave mode 0/1 only: the human's shots kept landing in.

What worked: `tools/record_ai_rally.py` gained two options.
- `HST_LINE=<m>` pokes the line-call margin global, so the lines pull in.
- `HST_MARGIN=<m>` pokes every CPU row's let-go margin.

Slot 5 with `HST_LINE=-2 HST_MARGIN=-3`, 3000 frames, became `context/fixtures/ai_let_go_s05.bin`:
- 2887 mode-2 entries;
- two real holds by the BASE CPU in substate 2 (gaps −1.55 and −1.71).

Test `ai_rally.rs cpus_let_out_balls_go` replays every receive, NET and BASE call bit-exact, with the table's
margins set to −3 as poked, and requires both holds.

## Not verified / not 1:1

- **No unpoked let-go.** No recording has a let-go with the game's own margins. Every out ball we could make was
  either not in front of a CPU in substate 2 or was poked. The hold branch and its inputs are the game's; only the
  two data values differ.
- **Path horizon.** The original's mode is the call at the path's growing end (14 steps on the hit, then 15 more a
  frame, up to 600). The app's is the call 180 steps from now. A ball still unlanded 3 s ahead would differ; that
  doesn't happen in a rally.
- **The app's mode and gap are not compared tick for tick** against a recording. The fixtures hold the path
  object, not the predictor ball's own steps, so there is nothing to compare them to.
- **Stale gap.** The app's gap persists like the original's, but it is only refreshed on ticks when `step` runs
  (CPU players). The original refreshes it every frame.
