# B336: browser mods read in on demand

Phone tabs died again with ~40 store mods installed (user 2026-10-14). The page read every installed mod file out of
OPFS into wasm memory at startup (≈ 10–40 MB a mod), and a Download all kept every new mod in memory until reload.

## Change
- `modfs.rs`: a `LAZY` index (path → size) beside `MEM`. A lazy file is listed (`is_file`, `size`, `read_dir`) but
  `read` gives `WouldBlock`. `ready(dir)` asks the page to read the mod's files in (`web::mod_load` → `hstModLoad`,
  then `mod_loaded`) and is true once they're in; it keeps the 6 most recently wanted mods in memory and unloads the
  rest (`unload`: back to LAZY, the `small` ones stay). Native: `ready` is always true, nothing is ever lazy.
- `small`: `.json` manifests and the mod's own `previews/` pictures stay in memory (listing mods and the select grid).
- `web/index.html`: startup reads only the small files and registers the rest (`add_mod_lazy`); `hstModLoad(dir)` reads
  one mod folder in a file at a time; after `hstModSave` the mod's big files leave memory (`mod_saved`).
- Callers that wait: the select's live model and card bake (`Loader::want`), the card voice, the local match launch
  (`step`'s `launching`, every `--slot-mod` dir), store upload (`mod_files`: "press again"). Online picks only ask
  (`wire`): a hover in the select has read the mod already (ponytail note there).
- Test `a_lazy_mod_is_read_in_when_wanted_and_unloaded_past_the_budget`.

## Check (headless Chromium, WebGL2/SwiftShader, `tools/web_check.mjs`, runs in `context/b336/`)
- Profile with the disc imported, three mods (Phoebe, Mike, Emma, 44 MB) written into OPFS.
- Startup: "mods read from OPFS (0.0 MB) in 11 ms" (before: all 44 MB). "3 custom characters".
- First select visit: each mod read in when the baker reached it (≈ 5.5 s each on SwiftShader, 13–17 MB), cards drawn.
- Second start (cards cached): only Mike read in, on hover; picking him and confirming started the match
  (`--slot-mod 0 ./mods/fore_pc01_mike`) and Mike played (run3 visit2-84s).

## Not checked
- `mod_saved` after a store download in the browser (a store install takes minutes headless); it's the same `unload`
  the test covers.
- A real phone: the user's to try with the 40 mods.
