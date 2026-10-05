# Agent notes

Rust/Bevy remaster of Hot Shots Tennis (SCUS-97610). Faithful: same AI, physics, timing; better rendering.

- Retail data is read from the user's ISO at runtime. Never commit game data, extracted assets, or dumps.
- No retail addresses or Ghidra names (`FUN_…`, `DAT_…`) in shipped code; describe behaviour instead.
- Research lives in `context/` (git-ignored). Journal: `context/artifacts/YYYY-MM-DD-<slug>/`, one README per task,
  newest iteration file says whose move it is: `-FINAL` done, `-PART` agent continues, `-READY` needs the human.
- Gameplay ports are verified against the real game over PINE (`tools/pine.py`, launch with `tools/pcsx2-hst.sh`).

## Research setup (rebuild with these if `context/` is lost)
- `context/iso/` — 7z-extracted disc. `context/xb/` — `cargo run -p hst-data --bin xbdump -- <iso> context/xb`.
- `context/mkoverlay_elf.py` — ELF + one Metrowerks overlay (ZZBIN/*.BIN, loaded whole at 0x322d00) per program.
- `context/ghidra/` headless project `hst` (r5900:LE:32:default); `context/decomp/hst_{game,menu,movie}.c` full dumps
  via `context/ghidra_scripts/DumpDecomp.java`; `context/fn.sh <addr>` prints one function.
