# B26 — P1 misses serves they should hit

Captures: `context/b26/run1/` (`tools/record_serve.py 3 …` with vpad; decoder `research/serve_timeline.py`), plus
B29's `context/b29/run{2,3}/`. RAM: `context/ram/s0{3,4,5}.bin`, `context/fixtures/{slot5,1p3goodcpus}_ee.bin`.

## What the original does (checked against the port)

- **The swing press is an edge.** The press window opens at the toss's release (toss motion t + speed ≥ 45). A
  button held through the release never swings (X1_hold: ✕ held from before the release → no swing, the toss lands and is
  re-tossed). The port's `tick_input` presses are edges as well, so they match.
- **Contact search** (strong toss: its own table; weak and underhand: the weak table): this is `serve::search` as ported.
  The horizon is min(table length, path left) and the search stops at the first bounce. Late presses that whiff in the original
  (B29 E2 at +76 after the release, E3 at +62 with the ball at 1.90 m under the 2.0 m low) whiff in the port too.
- **Where the port went wrong: everyone served with character 0's serve data.** `serve_data` read TParam row 0 for
  every player's contact window (and so the toss apex), reach, power and short-miss factor. It also used one hard-coded
  timing table (character 0's strong one, 21 frames) for both tosses. The original builds the two tables per
  character at setup, from TParam's "a/b/c" counts (columns 30/31 strong after/before, 32/33 weak):
  - It builds them the same way as the rally table (`swing::timing`, columns 28/29).
  - If a toss table's leading no-hit frames differ in number from the rally table's, its first 8 grades and biases
    are copied from the rally table.

  So, for example, Carol's strong table is 20 frames, not 21. Characters 8–11's tables are shorter (char 11: strong 1/0/9 after, weak 2/0/8), and characters 3, 7, 9–11 have higher windows (3.4/2.9/2.4 for 3 and 11). A human serving as one of those got
  the wrong contact frames: misses and grades the original wouldn't give, and the reverse.

## Port changes

- `serve::timing(after, before, stroke)` builds the toss tables as the original does.
- `play.rs` `serve_data(row, data)` reads the player's own TParam row (`mod_match::row`, so a mod's own row too).
  Everything from that row now comes from it, timing tables included. The unused `tparam_row` is gone.
- The short-miss factor goes through the game's decimal reader (`ai::atof`): character 11's "2.7" reads as
  2.6999998 in RAM, not Rust's 2.7.
- Tests:
  - `hst-sim/tests/serve.rs serve_timing_tables_ram`: every player in the five RAM dumps. The tables are built from the RAM counts
    and checked against the RAM tables, grades, biases and lengths, bit for bit.
  - `play/serve_check.rs serve_data_per_character`: the app's serve data for slot 3's line-up (characters 6, 4, 3, 11) is checked against
    that save's RAM: windows, reach, power, low power, short-miss factor and both tables.

## Not verified / not 1:1

- **□ at the serve**: the original tosses weak on □ and doesn't swing on it. The port's keyboard serve key rides on □
  and acts as strong (toss and swing). This is left as it is because it is the keyboard's only serve key (`tick_input`). A pad's □
  therefore isn't the original's.
- **Press-frame sweep, port vs original, per character**: not re-recorded after the fix. The per-character tables
  and windows are proven against RAM, and the search and acceptance already matched B29's captures (Carol). X2_tap's first tap fell
  in the post-load input gate, so it shows nothing.
- **Underhand table**: the original uses the weak table for the underhand swing, as the port does. No underhand
  press sweep was recorded for this task.
