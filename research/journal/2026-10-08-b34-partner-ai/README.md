# B34: the 1P CPU partner lets balls go by

## When

The app kept no shot record for a human player. It also never reset the records. So the human's slot held
`Shot::default()` for the whole match: n = 0, stamp 0, which reads as a contact at frame 0.

A computer partner beside a human (`mate > 0`) checks the human's record:
- In substate 1, give-way (3d5bb0) sees t = 0 ≥ 0 and f3 = 0 < 2.0. So it gives way to a human who will never play
  the ball.
- The not-found tail also follows the human's empty record.

In short, this hits every opponent shot that the CPU found a contact for while its partner was human. That is the
"sometimes": the CPU still plays the shots where it doesn't reach give-way (its own team's hit, serves).

## The original

A human player keeps an AI object of the same class. The player update runs 3646a0 on it (called from 173414 and
346864). That keeps the human's shot record up to date with where the human could play the ball. Fields:
- +0x40 found, +0x44 next, +0x48 kind, +0x4c min, +0x50 volley;
- +0x28 = 2 (beside a human for the tier search; the roll is still drawn).

The routine only runs on the opponents' shot, while the human's record isn't −2 and the slot isn't claimed
(path object +0x1fc).
- **Not found yet** (search from `next`):
  - smash → kind 3;
  - volley (if `volley`) → kind 2;
  - tiers with min 1, then min 0 → kind 0.
  - None found: next = n − 1, and if the path's last entry has bounced twice the record becomes −2.
- **Found:**
  - Kind 0 is re-searched each frame by the tiers from the current min. When nothing is found with min 0, the record
    becomes −2 if tier 3 has no swing frame.
  - Kind 2/3 is re-searched by smash/volley from the start. If that fails and its swing frame is gone, it is re-searched
    by tiers.
  - Any other kind becomes −2.

The path object's handler resets every record to n = −1 (and the claim) on the strike, new-path and point messages.
The base AI handler (35f070, messages 0x14/0x15/0x16/0xe) resets the human fields (35ef00): found 0, next 0,
volley = |z| < 6.4.

## Port

- `hst_sim::rally::Rally::human_record` + `Human` (the reset).
- `play/doubles_ai.rs`:
  - `human(g, i)` runs it for each doubles human from `control`.
  - `heard_shot` resets the records and the humans' searches once per shot count. It runs on the first step of
    either kind.

## Fixtures and test

`human_record_matches_the_game` (`crates/hst-sim/tests/ai_rally.rs`) replays tag-4 calls next to the CPU tags 1–3.
The fixtures are from `context/recordings/1p3goodcpus2.p2m2`: it is played with `tools/play_p2m2.py` to a vsync, saved
to slot 9 (`HST_SAVE_AT`), then `record_ai_rally.py 9 <frames> … 1 1,2,3,4` is run with the human idle
(`context/b34/run2.sh`).

| Fixture | From vsync | human | receive | NET | BASE |
|---|---|---|---|---|---|
| `ai_human_1p.bin` | 7930 | 126 | 172 | 358 | 310 |
| `ai_human_1p_b.bin` | 8380 | 165 | 41 | 108 | 394 |
| `ai_human_1p_c.bin` | 8960 | 200 | 0 | 200 | 400 |

All of them are bit-exact, including the records.

Reached:
- tier finds with min 1 and min 0;
- smash kind 3, found and kept;
- the kind-3 lost → tier path;
- the −2 puts.

Not reached:
- the volley kind 2;
- the kind-2 lost → tier path;
- the min-1 → min-0 retry after a find;
- the other-kind −2.

## Recorder

- `record_ai_rally.py` tag 4 hooks 3646a0.
- Tag 4 has no a1/a2 copies, so the stick/button compares are skipped.
- The new region 0x740 holds the path object +0x1f0 (the claim is at 0x74c).
- The scratch and drop counter are indexed by `tag & 3`.

## Gaps (P11k7)

- The claim slot +0x1fc (written by 35ed50 from 34d8a0/352880/352c40) is passed as never claimed.
- The resets are keyed on the shot count, not the 0x14/0x15/0x16/0xe messages.
- 3646a0's call conditions (+0x84, +0x3fa4, gm+0x344): the app runs it each doubles frame for every human.
- The branches no fixture reaches (above).
