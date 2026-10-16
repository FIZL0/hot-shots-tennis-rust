# 1 — the original never marks your own lob; port matches (FINAL)

## The original's rule (decompile)
The marker object's search setup, run on every hit message, empties the candidate list. It then picks a team:
- In a match, the hitter's slot `& 1` indexes the table `[1, 3, 0, 2]`. An even hitter gives slots 1, 3; an odd
  hitter gives slots 0, 2. So the candidates are always the **receiving** team.
- In practice (`players == 1`), the team depends on whether the ball is on the player's side.

A slot is a candidate only if the object's per-slot character table (`+400`, built from the controller index) holds
a character id. CPU slots hold −1. So a candidate is a human on the receiving team, and nothing else feeds it. Your
own lob, or your CPU partner's, has no candidate and is never searched. Shot type, lob height and character don't
matter. Sophie's lob records can't change this.

## The original, live (`research/b507_lob_look.py`, slot 4: doubles, 1P human + CPU partner, table `[6, −1, −1, −1]`)
Logged the hitter variable and the candidate count at every shot (`context/b507/r1`–`r4`). In r3, holding △ after
each CPU shot made 1P actually return in rallies:
- Every shot by slot 1 or 3 (CPU, toward 1P's team) set up 1 candidate (1P).
- Every shot by slot 0 (1P) or 2 (1P's CPU partner) set up 0: serves, and 1P's rally return (r3 hit 8, r4 hit 8).
  No placement, so nothing was drawn.
- The hitter variable is already the current hitter when the setup runs, not the previous one.

## Port
`strike` (play.rs) filters the receivers the same way: `i & 1 != who & 1 && humans[i]`. Nothing changed in the
game code.

New test, `crates/hst-play/src/play/smash_marker.rs` `own_lobs_show_no_marker`: a real human seat (1P) driven only
by △ presses (✕ to serve), against a bot. The human lobs back on its own:
- 11 shots in 2 minutes, 5 of them lobs. None of 1P's strikes sets up a search.
- Every opponent shot from the return on sets one up, and an opponent lob must place its marker before it bounces.
  The bot didn't lob in that window; B12's `lob_places_its_own_marker` covers that case.
- The shared match set-up became `bot_match(seats)`.

## Conclusion
This is not a bug. The yellow marker shows where *you* can smash a ball coming at you, and the original never
shows it on your own lobs. The lobs that "come over without it" are the user's own. Showing one there would be an
addition beyond the original, which only the user can ask for.

## Not verified / not 1:1
- No screenshot of the original during 1P's own lob. PCSX2's pad-bound screenshot hotkey queued up during the
  scripted run, and all the screenshots landed at once afterwards. A state saved 30 frames into 1P's return loaded
  after the point had ended. The RAM evidence is enough here: placed-point count 0 means the marker isn't drawn.
- Not recorded with Sophie in the original (slot 4 is Carol). The rule never reads the hitter's character.
- The port's test uses characters 6 and 1, not Sophie, for the same reason.
- Practice mode's side pick is still not ported (the port has no practice mode). Same as before.
