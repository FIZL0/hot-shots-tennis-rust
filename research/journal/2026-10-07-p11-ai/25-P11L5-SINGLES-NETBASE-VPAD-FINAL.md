# 25 — P11l5: the singles NET/BASE branches the bot recordings missed (vpad)

Same save and driver as P11l4 (journal 24): slot 8 = `context/recordings/singles_1p_serve.p2s` (copied into copy 2's
`sstates/… .08.p2s`), P1 driven by `HST_DRIVE=1`, tags 1,2,3. The other three branches named on the line (NET fresh
tiers, after-hit middle spot and clamp, BASE smash taken/rejected) were already reached by `ai_rally_singles_base`,
`_net` and `_rows` (journals 16, 22).

## Fixtures (`ai_rally.rs singles_vpad_net_and_base_match_the_game`)

All `HST_SINGLES=1 HST_DRIVE=1 tools/pcsx2.sh python3 tools/record_ai_rally.py 8 <frames> <out> 1 1,2,3`; every NET
and BASE call replays bit for bit.

| Fixture | Settings | Frames | NET / BASE calls | Reaches |
|---|---|---|---|---|
| `ai_rally_singles_vpad_redraw.bin` | `HST_WIDE=1 HST_RATE=0`, 4000 | 4029 | 855 / 458 | 60-frame redraw ×7 |
| `ai_rally_singles_vpad_chase.bin` | `HST_CHAR_RALLY=1 HST_CHAR=1:0xe0:0.01,1:0xe4:0.01,1:0xe8:20,1:0xec:19.5,1:0xf0:19`, 3000 | 3033 | 466 / 299 | chase arrival ×2 |
| `ai_rally_singles_vpad_let_go.bin` | `HST_LINE=-2 HST_LINE_FROM=3 HST_MARGIN=-3`, 3000 | 3030 | 300 / 463 | line-margin hold ×1 (gap −1.29, margin −3) |

New recorder switches (`tools/record_ai_rally.py`):
- `HST_RATE=<pct>`: every CPU's walk-back rate (ai +0x18), every poll.
- `HST_CHAR_RALLY=1`: the `HST_CHAR` values only while the shot count is ≥ 2; the record's own values for serves
  and returns.
- `HST_LINE_FROM=<n>`: the `HST_LINE` line-call margin only from the point's nth shot.
- `HST_WIDE=x:y`: the wide stick is (±x, y), not (±1, −1).

The branches were found with temporary eprintln markers in `rally.rs` `singles_rally` (chase target kind/distance,
arrival, out call at the press, hold), since removed. The test counts them from the entry/exit records instead.

## Why each needed a poke

- **60-frame redraw.** Every singles row (0–55) has centre rate 100, so `Return::new` always draws "going" and
  the wait counter never moves. No recorded
  singles call had a back-wait above 0. It can't run in an unpoked singles match.
- **Chase arrival.** The walk only ends when the AI is within a step of a fixed target, and the search hasn't found
  a contact yet. Unpoked, the after-bounce targets get found by the search first (the closest approach was 1.4 m,
  then RUN). Entry-11 targets move with the ball and pull away (closest was 0.53 m). Five driven runs (○/△/✕, wide)
  and the bot fixtures never arrived. With the heights poked in rallies only, the CPU still returns the serve and
  then chases without finding. Poked for the whole match, it misses the serve too and there are no rallies.
- **Line-margin hold.** It needs an out call (predicted first bounce) at the press, so a pre-bounce contact on a
  ball that lands out by at least the row's margin (0.2–1 m). These unpoked tries reached no hold:
  - ○, △, ✕ with `HST_WIDE`, and ✕ with NET row 45 (margin 0.2) and CPU speed ×1.8;
  - ○ straight-long (`HST_WIDE=0:-1`) and 0.6-wide (`0.6:-1`) on row 45.

  They gave a handful of out balls (gaps 0.21–0.75 m), all while the CPU had no contact (substate 0). `HST_LINE`
  from the first shot faults every serve, so there are no rallies; from the third shot it works.

## Not verified / not 1:1

- **60-frame redraw:** proven only with the walk-back rate poked to 0. No singles row has a rate under 100, so the
  unpoked game never runs it in singles (doubles rows do, but that's the doubles routine).
- **Chase arrival:** proven only with the CPU's contact heights poked (in rallies). No unpoked recording, bot or
  driven, arrived.
- **Line-margin hold:** proven only with the line-call and let-go margins poked (as `ai_let_go_s05` for doubles).
  The driven human's out balls never met a CPU contact. Once, with the poke.
- **Pad timing is wall-clock,** so re-recording won't reproduce these rallies. The fixtures are what's checked.
