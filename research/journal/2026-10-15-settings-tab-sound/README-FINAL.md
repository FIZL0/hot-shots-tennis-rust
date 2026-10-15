# B442 — Settings tab switch sound, quieter select cue

User (2026-10-15): switching tabs in Settings played the loud click; the select cue should be a bit quieter.

- Tab changes (L1 / R1 in `step_tabs`, a tap on a tab pill) went through `Menu::go`, which pushes the select cue
  (system bank key 2). They now use `Menu::go_tab` (`play/main_menu/options.rs`): same screen/row change, the soft
  cursor tick (key 0) instead. Opening Advanced from its row still plays the select cue (it is a pick, not a tab).
- `Sound::play_menu` (`audio.rs`) plays key 2 at `MENU_SELECT_VOLUME` = 0x68 (≈81% of 0x80, ≈ −1.8 dB); cursor and
  back cues stay at 0x80. Applies everywhere `play_menu` is used (main menu, pause menu, match stats).
- Test: `options::tests::a_tab_change_ticks`.

## Not verified / not 1:1
- Loudness only checked by reasoning about the linear sequence volume, not by ear (unattended run, no listener).
  Front-end sounds are ours to design (PLAN.md), so no comparison with the original is needed.
- The Mods page's tabs (`mods_tab`) were left as they were: the task names the Settings screen.
