# P11i: bot matches, serve and kind lock call by call (final)

Tool `tools/record_ai_serve.py <slot> <frames> <out>`. It patches the four AI routines below with jumps to stubs in
free RAM (0x1e00000..). Each stub records the call's entry and exit, along with the AI generator's draw count, which
a hook on the MT draw keeps; the first draw also snapshots the generator. The replay test therefore feeds each port
function the exact generator the game had and checks what the call left. Test file:
`crates/hst-sim/tests/ai_serve.rs`.

| Tag | Routine | Kept when |
|---|---|---|
| 1 | singles kind lock 0x364100 | it draws, or it is the frame before the contact (player +0x3ec4 == 1) |
| 2 | singles serve state 0x3c8f10 | it draws, it is the frame before the contact, or the state byte AI+0x55 changes |
| 3 | doubles serve state 0x3cee70 | same as tag 2 |
| 4 | serve contact pick 0x360010 | it found a frame; the record carries the AI's ball path 0x424e90 |

## Matches

- **Singles**, `context/fixtures/ai_serve_singles.bin`: from the save state of
  `context/recordings/bots_singles.p2m2` (copied to slot 8; the recording has no input), 7000 frames.
  - P0 is Carol (character 6, left-handed); P1 is Lola (character 10).
  - Both wear costume 9 (block 3, the hardest), so they use AIParam rows 48 and 52, level 3.
  - Calls: 65 button locks, 57 stick locks, 8 serves (toss, swing, aim each), 8 picks (3 of them quick).
- **Doubles**, `context/fixtures/ai_serve_s05.bin`: slot 5, 7000 frames.
  - Characters 0 and 2 against 1 and 5, all in costume 0: rows 84 and 86 against 85 and 89, level 3 (P11a).
  - Calls: 9 serves, with serve levels 0 and 1, one underhand toss (△ swing), and no quick serve.
- Neither match had a second serve, so the toss's second-serve draw is checked only through the first-serve path.

Every checked call matches bit for bit, including the draw counts.

## Kind lock

`lock_button` / `lock_stick` were already right.
- The stick lock reads the button kept at AI+0x2c after this frame's button chain.
- With no button, it runs only on the frame before the contact.

## Serve

- **Toss, swing and the doubles aim** were already right.
- **Fix:** the level-2 lane and depth draws were cut to a byte before `% 3`. The game takes `% 3` of the 15-bit draw,
  so these draws were wrong.
- **Singles aim** (new: `AiParams::serve_aim_singles`):
  - At level 2, a serve sends a NET player in after it (ALL style: an 80% draw) when it goes down either of two lanes:
    - a wide ✕ from lane 0 on the deuce court, or lane 2 on the ad court;
    - any other swing from lane 2 on the deuce court, or lane 0 on the ad court.
  - The dash spot:
    - x = madd(0, or 1.3716666 for a non-✕ swing, 1.3716666, 2.3283064e-10 · draw), then × (ad ? +1 : −1) × side;
    - z = −reach × side.
    - These are stored at AI+0x70 / +0x78, and the dash flag at AI+0x256.
  - Then every singles serve draws `net_dash_rate`, which also sets the flag.
  - All of this happens before the aim's own draws.
  - Neither match had a serve that set off the dash, so the spot formula comes from the decompile and the disassembly
    only. The dash flag itself (unset in every call) is checked.
- **Contact pick:** the game reads the ball's vertical speed on its path, not the next frame's height.
  - Quick serve takes speed ≥ 0, otherwise ≤ 0; an underhand toss is never quick.
  - The answer is the frame nearest the ideal height inside [low, top], the first one on a tie, and nothing from the
    first bounce on.
  - The index is absolute: it counts from the path buffer's start, and the scan runs from AI+0x30 to AI+0x3c.
  - New: `serve::ai_pick`. `ai_search` now calls it, with the speed taken as the next frame's rise.

## Return-of-serve chooser

- The singles receive chooser 0x3cc700 never ran in bot-only singles (8000 frames under `record_ai_aim.py`, about ten
  serves).
- A bot match can't cover it, because it only runs against a human server. It stays unchecked: recording it needs a
  human serve, driven with vpad on slot 3 or 4.

## In the app

- `ai_serve_kind` uses `serve_aim_singles` in singles and keeps the dash in `Player::ai_serve_dash`.
- `ai_wait_singles` puts it in place after the bot's own serve; this replaces the old `net_dash_rate` roll at shot 1.

**Ponytail:** the aim, and with it the dash draw, is still drawn at the swing press, not on the frame before the
contact.
