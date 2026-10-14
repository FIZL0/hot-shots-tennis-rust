# B269 — the next COM's cursor starts on the last pick

- `Menu::last`: the roster cell (Random card included) the last pick readied on, set whenever a player readies.
- Local: after a ready, `carry_pick` puts the computer player `driven()` moves on to (one 1P added, `bots[q]`)
  on that cell; a character lands on its next free costume (`skip_taken`). Open seats' COM cards (B258) keep their
  automatic pick.
- Online: `edit_bot` for a newly added computer seat opens its cursor on `last` (the lobby still holds its B217
  random pick until the host readies it).
- Tests: `the_next_com_starts_on_the_last_pick` (3 COMs in doubles: Random ✕✕✕✕ → all Random; a character carries
  on to the next costume), `an_added_bot_starts_on_the_last_pick` (online).

## Not verified / not 1:1
- Front-end behaviour (ours to design, PLAN.md); not compared with the original. `last` lives for the session,
  so the first COM of a later match also starts on the previous match's last pick — intended ("where the last pick was").
