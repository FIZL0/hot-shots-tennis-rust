# B227 — browser tab dies entering Singles with store mods installed

## Cause
The page read every installed mod file from OPFS into wasm memory before `run` (`web.rs add_mod_file` →
`modfs` MEM), and the select's preview baker then decoded them with the "upscaled textures" setting on (its
default). Sizes of the mods repo's builds (`../open-tennis-mods/out/mods`, 92 mods):

| | all mods | Anya (`oob_pc12_anya`) |
|---|---|---|
| `textures/upscaled/` PNGs | 6844 MB | 512 MB (207 PNGs, up to 10 MB each, 4K-class) |
| everything else (glb, wav, json) | 951 MB | 36 MB |

So ~90 % of what a store mod put into the tab's memory was its upscaled set, which then also got decoded at
full size for each baked/loaded costume.

## Fix
- `modfs::kept_in_memory`: the browser build (in-memory install, `install_with(disk = false)`) drops a mod's
  `textures/upscaled/` files as they're unpacked; they aren't checked or stored in OPFS. `mods::upscaled_dir`'s
  read then misses and the mod draws its own glb textures, as an online peer already does (`mods_online.rs`).
- `web/index.html`: before reading a mod, deletes an older install's `textures/upscaled/` from OPFS (frees the
  device's storage), and logs how many MB of mod files went into memory (`hst: mods read from OPFS (N MB)`).
- Anya now puts 36 MB into memory instead of 525 MB.
- Test: `modfs::tests::browser_install_leaves_upscaled_out`.

Not done (ponytail): reading the picked character's glbs from OPFS on demand. Those are 10–40 MB a mod; the
loaders are synchronous and the main thread can't read OPFS synchronously, so it needs an async prefetch before
the select/match loads a mod. Worth it only if many installed mods still outgrow a phone (comment in `modfs.rs`).

## Not verified / not 1:1
- On an iPhone: no device here. Not checked that Singles opens and a match plays with Anya + other store mods, and
  peak tab memory wasn't measured in a browser; the numbers above are file sizes on disk. Added under PLAN.md
  *Needs the human*.
- Which mods/device the user had: unknown (only "Anya + store mods, iPhone" from the task line).
- Mods in the browser no longer use their upscaled textures even with the setting on (deliberate, see above).
