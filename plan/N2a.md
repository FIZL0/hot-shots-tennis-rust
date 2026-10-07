# N2a

- [x] **N2a — Serve ball bouncing.** (User request 2026-10-06.) Before the toss the server bounces the ball on
  the court as the original: when it starts (on entering the serve stance / after the walk), how many bounces
  and whether that varies (per character, RNG, held input), the ball's path hand → ground → hand (a scripted
  track on the motion's ball bone like N2's `sh_pcNN_serve_ad00_ball`, or real ball physics), the motion that
  plays it, bounce sound trigger (N3), and whether pressing serve cuts it short. Builds on N2 (ball in hand,
  game ball model). Verify ball position and motion number/time per frame through a serve's bounces against a
  recording (slot 5 serves in `match_s05.bin`, or a fresh capture with the live ball).
