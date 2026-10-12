# B47: NPCs clap with no sound (FINAL)

## Cause
The walkers' clap (animation 5, `npc::cheerers`) runs on every decided point and is proven bit-exact (P14). The
clapping *sound* is the gallery's applause (court bank slot 6, program 0 from stands 0/225/135°). The port only
applauded a plain point for a smash winner (and a game point for a favoured side); the original applauds more
plain points, so on aces, return winners, dives and line-hugging winners the walkers clapped over the plain cheer
(program 10) with no applause.

## The original (scoreboard msg 0x17 handler, gallery start `(level, cheer, event, chain)`)
Plain point (call 0, short of a game): `+0x560` is the scoreboard's named point from the judge (1 ace, 2 return
winner, 3 smash winner, 4 counter; = the finish banner's kind + 1, gated by the hitter's team winning and the
rally call not net-in). Applause `(3, 1, 0, chain)` when:
- `+0x560 == 1` and the server's toss class (player +0x3ea0 through the toss-class table) is strong;
- `+0x560 == 2` or `3`;
- else by the last hitter's branch (the effects manager's per-player record, +0x761): 3 (dive) always, chain when
  (on the line or the shot before was a smash, scoreboard +0x1a1 == 4) and no body hit (0x42305c == −1);
  1/2 (ground/volley) only when on the line (chain 1).
- otherwise `(0, 1, −1, 0)`: the cheer at once.
In every applause case above the chain is "on the line" unless stated.

"On the line" is the same test as the finish banner's *On the Line* (`popups::on_the_line` on the first bounce
mark, serve/rally line distances): its helper returns, for a call-0 point, true when the mark touches or crosses a
line; it is false outright when there was a body hit and the ball made at most one court contact.
Scoreboard +0x19c/+0x198 previous/last hitter, +0x1a1/+0x1a0 previous/last branch (5 at a new point),
+0x1a3/+0x1a2 their counter flags, shifted on each new hit count.

## Port
- `popups::Finish::applause` (the rule; `Finish` already mirrors the scoreboard's shot record and bounce mark):
  new `branches` (last, before; reset to 5 each point) and `strong` (set at the serve contact in `play.rs`).
- `sound::reaction` takes `applause: Option<bool>` (`Some(chain)`); a plain-point applause chains the shout
  (program 5, 0x3c ticks later) as the original. Game points unchanged (favoured side, no chain).
- Test: `play::popups::banner_tests::applause` (each case above). `gallery_matches_the_game` still passes.

## Not verified / not 1:1
- **No recording of the new cases.** The only gallery recording (`hits_s05.bin`) has one smash winner and two plain
  cheers; aces, return winners, dives and on-the-line winners were ported from the decompile only. A check needs a
  sound recording of such points (a human serving aces from slot 3/4); not done in this run.
- The scoreboard's "called branch" (rally block +0x14, `bVar3`) that moves the ace/return kinds one shot on is not
  ported (same gap as the finish banners).
- The last hitter's branch is the port's last strike's branch, not the effects record posted at the contact search
  (they only differ if the last hitter swung again without hitting before the point ended).
