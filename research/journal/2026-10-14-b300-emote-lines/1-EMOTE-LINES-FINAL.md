# B300 — emotes vary their voice line

Cause: `emote::voice` picked the reaction key from the player's position bits with no key memory; an emoting
player stands still, so each direction kept landing on the same key.

Fix (`play/emote.rs`): the emote line takes the singles reaction keys (`sound::reaction_voice(2, …)`: up 9/0–1,
down 10/0–1, left 7/1–2, right 8/0–1) whatever the player count (doubles' reactions ask for one key only, the bank
has all of them), and remembers its last key in `sound::Voice` slot 2 — free: shouts use 0/1, reactions 3–5. With
two keys per emote it alternates. The memory is part of `Game` (snapshots carry it), so peers pick alike; the
match generators aren't touched, so replays stay bit-exact. A mod's wav takes wrap by key (`audio.rs`), so a pack
with ≥2 takes per program alternates too.

Test: `emote::tests::emote_lines_vary` — four emotes of each direction never repeat a line back to back, and the
real reactions' key choice is unchanged afterwards.

## Not verified / not 1:1
- Emotes are ours (B145), no original to match. Not listened to in-app (no audio check unattended); the test
  checks the keys asked for.
- A mod pack with more than two takes for a program only uses its first two (the singles key range); widen the
  range per pack if wanted.
