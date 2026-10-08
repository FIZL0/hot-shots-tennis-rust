# B28 — Between-points camera bottom half (FINAL)
- Symptom (stage 1, `--play`, autoplay): on cut-aways from behind/beside the near players (0x61, 0x62 on a near
  player) the bottom third of the screen was the clear colour, cut by a straight horizontal line, and the shown
  player was missing or sliced; court views (0x0c, overhead) looked fine.
- Cause: not the shot camera (its eye/rotation are verified bit-exact in `cutaway_s05`) but the render camera's
  near plane, fixed at 5 m for the ~40 m match camera (`play.rs` camera sync). The cut-aways stand 1–4 m from the
  player, so the near ground and the player fell in front of the near plane and were clipped.
- Fix: near plane 0.1 m. Bevy's reverse-Z Depth32Float keeps relative depth precision independent of the near
  plane, so the layered character models at 40 m show no z-fighting (checked on a match-view crop).
- Original (slot 5, six cut-aways via PINE + F8 screenshots, `context/b28/orig_*.png`): players framed up close with
  the ground to the bottom edge, as the port now draws (`context/b28/port_*.png`, `m1.png` before / `m2.png` after).
