# B262 — the server at a change of ends

## Bug
Through the change-ends phase the next server stood still with the ball hanging beside them. The placement
(`change_ends_placement` → `reset_positions`) leaves the ball at the server's spot + 0.3 m x, y = −1. Game space y
points down, so that's 1 m *up*. `held_ball` and the serve branch of `motions` only ran in `Phase::Serve`. They also
keyed on `g.score.server`, and that's still the *last* game's server during the phase: the placement advances the
score only on a copy.

## The original
`context/b49/run1/shot_00..79.png` (slot 5, the B49 capture over the change-ends phase): the next server stands at
the new end in the serve stance. The ball is in the left hand, bouncing (shot_20: low by the hand; 60/79: at the
hand), and the pose changes from frame to frame. So the stance motion 0x20 plays over the whole move. No new capture
was needed.

## Port
- `change_ends::server(g)`: during `Phase::ChangeEnds`, the server of the next point (score advanced on a copy, as
  the placement does).
- `motions`: that player takes the serve branch (`g.serving` is default after the placement → motion 0x20).
- `held_ball`: during the phase it puts the ball on that player's stance ball track (`stance_ball`), as in the serve
  set-up.
- Test: `change_of_ends_glides` (both variants) now asserts on every change-ends tick that the server's motion is
  0x20 and the ball is in their hand (0.06–0.6 m up, < 1 m from them).
- App shot `context/b262/run1/g165.8.png` (`--play --singles --chars 6,1 --games 4 --sets 1 --umpire 4 --shot-at
  165.8`, autoplay): "Change Sides" with the server bouncing the ball in hand.

## Not verified / not 1:1
- `g.flight.ball.pos` now follows the hand during the phase instead of sitting at y = −1. It's a sim field, but no
  sim RNG draw or phase timing reads it there. The only readers in the phase are presentation (the camera's end
  view, the umpire's look at `next_point`) and `locomote`'s side pick for a human in the phase. In the original the
  ball is in the hand there too, so this is closer to the game. The replay/synctest suites still pass (done.sh).
- Whether the original restarts the stance at frame 0 on the phase's first tick, or when the serve phase starts:
  not recorded. The port starts it on the change-ends tick and plays straight on into the serve (presentation only).
- In doubles, the serving pair's partner isn't changed (stands, as before). The capture shows partners standing.
- The app shot has no court drawn (the same in the B73-style shots: a `--shot` environment issue, not this task).
