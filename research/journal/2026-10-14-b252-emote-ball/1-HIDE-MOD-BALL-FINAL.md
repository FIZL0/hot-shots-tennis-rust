# B252 — the ball while the server emotes before its toss

## What was there

`held_ball` puts the held ball at `Bip01LFinger21` of the server's current motion — during an emote the emote's
clip, sampled at the drawn time. For a disc character that is the clip that's drawn, so the ball stays in the hand:
seen in `--shot`s of Ryo's `gu` emote (char 6, `context/b252/run2/`) the ball rides the raised hand the whole emote.
(Sim check: ball 0.21 m from `Bip01LFinger21`'s hand bone through the emote vs 0.08–0.16 m in the stance.)

A mod with its own clips (standard §6 `motions`, `CharacterData::visual`) draws its own emote, but the ball is
placed on the donor's clip (the sim stays the donor's), so it floats off the drawn hand.

## Fix (per the user's note: the disc characters' ball must not disappear)

`play/emote.rs` `hide_ball` (Update, after `draw`; one line in `play::plugin`): while the server emotes in
`Phase::Serve` and its emote motion is drawn from the mod's own clip, the ball, its shadow and its outline
(`BallView` / `shade::Ball`) are `Hidden`; when the emote ends they go back to `Inherited`, so the ball is in hand
for the toss. Visibility is only written on a change, so the ceremony's hiding stays. Disc characters, and mods
without their own emote clip, keep the ball shown in hand. Draw-side only: no sim state, no generator draws.

`main.rs` `HST_KEYS` also takes `Digit1`–`Digit4` (the default emote keys), for `--shot`s of an emote.

## Tests

- `play::emote::match_tests::a_mods_ball_hides_while_it_emotes`: the 1P server emotes; with the disc data the
  ball is never hidden, with the emote clip copied into `visual` it is hidden exactly for the emote's ticks.
- `--shot`s: Phoebe (`fore_pc00_phoebe`, own clips) in slot 0 — no ball through the emote, back in hand after
  (`context/b252/run3/`).

## Not verified / not 1:1

- Emotes are ours (B145): nothing to compare against PCSX2.
- The emote's last tick (counter at 0) already shows the ball while the mod's clip shows its last frame: one frame.
- Not checked online with two instances; the system reads only the synced `Game`, so both sides hide alike.
