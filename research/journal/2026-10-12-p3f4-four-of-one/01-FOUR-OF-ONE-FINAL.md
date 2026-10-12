# Four of one character

`research/p3f4_four_of_one.py <slot> cross [ms]` loads save slot 2 (doubles character select), walks all four
cursors onto character 1 (P1 right, 2P left, 3P stays, 4P left left), confirms each and saves the first serve. On
PCSX2 copy 4 ✕ confirms (○ backs out to the main menu, as on copy 3). The extra wait before the first press shifts
the menu's draw count, so each run gets a different seed.

Four runs (`context/p3f4/four{0,700,1900,3100}.p2s`, courts 2, 11, 3, 9), read by
`research/p3d3_setup_ram.py --chars context/p3f4/setup_ram.bin`. Voice banks (1 = b):

| court | banks   | case |
|-------|---------|------|
| 2     | 1 0 0 1 | first two differ: third keeps its draw; fourth b (two a among the first three) |
| 3     | 0 1 0 1 | first two differ; fourth b (two a) |
| 9, 11 | 0 0 1 1 | first two both a → third b; fourth b |

`setup_draws_like_the_game` now reads `context/p3f4/setup_ram.bin` too, and all four states match
`rng::voice_bank` from the recovered shared generator. Deleting either four-of-one arm of `voice_bank` fails the
test (court 9 player 2; court 2 player 3), so both arms are pinned by the original.

## Not verified / not 1:1
- The third player's "both b → a" arm and the fourth's "exactly one a → a" arm: no run drew them (four seeds; the
  third-player rule only was hit with a,a). They are the same code paths as the recorded mirror cases; more runs of
  the script with other waits would reach them.
