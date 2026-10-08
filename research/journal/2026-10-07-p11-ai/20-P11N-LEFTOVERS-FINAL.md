# 20 — P11n leftovers

With the ported routines running receive and rally (`doubles_ai::step`, singles and doubles), most of P11n's list
was the stand-in's and is now the routines' own:

- rally aim and dash on the frame before the contact: the routines draw the aim at `swing + lead < 9` and output the
  stick at `swing == 1`; the singles kind lock now runs on the routine's button (`lock_button`) and at the contact
  (`lock_stick`).
- run boost after a right guess: the receive's push (+0x24e), bit-exact (`ai_rally_s05_slow*.bin`,
  `ai_rally_singles_spec.bin`).
- ALL coin: `Mind::rally`'s counter (+0x258/+0x26c), in the routines' set_sub(0).
- shot choice 10's deeper dash: `Rally::dash_spot`.
- `ai_wait` from the partner's shot record and volley hold: the doubles NET/BASE substate 0.
- `lets_go` stops the run: the routines' line-margin hold (its inputs, path mode/gap, are P11k6/P11l6).
- give-way voice: 0x3d5b20 plays program 6 key 3 + ((roll >> 16) & 1) on the AI's player when the partner's
  +0x3fa5 < 2 and chance(25), on the AI generator 0x427130. `Rally::said` keeps the bit; `step` plays
  `sound::call_out(i, bit)`.
- fresh `Mind` per match: the app builds a `Game` per match, so `ai_mind` starts None and `Mind::reset(first)`.
- `ai_aim`'s fields: the routines build their own `Look` (`shot_aim`, `aim`); `ai_aim` is left only for the
  stand-in.

New in the app: the serve aim (and the singles serve dash) is drawn on the frame before the contact
(`ai_serve_aim`, at `left == 2` in `bot_serve`; at the press when the app's serve search puts the contact a frame
away). The swing is drawn at the press (`ai_serve_kind`, kept in `Player::ai_serve_swing`). Checked in a 60 s
singles run: every serve aims at left 2 except a contact 1 frame out.
