# B63: the original's menu → match transition (capture in progress, run ended)

## Found (PCSX2 copy 1, slot 2 → confirm screen → "Start the match!")
- ✕ confirms on copy 1 (○ backs out). Slot 2: ✕ ×4 + Start reaches Confirm Settings; the menu ignores ✕ until the
  VS banner's intro is over (~170 frames after a state load), so wait ≥4 s before pressing.
- Sequence: the last menu frame **zooms up about the screen centre while fading out**, over the "Now Loading..."
  screen (HST logo, a help-tip panel with the pad diagram: that screen is B43), ~30 frames (vsync ~171–180 → ~206–211
  after the load, from 0.02× slow-motion shots; ±5 frames). The loading screen holds ~6 s at 1×, then the court
  intro fly-by (Skip), "Start the Match!", play.
- Not the transition: the menu's help-pane object (draws `Loading.tm2` / `L_txt..` panes) and the sound-volume fader
  (+1/−1 per frame) next to it in the menu overlay.

## Tools
- `HST_SPEED=<x> tools/pcsx2-hst.sh` (this commit) launches a copy in slow motion.
- Frame-exact shots work in lock-step (`HST_LOCKSTEP=1`, `Pine(step=True)`): send the copy's F8, then `next_frame`;
  the PNG is that frame (wait for its size to settle before copying). Recipe in this file; the script was scratch.

## Left
- Frame-exact zoom/fade curve: rerun the lock-step capture with the ✕ pressed ≥5 s after loading slot 8 (scratch save
  at the confirm screen, copy 1), ~90 frames.
- The match → menu direction (pause → "Return to the Menu") not captured yet.
- Port: freeze the last menu frame (Bevy screenshot to an image), zoom + fade it over the loading screen for the
  measured frames, then switch (`mode.rs` `switch`/`show_loading`).

## Not verified / not 1:1
- Transition length and zoom/alpha curve: measured ±5 frames only, not frame-exact; not in the port yet.
- Match → menu transition: not captured.

## Port: a look-alike zoom-out (user 2026-10-11)

The user decided menu, loading and transition screens only need to look similar and good, not be 1:1, and that the
"Loading..." text was a temporary placeholder. So `mode.rs` now:

- Before leaving a running mode, asks for a screenshot of its last frame (it waits at most 10 frames, for headless
  runs with no renderer). The capture comes back CPU-only (`RenderAssetUsages::MAIN_WORLD`), so it is re-flagged
  before upload, or it draws nothing.
- Draws that frame full-screen above the loading cover and scales it 1 → 1.5 about the centre while fading it out over
  0.5 s of real time. The new mode is built only after the fade ends.
- The loading cover is plain black now, with no text.
- `main_menu.rs`: `Screen::Playing`, set when the confirm screen starts the match, used to draw as the main menu, so
  the snapshot showed the main menu instead of the screen the player left. It now keeps drawing the confirm screen.

Checked in the running app (frames saved during the fade, in `context/shots/r1/b63/zoom4*`): from the confirm screen,
Enter zooms out the confirm screen and fades it to black, then the match loads in the same window with no panic.
`tools/check.sh -p hst`: 83 tests passed.

## Not verified / not 1:1

- The transition is a look-alike, not frame-matched: FADE 0.5 s and ZOOM 1.5× are eyeballed against the original's
  roughly 30-frame zoom-fade (user decision).
- The way back (match → menu) runs through the same code, but I didn't look at it frame by frame. I also didn't drive
  the pause dialog's "Return to the Menu" key path in this pass.
- The original's "Now Loading" screen with hints is B43 (still open). The port shows a black cover.
