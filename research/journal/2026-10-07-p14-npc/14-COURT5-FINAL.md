# P14d — court 5's own creatures (roster kinds 61..66)

## What they are
Court 5's stand crowds: one object per roster record of type 61..66 (8 on court 5: types 61, 62, 61, 62, 63, 64,
65, 66 in spawn order). Each draws `creatureMdl/wim-0N` (N = type − 61, `COURTSET/05/HOLE01.XB`), a model of
billboard spectators laid over the stands (its vertices sit 8–32 m off the record's anchor), with
`creatureAni/wim-0N.UVA`: four tracks `@mtluva_human00..03`, one per material, 16 keys at 80 ticks a frame that
step a 4×4 texture atlas (key k: column k % 4, row k / 4; row 0 seated, rows 1..3 standing/arms up).

The object (0xc0 bytes) keeps a column `base` = (MT >> 16) & 3, drawn once when made, and a state block: tick
count, on flag, UV frame, level, direction.

- **Made**: one draw for `base`, frame = base (all else 0).
- **Seated again** (state block zeroed, frame = base): on the serve set-up's enter message and the point-over
  state's exit message (so a change of ends seats them too).
- **Stand up** (on = 1, level = ((MT >> 16) & 0x7fff) % 3 + 1, one draw each; tick and direction carry on): the
  intro's enter message only with more than one player, the match-over enter message always, and the point-over
  enter message unless an instant replay was picked for the point, and then only for a game/set/match won or a
  call other than 1..4, 6 (faults, lets, net).
- **Tick** (after the frame's messages): first hands the current frame to the model's UV player; then while on,
  on even ticks the level ping-pongs (down: < 2 → turn up, else −1; up: < 3 → +1, else turn down) and frame =
  base + 4·level; the tick count goes up every on-tick.
- **Draw**: types 63 and 66 skip their draw when a point in front of them (0, −8, ∓49) fails the visibility test.

## Port
- `hst_sim::npc::Crowd` (`new`, `reset`, `cheer`, `step`) and `npc::crowd_cheers_point`.
- `play/npcs.rs`: the creatures are made in spawn order (their `base` draws interleave with the walkers' and
  trigger creatures' draws on the court generator as the game's do), stand up after setup with > 1 player, at a
  decided point per `crowd_cheers_point` (the app's instant-replay decision), seated at the first serve, each new
  point and a change of ends, stepped each tick; `crowd_model` spawns each model under the game space at its record's
  matrix and `draw_crowd` sets each material's UV offset to its track's key at the creature's frame.

## Proof
`research/p14d_record.py` (drives slot 2's menu to a doubles match on court 5, P1 pressing ○ every 80 samples):
`context/p14d/run1/c5.bin`, 5000 frames from the intro, 6 of the creatures (the recorder stopped looking once it had
six), the phase, instant-replay byte, result kind, call and MT each frame. `research/p14d_dump.py` prints it.
`tests/npc.rs court5_crowd_like_the_game`: every consecutive pair of samples, from the earlier sample's state, the
port reproduces all six creatures' tick, on, frame, level and direction bit for bit (9444 creature ticks; the
phase's message comes before the tick; a frame with two game ticks steps twice; frames the objects weren't ticked
are skipped), with 8 stand-ups at point-over entries (calls 0 and 5, a game won) each drawing consecutive MT words in
creature order, and 10 resets (serve set-ups and point-over exits into a change of ends or a set-up).

Remaster shots (`context/p14d/shots/c5_side_2.png`, `c5_side_2.05.png`, HST_EYE through an HST_BIN wrapper): the
side stands show the billboards switching poses between the two shots during the intro cheer.

## Not verified / not 1:1
- Match-over stand-up (the match-over state's enter message): the recording didn't reach a match end, and the app
  doesn't play the match-over state (its decided match point does stand them up, as the point-over message would).
- The instant-replay-suppressed case and the fault/let/net calls (1..4, 6): none happened in the recording; ported
  from the decompile only.
- Types 65 and 66 and the construction draws: the recorder captured six objects after they were made; the base
  values of all six matched what the model had, but the draws at construction (and the spawn count of 8) weren't
  compared against an MT sequence.
- The order of the stand-up draws among other objects' draws on the same message (the walkers' start cheer,
  the trigger creatures' reaction, the gallery): the original broadcasts through its object tree; the app draws them
  after the walkers and trigger creatures. Not recorded together.
- The drawn frame: the original hands the frame to the model at the start of the creature's tick (one tick behind
  its state); the app draws the state after the tick. Rendering only.
- Types 63/66's visibility skip is not ported (rendering only; the stand is simply drawn).
- No side-by-side with the original: `tools/screenshot.sh` can't reach copy 1's window since the desktop session
  restarted (its Hyprland instance has no windows); judged from the remaster shots and the atlas alone.
