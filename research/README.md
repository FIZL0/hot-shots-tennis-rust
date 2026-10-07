# Research setup

How the port is checked against the original game, and how to rebuild the private working folder `context/`
from your own disc. Everything in `research/` is our own work: scripts, Ghidra scripts and the research journal.
**No game data, decompiled code, captures or screenshots go in here.** Anything made from the disc or the running
game lives in `context/`, which is git-ignored and must never be shared.

## What's here

| Path | What |
|---|---|
| `journal/` | Research notes per task (`YYYY-MM-DD-<slug>/N-TOPIC-STATE.md`). `-FINAL` done, `-PART` in progress, `-READY` needs the human. `TODO.md` links each item to its entry. |
| `fn.sh <addr>` | Prints one function from the decompilation dump, e.g. `research/fn.sh 34afc0`. |
| `mkoverlay_elf.py` | Builds one ELF per program overlay (GAME, MENU, MOVIE) from the disc's boot ELF, so Ghidra sees the overlay code at its load address. |
| `ghidra_scripts/` | `DumpDecomp.java` decompiles every function to one `.c` file. `DumpAsm.java` prints the instructions of chosen functions. |
| `tools/` | Python analysis scripts: format walkers (`mdlwalk`, `mtlwalk`, `mdlvif`, `vifwalk`), the PS2 FPU model (`ps2fpu`), a VU0 disassembler, ball-flight and shot-table models, and the generators for test recordings (`fixture_*.py`). |

The scripts that drive the live game are in the top-level `tools/` (PINE client, recorders, virtual pad,
screenshots).

## You need

- Your own **Hot Shots Tennis US 1.00 (SCUS-97610)** disc image at the repository root as
  `Hot Shots Tennis (USA).iso` (see the main README).
- **For the decompilation:** [Ghidra](https://ghidra-sre.org) 12.x plus the
  [ghidra-emotionengine-reloaded](https://github.com/chaoticgd/ghidra-emotionengine-reloaded) extension, which
  adds the PS2's `r5900` processor. This setup used Ghidra 12.1.2 at `/opt/ghidra`. You also need `7z` and
  Python 3 with `numpy`.
- **To check against the running game:** [PCSX2](https://pcsx2.net) 2.x with **your own PS2 BIOS**, and
  Linux for the virtual pad (`/dev/uinput`).

## Rebuild `context/`

Run everything from the repository root.

```sh
# 1. The disc's files, and every XB archive unpacked
7z x "Hot Shots Tennis (USA).iso" -ocontext/iso
cargo run --release -p hst-data --bin xbdump -- "Hot Shots Tennis (USA).iso" context/xb

# 2. One ELF per overlay (the overlays are loaded whole at 0x322d00)
mkdir -p context/elf && python3 research/mkoverlay_elf.py

# 3. Ghidra project `hst` and the full decompilation (slow: tens of minutes per program)
mkdir -p context/ghidra context/decomp
for p in game menu movie; do
  /opt/ghidra/support/analyzeHeadless context/ghidra hst -import context/elf/hst_$p.elf \
    -processor r5900:LE:32:default -scriptPath research/ghidra_scripts \
    -postScript DumpDecomp.java "$PWD/context/decomp"
done
# → context/decomp/hst_{game,menu,movie}.c and .funcs.tsv; research/fn.sh <addr> reads hst_game.c

# Instructions of a few functions, from the existing project (no re-analysis):
/opt/ghidra/support/analyzeHeadless context/ghidra hst -process hst_game.elf -readOnly -noanalysis \
  -scriptPath research/ghidra_scripts -postScript DumpAsm.java "$PWD/context/notes/asm_x.txt" 32f690 335b70
```

`context/` in use looks like this:

| Path | Made by |
|---|---|
| `context/iso/`, `context/xb/` | steps 1 |
| `context/elf/`, `context/ghidra/`, `context/decomp/` | steps 2–3 |
| `context/notes/` | `DumpAsm` output, hand-copied functions, tool logs |
| `context/ram/s05.bin`, `context/fixtures/slot5_ee.bin` | EE RAM from a save state: `7z e -so <state.p2s> eeMemory.bin > …` |
| `context/fixtures/match_s05.bin`, `round1.bin` | `tools/record_p2m2.py` (slot 5 bot match; the round1 input recording) |
| `context/fixtures/camera_s05.bin` | `tools/record_camera.py 5 …` |
| `context/live/net_s05.bin` | `tools/record_live.py 5 …` |
| `context/shots_s05/` → `fixtures/shots_s05.csv`, `flights_s05.csv`, `shot_tables_s05.csv` | `tools/trace_shots.py 5 …`, then `research/tools/fixture_shots.py`, `fixture_flights.py`, `traj_inverse2.py` + `fixture_shot_tables.py` |
| `context/fixtures/ball_path_s08.csv` | `research/tools/fixture_ball_path.py` on a RAM image |
| `context/fixtures/shot_params_ram.bin` | the shot parameter table from a save state's RAM |
| `context/recordings/*.p2m2` | PCSX2 input recordings (Tools → Input Recording) |

Each test names the recording it needs in its header comment and skips itself when that recording is missing.
`HST_FIXTURES` points the tests at another fixtures folder.

## PCSX2 save states

The recorders and `TODO.md` refer to save-state slots. They are yours to make: load the game, get to the moment,
and save the state to that slot.

| Slot | Moment |
|---|---|
| 3 | Start of a doubles game: P1 human + 3 CPUs |
| 4 | Mid-rally, right after the serve |
| 5 | A full CPU-only doubles game (no human input; for ball, AI and camera captures). Court 10. |
| 8, 9 | Scratch saves for tools. Never overwrite 3–5. |

Launch with `tools/pcsx2-hst.sh`, which turns PINE (PCSX2's memory IPC, port 28011) on and boots the disc. For
gap-free per-frame recordings, slow the game down (`[Framerate] NominalScalar = 0.5` or `0.25` in `PCSX2.ini`).
Each recorder says which in its header.

## Pitfalls

- **One PINE connection at a time.** A second client while a recorder is polling hangs both. To read static
  values during a capture, read them from the save state (`7z e -so <p2s> eeMemory.bin`) instead.
- **Don't touch PCSX2 during a capture** (`pgrep -f record_p2m2`).
- **`pkill -f <pattern>` kills your own shell** when the pattern is in its command line. Use
  `pgrep -x pcsx2-qt` and kill by PID.
- **Never press Select on the virtual pad.** PCSX2's hotkeys are Select + shoulder combinations.
- **`tools/vpad.py` timing is wall-clock.** For frame-exact input, use PCSX2 input recordings (`.p2m2`).
- **Gamepads with a read-only device node** (udev rules that block rumble writes) work in the remaster through the
  patched `third_party/gilrs-core`. Steam Input's virtual pads are ignored.
