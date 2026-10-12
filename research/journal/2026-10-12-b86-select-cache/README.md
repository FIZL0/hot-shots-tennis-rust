# B86 — character select from a PNG cache, no crashes while scrolling

Task: scrolling the character select at full speed through every disc character, costume and mod must not
crash. Previews and icons come from PNGs drawn once and cached; a live model only where needed.

Files: `crates/hst/src/play/main_menu/previews.rs` (rewritten), `noise.rs` (`try_insert`), `main_menu.rs`
(menu input off under `HST_STRESS`), `tools/select_stress.sh`, `tools/vram_watch.sh`; ../HST-MODS
`standard/HST-CHARACTER-STANDARD.md` §1 `previews/`, `tools/rerig/previews.py`, README step 4.

Iterations: `1-B86-FINAL.md`.

B99 (cached pictures not loading properly on the cards): `2-B99-FINAL.md`. Files: `previews.rs`, `mods.rs`
(`own_motions` skips a joint the costume lacks), `tools/select_stress.sh` (`HST_SHOTS`); ../HST-MODS standard §6.
