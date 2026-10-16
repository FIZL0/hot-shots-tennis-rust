# B530 — online mod looks: nobody asked, the owner saw the donor

## Cause
A lobby pick of a custom character carries its `mods_online::Wire`, built by `Wire::of`: the costume's skeleton and
the hash of the mod's whole pack. In the browser a mod's big files are read in only when asked (B336:
`modfs::ready`; until then `modfs::read` fails with "still in the browser's storage"). The pick went out as soon as
the player readied (`online_ready` → `Cmd::Picks`). If the read-in wasn't done, `Wire::of` failed and only logged a
warning, so the pick went out as the **bare donor** (`r#mod: None`), and nothing ever sent it again. Then:
- the seat had no mod, so `Room::mods()` was empty: no peer asked or downloaded anything, and every peer, the owner
  included, played the plain donor (`launch` only puts in mods from the seats);
- the lobby note still read "Anya mod: you'll be asked to download it". The live place (`Live.r#mod`) named the mod,
  but the pick had no wire, so `remote_mod_note` fell through to the Ask setting's default text.

The desktop path itself is fine: the real Anya mod (`oob_pc12_anya`, 35 MB packed without `textures/upscaled`) builds
a wire for all 6 costumes, passes `Wire::check` and is 16 KB as JSON (checked with a throwaway test, not kept). So the
picker in the user's lobby was one of the browsers.

## Fix (`crates/hst/src/play/main_menu/online.rs`)
- `Menu::mod_picks_out`, called every frame from `poll` (as the court's `court_looks_out` already was for the same
  problem with court mods). It runs in the Lobby or the Confirm screen, outside a vanilla lobby. When one of this
  machine's seats holds no mod but its player is on one of this machine's own mods, it waits until `modfs::ready`
  (which asks the page to read the files in), then sends the picks again. It tries once per line-up (`View.rewired` =
  `Room.ver`). If the wire still won't build, the status line says why and that the donor plays it, instead of failing
  silently.
- `settle_to`: this machine's own pick always uses its own roster folder for its looks, found by id rather than by
  re-hashing the pack. So the owner sees their mod even if the pack hashes differently now, or a browser has unloaded
  part of it. A roster copy of someone else's mod is read in before it's hashed: the browser waits a frame instead of
  asking for a mod it already has.
- The Ready gate also waits until the mods' looks drawn on this machine are read in (`modfs::ready`), as it already did
  for the court mod, so a browser doesn't start the match with half a mod in memory.

## Checks
- `main_menu::online::tests::a_mod_pick_goes_out_again_once_read_in` (uses `context/mods/getagrip_pc00_emi`, copied to
  a temp dir; one costume marked lazy with `modfs::add_lazy`, as the browser leaves it). It checks that:
  - the pick goes out as the donor alone;
  - nothing is sent while the mod is still being read in;
  - once read in, the picks go out again with the mod's wire, its looks hash equal to the folder's pack;
  - they don't go out again for the same line-up;
  - the owner's seat settles to its own folder, unasked, even with a looks hash its pack no longer matches;
  - Ready is sent.
- `tools/check.sh -p hst mod`: 57 passed.

## Not verified / not 1:1
- **A real lobby with browsers, desktop and Android.** This unattended run had no browsers or second device, so the
  user's exact setup wasn't replayed. The fix is checked only through the browser's lazy-file path, simulated
  natively (`modfs::add_lazy`/`loaded`); the wasm build was only compiled (`cargo check --target wasm32-unknown-unknown`), not run.
- **Android as the picker.** Its mods are on a real file system (no lazy files), so as far as the code shows it never
  had this failure. Not run on a device.
- Not checked against the original: online play and mods don't exist in it (the menus are ours, PLAN).
