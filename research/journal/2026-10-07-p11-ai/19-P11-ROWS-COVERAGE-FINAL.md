# 19 — singles rows recording, coverage

`tools/record_ai_rally.py` takes `HST_AI=k:row:level,...` and points player k's AI object (player+0x80) at
AIParam row `row` (+0xc = 0x3174c0 + row·0x118) with level byte `level` (+0x10) before resuming.

`HST_SINGLES=1 HST_AI=0:45:3,1:16:1 tools/pcsx2.sh python3 tools/record_ai_rally.py 8 6000
context/fixtures/ai_rally_singles_rows.bin 1 1,2,3` (`bots_singles`, rows 45 level 3 and 16 level 1, both NET):
6037 frames, 2952 draws, nothing dropped. Bit-exact in `singles_receive_matches_the_game` (2882 calls) and
`singles_net_and_base_match_the_game` (NET 4294 calls; no BASE calls, both rows are NET).

Coverage (temporary eprintln markers, all fixtures):
- reached now: singles return level 0 (rows), NET fresh tiers, the NET after-hit middle spot and its opponent
  clamp, the BASE smash rejection; the receive's landing pick (singles fixtures) and the push after a right guess
  (`ai_rally_s05_slow*.bin`).
- still unreached: out-of-court aims, special return, level-3 plan-10 upgrade, last-entry pick, smash find in the
  receive; walk back's 60-frame redraw, line-margin hold, chase arrival, BASE smash taken. P11k4/P11l4/P11l5 updated.

The app's singles step now applies the dispatcher's kind lock (`lock_button`) to the routine's button.
