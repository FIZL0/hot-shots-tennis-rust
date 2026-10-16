# B483 — online screens on the graph-paper wall

## Finding
Release `--shot`s (`HST_MENU=online|lobbies|code|lobby|lobby-hosting|lobby-failed|settings`, in `context/b483/run1`):
Online, Find (Lobbies), Join by Code, the name/relay keyboard and the lobby itself already draw sheet 5
(`wall_playerselect`, the rebuilt graph paper `look::backdrop` uses); their wall lines sit on the same pixels as
Settings'. The odd one out was the screen between choosing Host/Quickplay/Join and the lobby showing up
("Creating lobby...", "Finding a lobby...", "Joining...", and "Couldn't join the lobby"): `d.frame(6, 8)` drew
sheet 8, the green `wall_confirmation` tiles.

## Change
`online.rs`: that screen now uses `d.frame(6, 5)`. It keeps the green "Lobby" title band (matching the lobby's
green), on the same lavender paper as Settings, Mods and the lobby. After: `context/b483/run2/lobby-{hosting,failed}.png`.
The "Leave lobby" pill keeps its pale green fill, which matches the band and reads fine on the paper.

## Seen, not changed
Find and Join by Code still show the "Main Menu" title band (`frame(0, 5)`), not "Online". The task only covers
the background, so this is follow-up task B495.

## Not verified / not 1:1
- None: front-end screens are ours (PLAN.md); checked by eye in screenshots.
