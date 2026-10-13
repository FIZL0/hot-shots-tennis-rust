# B125 — P1 tossed the serve by itself at match start

## Cause
The court intro holds the fixed ticks (`play/intro.rs` `hold`), but `read_input` (Update) keeps latching presses
into `Pads` all the while. The intro skips on Enter, Space or J / pad ✕. Space is the serve key and J / ✕ are
topspin, so the press that skipped the intro (or any shot or serve press during it) stayed latched. The match's
first fixed tick then spent it as the human server's toss press (`serve_turn` → strong toss). No swing followed,
because the swing needs a second press. Not `auto_shot`, the bot path or the menu's confirm (a match starts with a fresh
`Pads`; the menu's last press is spent frames before the match is built).

## Fix
`hold` drops every slot's latched shot/serve press each frame while the intro runs, including the frame it ends
on, before the fixed ticks are let go. This is what the pause screen already does (`menu.rs`). `HST_KEYS` can now press Space too.

## Verified
A temporary stderr probe at the toss press and the intro skip (not committed). Run: `--play --singles --court 4`,
`HST_INTRO=1 HST_KEYS="3:Space"`, P1 serving:
- before: `intro skipped`, then `toss player 0 human Some(true)` with nothing pressed after;
- after: `intro skipped`, no toss up to the 9 s shot.

## Not verified / not 1:1
- Not checked against PCSX2. The original doesn't read the match pad during its intro, so it can't toss early. The
  remaster now matches that by dropping the presses. No recording was needed for a stale-input bug.
- Only checked with the keyboard (Space). Pad ✕ and J go through the same latch, so the same clear covers them.
