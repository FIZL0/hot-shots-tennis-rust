# B40i — menu values from the original: closed by user decision (2026-10-11)

The user, 2026-10-11: "menu and loading and such do not need to be 1:1 and just make a version that looks similar and
good". B40i was "replace B40's eyeballed menu values with the original's". Items (1)–(7) are all menu presentation:
sprite rects, font advance, menu sound keys, portrait cells, court order, play-style/grade labels, umpire faces.
B40 already made them look like the original, so per the user they don't need reading from MENU.BIN.

- (8) sets 1/3/5 → sets-to-win: verified from the menu code earlier (4-B40I in the b40 journal).
- (9) each pad keeps its number from the menu into the match. Checked in the code: the menu (`main_menu.rs`, the
  `pads` list) and the match's input (`play.rs`, `PadSlots::update`) both number pads by the sorted list of readable
  `Gamepad` entities. B63's one-process switch keeps the `Gamepad` entities (`mode::teardown` skips them), so the
  `--pads` indices the menu writes point at the same pads in the match. Covered by the `main_menu.rs` seat-mapping
  test.

## Not verified / not 1:1

- (1)–(7) stay B40's look-alike values, not read from the disc (user decision: menus need not be 1:1).
- (9) wasn't tried with two physical pads. An unattended run has none, so it is checked in the code only.
