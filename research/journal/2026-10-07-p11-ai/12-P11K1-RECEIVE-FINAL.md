# P11k1: the doubles receive routine (final)

The tool `tools/record_ai_rally.py <slot> <frames> <out> [keep 1 in N] [tags] [speed scale]` hooks the receive 0x3cf7b0
(tag 1), NET 0x3d02c0 (tag 2) and BASE 0x3d1320 (tag 3), plus the AI generator's draw counter.
- Records are 0x780 bytes plus a tail. The record layout is in the tool's docstring.
  - The entry tail is the path object's live entries (start..end, at most 180 x 0x30).
  - The exit tail is the AI's path copy (0x424e90, ai+0x3c entries).
- The 6th argument scales every player's speed stat (player+0x1374) after the load. Bot-only slot 5 almost never
  misses a ball, so slowed players are what make the receive's fallbacks run.

The test `crates/hst-sim/tests/ai_rally.rs` replays every tag-1 call through `rally::Rally::receive`. It compares:
- the AI object's 0x280 bytes (the port's fields written over the entry bytes);
- the run target and stick, the button, and the return;
- the path copy: stamp, count, and its entries when the call refreshed it;
- the seen flags, the ball-lost state and log, and the draw count.

The copy's entries are not recorded on entry. When the copy is taken this frame they are the path's first `count`; an
older copy only shows its count.

| Fixture | Frames | Speed | Calls | Substates 0/1/2/3 on entry |
|---|---|---|---|---|
| `ai_rally_s05.bin` | short | 1 | 663 | 504/25/54/80 |
| `ai_rally_s05_all.bin` (tags 1-3) | ~7800 | 1 | 2987 | 2299/168/249/271 |
| `ai_rally_s05_long.bin` | 25000 | 1 | 11307 | 9209/678/540/880 |
| `ai_rally_s05_slow.bin` | 12000 | 0.4 | 6513 | 4902/852/194/565 |
| `ai_rally_s05_slow2.bin` | 20000 | 0.25 | 12865 | 10532/1076/212/1045 |

All slot 5, bot-only doubles. Every call matches bit for bit.

Coverage (branch counters over all fixtures):
- fresh searches 101 (90 found), dive 6;
- fallback walks after a failed or stale search 433 (after-bounce pick 375, entry 11 pick 58);
- guess section 27 (3 wrong guesses, so the ball-lost log);
- aim branches sure/wide/centre 55/12/3;
- stroke-frame pickers kinds 0 and 1.

Never reached: the landing pick (361af0, needs a path that ends before its second bounce), the last-entry pick
(361eb0), the smash search finding a ball, the out-of-court aims (plans 10/11 from |x| > 5.485), the push after a right
guess (+0x24e), the volley/smash pickers (kinds 2/3 only come from NET/BASE). These are ported from the asm and tracked
as P11k4.

## The port (`hst_sim::rally`)

Types: `Rally` (the AI's rally fields), `Body` (its player), `World` (the match and the live path), `PathCopy` (the
shared copy and its frame stamp), `Out` (stick and button).

- **Entry.** Frames, index and swing count down every call.
- **Substate 0.**
  - Held: nothing.
  - Guess wait > 0: walk toward the guessed sideline (x = +-5.485 * side by court and guess, z kept); the wait is
    cleared when the walk arrives.
  - Otherwise the path copy, which is skipped when the ball isn't coming its way: no hitter, own team's hit, the serve
    before its return for a non-receiver, an unsettled hit on its own half.
  - When the copy is fresh and the path doesn't end short: reach search (style 2), the tiered search tiers (from 3 with
    the low flag), then the smash search (kinds 1/0/3). A find goes to substate 1 (or resolves the guess).
  - No find: landing pick (path ends short) / dive check (dive flag, not after the point) / the fallback picks (after
    the first bounce's rise, entry 11, the last entry when it ends on the other half), walking there.
- **Guess resolution.** Dot of the guessed side with the direction from the guess walk's start to the stand spot.
  - >= 0.5: right (push flag).
  - < 0: wrong. Ball-lost log for a computer player, stick zeroed, wait = row guess[2], back to substate 0.
- **Substate 1.** Walk to the stand spot until the run frames reach 1 and the ready check says the ball is in reach (or
  0 frames). Then pick the swing frame by kind (35f450 / 35f660 tiers / 35f970 / 35fb60; 8 when none) and go to
  substate 2 (lead by kind; random -1..1 after a right guess).
- **Substate 2.** When swing + lead < 9: the return aim (plans 7/8/10/11, lane and depth by level tables 50/60/70/80
  and 55/67/80/95) and the plan's button, then substate 3.
- **Substate 3.** At swing 1 the stick (x1.5/x2 and normalised after a right guess). Returns true once the stroke is
  over. Counts down the guess wait while the opponents have the ball.
