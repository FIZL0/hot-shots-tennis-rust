# B94 menu crashes — re-run after B84/B86, the bad-path crash

## Masher re-runs (B84 and B86 merged)
`HST_MASH=secs,seed[,frames]` from a scratch dir (`game.iso` + `mods` symlinks; the ISO path goes first: `hst game.iso
--play`, a leading flag makes `disc::resolve` open the picker), `RUST_BACKTRACE=1`, under `tools/vram_watch.sh` (4 GB):

| run | start | secs | seed,frames | Steam overlay layer | result | own VRAM max |
|---|---|---|---|---|---|---|
| r1 | main | 120 | 7 | – | clean exit 0 | 915 MB |
| r2 | main | 120 | 11 | – | clean | 874 |
| r3 | main | 120 | 23,8 | – | clean | 874 |
| r4 | main | 120 | 7 | on | clean | 884 |
| r5 | main | 120 | 31,6 | on | clean | 876 |
| r6 | graphics | 90 | 5 | on | clean | 877 |
| r7 | graphics | 90 | 9 | – | clean | 876 |
| r8 | chars | 120 | 13,3 | on | clean | 914 |
| r9 | chars | 120 | 17,3 | – | clean | 874 |

No panic, no `DeviceLost`, VRAM flat (B86's growth is gone), every run ends with the masher's normal `AppExit` (the
same path as the menu's Quit), and `coredumpctl` shows nothing new since 2026-10-09.

## The 2026-10-09 SIGSEGV
`coredumpctl info 250726`: the command line was `dist/linux/hst '/path/to/Hot Shots Tennis (USA).iso'` (a Steam
launch with the placeholder path from the README), and `vkDestroyDevice` → `steamoverlayvulkanlayer.so` crashed on a
worker thread. Reproduced the first half here: with a path that doesn't exist, `disc::resolve` passed it through, the
window and device came up, then the startup systems panicked (`open iso: NotFound`, audio.rs) and the app tore the
device down; under Steam that teardown runs through the overlay layer, which segfaulted.

Fix at the cause (`disc.rs` `resolve`): a disc path that `Iso::open` can't open is dropped with a note on stderr and
the start falls back to the imported pack (or the picker / import when there is none), before any window exists.
Checked: the placeholder path now prints `Couldn't open the disc image …: using the imported disc` and opens the
picker, no panic, no window.

## Not verified / not 1:1
- Steam overlay with Steam running: Steam wasn't running on this machine during the runs, so
  `ENABLE_VK_LAYER_VALVE_steam_overlay_1=1` loads the layer inert; the overlay's own teardown crash couldn't be
  reproduced, only its trigger (the panic) removed. A launch from Steam with a bad path is the check for the user.
- The masher reaches screens at random; the Graphics and select screens were covered by starting there (`HST_MENU`).
- Menus are ours to design (PLAN), so there is nothing to compare with the original.
