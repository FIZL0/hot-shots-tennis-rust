# B71 — tiebreak has no sound

Reported 2026-10-08 ("tiebreak points make no sound") and 2026-10-11 ("Tie break has no sound").

## The original in a tiebreak
Slot 5 (bot doubles, court 10) poked to games 4-3, team 1 at 40-0 (`context/b71/poke.py` → scratch slot 9):
team 1 wins the game to 4-4 and the bots play a whole tiebreak.
- `tools/record_umpire.py 9 9000` → `context/fixtures/umpire_tiebreak.bin`. The umpire announces the game
  (program 2 key 1) and then the tiebreak at the next serve. On every tiebreak point after that she starts the
  call (part 0, programs 0) but plays nothing (calling off). So the original has **no score call on tiebreak
  points**, and that is the only silence.
- `tools/record_sound.py 9 9000` → `context/b71/sound_tb.bin`. The sound ring keeps keying voices all through
  the tiebreak (5–49 key-ons per 300 vsyncs, the same as normal points): rackets, bounces, crowd, BGM.
  (`context/b71/keys.py`, `ump.py`)

## The remaster
- `hst_sim::umpire` already does the same: `call_score` turns calling off for a tiebreak point, and
  `serve` plays the tiebreak announcement. `tests/umpire.rs` can now start mid-match (`run(name, mid)` takes her
  state from the first sample), and the new `umpire_tiebreak` replays the recording: 8936 ticks match the
  original's umpire state tick for tick (14 queued calls, 2 voices).
- New headless test `play/snapshot.rs tiebreak_points_sound`: a singles match with the court, gallery,
  umpire and replay systems, pushed into a tiebreak. Every tiebreak point plays sounds (`Sound::plays` grows).
- Real app (probe build, `--play --singles --games 1 --music`, poked into a tiebreak): mixer peaks of 0.3–1.0
  during tiebreak points, and the music comes back to volume 55 after each point.

I couldn't reproduce the bug on the current main: nothing in `play_sounds`, `Bgm` or the mixer depends on
the tiebreak. It was probably fixed by the sound work after 10-11 (the M2d `Heard`/`Played` dedup and the
mixer changes). The only thing a player can miss in a tiebreak is the umpire's score call, and the original
leaves it out too. No code change; the two tests guard it.

## Not verified / not 1:1
- The doubles and human-vs-bot tiebreaks in the real app were only exercised headless (singles, one human seat on
  autoplay); the original was recorded with bots in doubles. The sound path doesn't depend on the mode.
- Per-sound comparison (which sample, at which tick, at which volume) of tiebreak points against the original:
  not done. The ring recording only shows the original keeps sounding, and court-sound volumes are already
  covered by `tests/sound.rs` on normal points. Presentation SFX only have to feel right (PLAN).
- An instant replay rewinds `Heard.tick` with the snapshot, so its re-run ticks aren't "fresh": the BGM level and
  whistle hold during a replay. This is not tiebreak-specific and wasn't shown to be audible; it was left as is.
