# B210 — every celebration a character has

## What the disc has (per character 0–13)
- `AZUMA/PRIZE/ANI/WINANI.XB`: one win motion `win_XX` (`_n` for 6 and 11), the doubles hand slaps `hand_*`.
- `AZUMA/PRIZE/ANI/LOSTANI.XB`: two lost motions `lost_XXa`, `lost_XXb` (`_n` for 6 and 11).
- `SND/VOICE/PRIZE/PC_HVXX.XB0` (and `…B.XB0`, the same samples byte for byte): 2 programs (0 win, 1 lost),
  2 keys (lines) each.
- In-match point reactions (`PCANI/PCXXANI.XB`): one each of `re_pcXX_gu`, `gu_set`, `di`, `di_set`, plus the
  shared doubles team reactions co01–co05. These are picked by the sim (`motion::reaction`/`team_reaction`, from
  the shared generator, and they move the players), so they stay as they are.

## How it was chosen
The port did what the original does: the lost pair is fixed by character (`LOST_B`, a partner takes the other one)
and the voice line is one bit of a copy of the sound generator. So a character always lost with the same motion.

## Now (`play/match_stats/ceremony.rs`)
`pick(char, kind, n)`: a ceremony-only xorshift (seeded from the clock once, never the sim's generators), never the
same as that character's last pick of that kind in this run. Used for the beaten side's lost pair (`vary_lost`; a
doubles partner still takes the other pair) and for the prize voice line (win or lost program). The one win motion
has no alternative on the disc. Test `celebrations_take_turns`.

Live: `hst --play --singles --games 1 --pads -,- --chars 3,0` logged
`winners' ceremony: [[win_03, hand_03l, lost_03a], [win_00, hand_00l, lost_00a]]` and `ceremony voice: character 3
program 0 line 0` (log in `context/b210/run1.log`).

## Not verified / not 1:1
- Not 1:1 on purpose (the task's request): the original's fixed lost pair and its sound-generator voice bit are
  replaced by turn-taking picks. The sound generator copy is no longer drawn here (it was a copy, the sim never
  sees it).
- Not watched over several matches in one run: Continue needs input, so each unattended run had one ceremony. The
  turn-taking is covered by the unit test, not seen live.
- Mods: a mod character has no ceremony motions or prize voices of its own (it uses stock character 13's), so there
  is nothing more to pick for them.
- In-match reactions: one of each per character on the disc, chosen by the sim; left as they are.
