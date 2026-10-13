# B153 — players and ball under the "Start the Match!" banner (FINAL)

## Original (`research/b153_banner.py`, doubles from slot 2, court 4; log in `context/b153/run1/log.txt`)
- The match's frame counter (gm+0x58) and every player's motion clock run through the whole intro (phase 0), from
  its first frame, not just under the banner. Motion ids the whole time: the server 0x20 (its stance: the ball
  bounce loop, ~52 frames), the receiver and both partners 0 (ready). Positions stay at the serve set-up.
- The live ball sits on the stance's ball track: x/z within ~1 cm of the server's hand spot, y bouncing between
  −0.06 and −0.55 (game y down), velocity 0.
- Phase 2 starts with no change: the same motion ids, clocks counting on (server t 27→31 across the switch).

## Cause
The intro holds FixedUpdate (the match ticks), and with it `motions`, `character::tick` and `held_ball`, so under
the banner (and the flyover) the figures showed whatever the setup left: no stance loop, the ball at its set-up
spot instead of the server's hand.

## Port
`play/intro.rs` `pose_players`: on each of the intro's own 60 Hz ticks it runs `motions`, `character::tick` and
`held_ball` (cached one-shot systems). That's only the drawn motion clocks and the held ball's spot. The ball is
rewritten every tick before the toss, and the toss leaves from `serve::toss_points` (the player matrix), not from
the held ball. So the sim state, its RNG and match timing are untouched. The match now starts with the stance clocks
where the intro left them, as the original does. No play.rs change.

## Side by side
Court 5 doubles, `--shot`s at 5.6/5.9/6.2 s with `HST_INTRO=1` (`context/b153/fix/`, sheet `sheet.png`, server
close-up `server.png`) against the original's `context/b42d/orig_c5/021.png`: server crouched mid-bounce with the
ball at its hand, the receiver and partners in the ready stance, the same in both. Before
(`context/b153/mine/c5_6.0.png`): server frozen upright with the ball at the racket.

## Not verified / not 1:1
- The stance clock's phase at a given banner tick isn't compared against the original frame by frame. Our intro
  has no loading hold (the original's clocks sit at 0 through "Now Loading"), and the original's p3 ran 2 ticks
  behind the others on court 4. Presentation only, so it was left as is.
- Only court 4 was recorded and only court 5 compared by screenshot (doubles). Singles takes the same path
  (`motions` is per player), but it wasn't shot.
- The motion picks under the intro are the match's own (`motions`). A human server's pre-serve walk (`serve_turn`)
  can't start during the intro, since input is held.
