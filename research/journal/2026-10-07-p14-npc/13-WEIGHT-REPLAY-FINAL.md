# P14c3f — trigger weight, msg 0xe keep/restore, +0x280

## Model weight (`Trigger::weight`)

- The startle callback (types 0, 1, 5, 31, 32, 39) writes the weight at a reset or the match start. Type 32 gets 1.0
  and its `on` set; the others get 0.0. The weight's address is the model data's +0x5c: inst = **(+0x5c),
  data = *(inst+0xc).
- **Recorded type 32** (court 8, trg_BA-02). The script is `research/p14c3f_court8.py start`: from slot 2 it goes to
  Ashley vs the COMs, court 8, doubles, and saves slot 8 at P1's first serve. On copy 4, ✕ confirms.
  - Both type-32 creatures: weight 1.0, +0x281 = 1.
  - Both type-31 creatures: weight 0.0, +0x281 = 0.
  - Types 33 and 34 keep the model default of 1.0, because their callbacks never write it.
- **What the factor does.** The model draw multiplies the weight by each material record's float factor. Every
  factor on these models is 1.0. The per-packet draw then stores the product as the w of the packet's first VU1
  quad (+0x10..+0x1c) and uploads it with the next quads (+0x20..+0x3c). It does this only when the packet's draw
  mode is not 0:
  - mode 1: the model's bones have runtime flags & 0x500;
  - mode 2: the instance's projection-pass flag together with the batch header's bit 4.
  - Mode 0 packets never see the weight.
- **Draw modes found** (`research/p14c3f_weight_modes.py 8`, computed the same way the draw does):

  | type | mode | packets |
  |---|---|---|
  | 31 | 3 | 12 |
  | 32 | 1 | 9 |
  | 34 | 1 | 4 |
  | 33 | 0 | – |

- **Packet quads in RAM:**
  - Type 31: every quad is 0, with w = 0.
  - Types 32 and 34: (0, 0, 0, 1.0), with zeros after.
- **Conclusion.** The weight only reaches the VU1 microprogram, as a per-sub-mesh scalar. This is rendering. The
  port keeps it as state (`Trigger::weight`, which was already ported and is unchanged). How the creature draw
  should use it is left to the eye check below.

## Msg 0xe: keeping and restoring `done` (+0xbc via +0xbd)

- Msg 0xe is the message the match sends at each serve set-up and when a replay starts.
- **Not replaying** (match +0x344 clear), it keeps four things:
  - +0xbc into +0xbd;
  - the drawn matrix (+0x1f0, which the step copies from +0x1b0 each tick) into +0xe0;
  - node, previous node and leg ticks left (+0x130/2/6) into +0x120/2/4;
  - pause left (+0x13e) into +0xda.
- **Replaying**, it puts them back. The matrix goes back into +0x1b0.
- **Then a reset follows**, unless:
  - the type is 37, which is never reset; or
  - the match phase is 0 or 1 (start, change of ends), in which case only type 15 is reset.
- Everything kept sits below +0x128, so a reset doesn't clear it. The restored matrix and place survive only where no
  reset follows.
- **Ported** as `Trigger::serve_setup(row, replaying, phase, …)`, with the new fields `kept_done` and `kept`. Resets
  carry both fields over.
- **Recording.** `tools/record_npc.py` gained three opt-ins: `HST_GM=1` (log the match object), `HST_BYTE` (byte
  pokes into a creature) and `HST_REPLAY` (force the instant replay). The recording:

  ```
  HST_GM=1 HST_REPLAY=1 HST_BYTE="10:0:0xbc:1;500:0:0xbc:0;500:2:0xbc:1" \
    tools/pcsx2.sh tools/record_npc.py 5 1400 context/fixtures/trig_replay_s05.bin trig
  ```

  It shows three 0xe arrivals:

  | arrival | replaying | phase | what happened |
  |---|---|---|---|
  | match's first serve | no | 0 | creature 0 kept bd = 1 |
  | forced replay's start | yes | 4 | creature 0's bc 0→1; creature 2's bc 1→0 |
  | serve after the replay | no | 4 | kept again |

- **Whether a reset ran.** `research/p14c3f_msg_e.py` prints the transitions. Byte diffs of +0x128..+0x290 across
  each arrival show:
  - phase 0: only +0x283 changes, so no reset ran;
  - phases 3 and 4: the matrix and leg fields change, so a reset ran.
- **Test.** `crates/hst-sim/tests/npc.rs` `serve_setup_keeps_and_restores_like_the_game` runs the port on every
  creature from the sample before each arrival. `done`, `kept_done`, `active` and the kept matrix and place all match
  the sample after it.
- **The app.** `play/npcs.rs` is unchanged. The app's instant replay (`play/instant_replay.rs`) restores the whole
  `Npcs` snapshot taken at the serve set-up. For every type except 37, that is the same as the game's restore plus
  the reset that follows. Replays only have to feel right (PLAN).

## +0x280

- A byte scan of the main program and the game overlay found no byte load at offset 0x280. Its only accesses are
  stores:
  - the motion reset writes 1 if the row ground-snaps, else 0;
  - the startle callback writes 0.
- Nothing reads it, so it is deliberately not ported. In the court 8 RAM it is 0 on every creature.

## Not verified / not 1:1

- **The weight's visual effect.** No screenshot was possible this session: `hyprctl clients` listed no windows at
  all (no PCSX2 copy, no other window), so neither `tools/screenshot.sh` nor an A/B poke of the weight could be
  captured. The VU1 microprogram's use of the w wasn't disassembled. The app's creature draw does not use the weight.
  This is a rendering item: an eye check whenever screenshots work again.
- **Restoring the matrix and place without a reset.** This only happens for type 37 (never reset) or at a replay
  during phase 0/1, which never occurs. Court 10 has no type 37, so in the recording every restore was followed by a
  reset. The save half of the matrix and place is checked bit-exact.
- **The reset decision.** It was checked through the byte diffs and `active`. Every court-10 type rearms, so `active`
  stays 1 either way. No non-rearming creature with `done` set was recorded across a reset.
- **Handler messages 6 / 0x1b** (clear +0xbc/+0xbd/+0x282/+0xd9) and **0x1a** (reset all but 37) are not ported. A
  new match builds fresh triggers in the port.
- **The +0x283 flag** (1 after 0xe, 0 after 0x19/0x1a) is not ported. Three type callbacks read it; it is outside
  this task.
- **The app still uses `reset_near`** at the serve, not `serve_setup`, and its replay restores the whole snapshot, so
  type 37's post-point state is restored too (the game keeps it). That is in-match presentation.
