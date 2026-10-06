# Agent notes

Rust/Bevy remaster of Hot Shots Tennis (SCUS-97610). Faithful: same AI, physics, timing; better rendering.

- Retail data is read from the user's ISO at runtime. Never commit game data, extracted assets, or dumps.
- No retail addresses or Ghidra names (`FUN_…`, `DAT_…`) in shipped code; describe behaviour instead.
  Single exception (user decision, 2026-10-06): `hst-data::exe` reads tuning *data* that only exists inside the
  game program from the user's own GAME.BIN at documented offsets, after checking the disc is SCUS-97610 US 1.00.
  No code addresses, no game data in git.
- Exactness: ported math uses `hst_sim::ps2` (PCSX2's EE FPU model: chop rounding, aligned adds) wherever the
  original's float results must match bit for bit.
- Research lives in `context/` (git-ignored). Journal: `context/artifacts/YYYY-MM-DD-<slug>/`, one README per task,
  newest iteration file says whose move it is: `-FINAL` done, `-PART` agent continues, `-READY` needs the human.
- Gameplay ports are verified against the real game over PINE (`tools/pine.py`, launch with `tools/pcsx2-hst.sh`).

## Research setup (rebuild with these if `context/` is lost)
- `context/iso/` — 7z-extracted disc. `context/xb/` — `cargo run -p hst-data --bin xbdump -- <iso> context/xb`.
- `context/mkoverlay_elf.py` — disc ELF (`context/iso/SCUS_976.10`) + one Metrowerks overlay (ZZBIN/*.BIN, loaded whole at 0x322d00) per program.
- `context/ghidra/` headless project `hst` (r5900:LE:32:default); `context/decomp/hst_{game,menu,movie}.c` full dumps
  via `context/ghidra_scripts/DumpDecomp.java`; `context/fn.sh <addr>` prints one function.
- `replacements/` — user's upscaled texture pack (PCSX2 hash-named PNGs); to be mapped onto disc textures.

## Plan
`TODO.md` is the ordered plan; "continue" means: take the first open item there, consult its journal entry in
`context/artifacts/`, port + verify, update TODO.md, commit.

## Controlling the game
`TODO.md` → *Controlling the real game* (PINE, virtual pad `tools/vpad.py`, `tools/screenshot.sh`) and
*When blocked* (switch prompts, never stop while work remains). Unattended runs: `tools/overnight.sh`.

## If you get stuck, move on
Never wait or loop on one obstacle. If a tool hangs, PCSX2 won't cooperate, the same fix fails twice, something
needs the human, or ~20 minutes go by without progress: write what happened and what you tried in the prompt's
journal, mark it `- [~] … BLOCKED: <one-line reason>` in TODO.md, commit what's solid, and start the next open
prompt. Don't touch PCSX2 while a capture (`pgrep -f record_p2m2`) is running.
