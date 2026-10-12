# 12 — Type 21 on court 6 (P14c4b), in progress

- No save state on any copy is on court 6 (scanned `*(0x422f90)` in every copy's .p2s).
- Plan: from slot 2 (doubles character select) take pick.py's menu path and set the court index `0x422f90` = 6
  before "Start the match!" (the carousel's court 6 is 3 steps right of its start, REFERENCE.md), save the serve to
  scratch slot 9, then `HST_BALL=1 HST_MATS=1 tools/record_npc.py 9 <n> context/fixtures/hit_c06.bin prox` with
  `HST_POKE` onto the ball (`B`), and add `hit_c06` to `ball_hit_obstacles_match_the_game`.
- Blocker hit so far: the Hyprland session restarted mid-run (old socket gone, `hyprctl clients` shows no windows),
  and since then copy 4 takes no virtual-pad input: even `tools/pick.py 0 9` stops at "match didn't reach the
  serve", although PCSX2's log shows "HST vpad 4" opened. PINE reads work. Next: restart copy 4 once the desktop
  shows its window again, re-run pick.py to confirm the pad, then the plan above.

## Blocked (needs the human)
- `pgrep -a Hyprland` → `Hyprland --watchdog-fd 4 --safe-mode --locked`, started 02:13, before this run. The
  compositor lists no windows; screenshots (F8 via hyprctl) write nothing.
- The vpad device (`/dev/input/event26`, "HST vpad 4") does emit events (read back directly), and PCSX2's log
  opened it, but the game reacts to none of them: `pick.py 0 9` failed twice, once before and once after restarting
  copy 4. PINE reads and writes work.
- Unblock: unlock or restart the desktop session, then `tools/pcsx2-hst.sh` (copy 4) and `tools/pick.py 0 9` to
  confirm the pad. After that, follow the plan above.

## Not verified / not 1:1
- Everything for type 21: no recording yet (above).
