# B203 — menu sounds stacking when mashed

Cause: every menu cue (cursor tick, confirm, back) was a fresh `play_centre` on the system bank, so presses
faster than a cue's length (or a held direction's auto-repeat) layered copies and the sum grew louder.

Fix: `Sound::play_menu(bank, key)` (`audio.rs`) — one menu cue at a time; starting one hard-cuts the previous
(its pending key-ons and voices). Used by the main menu and its screens (`main_menu.rs` `play`), the pause menu
(`menu.rs`) and the match result screen (`match_stats.rs`). Covers move ticks, confirm and back alike.

Test `audio::tests::menu_cues_do_not_stack` (needs the ISO): a cue every 1, 2, 4, 6 frames for 2 s peaks at
exactly one press's peak (0.394) and never has more than one cue's voices sounding.

## Not verified / not 1:1
- What the original does when a menu is mashed was not checked by ear (unattended run, no audio listening). The
  front-end is ours to design (PLAN), so the cut-the-previous rule stands on its own.
- The cut is hard (no release tail); fine for the short system ticks, not checked by ear for clicks.
- Intro/popup system sounds (`intro.rs`, `popups.rs`) are in-match cues, left on `play_centre`.
