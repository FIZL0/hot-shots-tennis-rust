# B145 — right-stick emotes (addition)

Ours, layered on top: `crates/hst/src/play/emote.rs`.

## What it does

- A human flicks the right stick fully (length ≥ 0.9, the dominant axis) up / down / left / right, or presses keys
  1 / 2 / 3 / 4 (`controls.txt` actions `emote_up` … `emote_right`, rebindable): their player plays one of four
  motions with its voice line.
- The flick is match input: the wire's right-stick bytes (`Pad::rx/ry`) carry only it — one axis at its end, the other
  at 0x80 (`tick_input::FLICKS`); a new flick direction is the press (`decode`). So instant replays, lockstep and
  rollback netplay see it like any button. PCSX2 recordings' right sticks (which the original ignores, but which the
  recordings do carry: round1.bin has 12 frames that would read as flicks) are centred by `tick_input::recorded` in
  the app tests that feed recordings.
- When it may start: in a rally, or while the opponent serves (never the server in the serve phase), with nothing
  under way (no press, swing, whiff, dive, serve follow-through, reaction, body hit).
- While it plays (`ceil(motion length)` ticks, the motion as the game-side clip `Game::data` holds it, so online
  peers agree): `control` skips the player's turn — no run, no aim, no press (a press is dropped, not queued).
  The body stops (run velocity zeroed). After it, the next tick runs and swings normally.
- **The ball while emoting**: it can't be hit — it goes by, and the point is played out as the original would
  (a winner, or a body hit if it strikes the player). The point's end (any phase other than rally/serve) clears
  the emote, so the reaction plays as always.
- Voice: the reaction's own voice program (`sound::reaction_voice`, as a winner), key picked from the player's
  position bits, no key memory: nothing draws from the match's generators (`g.rng`), nothing changes the real
  reactions' key memory. Three-player matches have no reaction voices, so there the emotes are silent.
- The Settings → Controls screen (and the pause menu's) now lists 16 rows: its pitch went from 28 to 21 (text 19)
  and it starts at y 62 so `back` clears the info bar (checked with a `--shot`, `HST_MENU=controls`, in
  `context/b145/controls2.png`).
- Bots never emote: only the human branch of `control` reads it.
- Untouched stick: the wire's right stick stays 0x80/0x80, `emote::tick` returns false, the match is the same as
  before (the replay tests are untouched; no sim code changed).

## The four, per character

Every character has the same four post-point reaction motions on the disc (the match's motion table,
`hst_data::ani::MOTIONS`), each in the character's own `PCnnANI.XB`, so the four are each character's own:

| Flick / key | Motion | Voice (singles) |
|---|---|---|
| up / 1 | `re_pcNN_gu_set` 0x2e (big win, the set/match celebration) | program 9, keys 0–1 |
| down / 2 | `re_pcNN_di_set` 0x2f (big loss) | program 10, keys 0–1 |
| left / 3 | `re_pcNN_gu` 0x2c (point won, joy) | program 7, keys 1–2 |
| right / 4 | `re_pcNN_di` 0x2d (point lost, anger/frustration) | program 8, keys 0–1 |

Ashley, Cody, Jun, Kaito, Momoko, JJ, Carol, Big Chief, Miranda, Kent, Lola, Will, Gloria, Suzuki: all as the table
(pc00..pc13). In doubles the voice keys are the doubles reaction keys (9, 10, 8: key 0; 7: 0–1).
A mod character plays its pack's clips for these motion numbers and its pack's voice bank; a pack missing a clip
just doesn't emote in that direction. Online with a mod, the lock length comes from the donor's game-side clip (all
peers agree), the drawn clip is the pack's.

## Test

`play::emote::tests` (flick classification, the flick over the wire and back once) and
`play::emote::match_tests::an_emote_locks_then_plays_on`: the app's singles match from the ISO, 1P holding the stick
left, flicks up right after 2P's serve: for the motion's length 1P doesn't move, plays 0x2e and a ✕ press does
nothing; after it 1P runs again and a ✕ press swings.

## Not verified / not 1:1

- Everything here is an addition with no original to compare to; nothing checked against PCSX2.
- The emote plays in place: `gu_set`'s root path (which carries the player in the real reaction) isn't applied.
- The voice key is picked from position bits rather than the game's random draw (which would touch the sim RNG).
- With the free camera on, flicking the right stick left/right also turns the camera while it emotes.
- Not looked at by eye in the running remaster (no `--shot` taken): the motions are the reactions already drawn
  after every point.
