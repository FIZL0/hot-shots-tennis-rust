# B149 — Bevy errors when running

## Run
Release `hst` on the disc with `RUST_LOG=warn`, `HST_BG=1`, in a floating window, driven by `HST_KEYS`:
Local → Singles → controllers → 1P picks → COM picks the next character → Start the match → ~50 s of the match →
Esc → Return to the Menu → a second match → menu again.

## Distinct messages (before)
| count (1 match, ~60 s) | message | cause |
|---|---|---|
| ~20 000 | `ERROR bevy_render::slab_allocator: Use-after-free: attempted to copy element data for an unallocated key` | `play/flare.rs` |
| 1 per mod | `WARN hst::mods: … face channels missing / no blink_eye / Bip01 height …` | ours: the mod checker on the user's `mods/` (data, not a Bevy error) |
| 1 per costume | `WARN hst::play::main_menu::previews: select preview of Mod(45) … rest pose must be the bind pose` | ours: that mod (oob_pc01_nick) fails the standard; refused, as meant |
| 1 | `WARN gilrs::gamepad: No mapping found for UUID …` | a connected pad SDL has no mapping for; gilrs' default mapping is used |

The menus alone logged nothing from Bevy.

## Cause and fix
Bevy 0.19's mesh allocator skips allocating a mesh with an empty vertex buffer but still copies its data, logging the
error once per such mesh every time it is extracted. A probe on `AssetEvent<Mesh>` (removed again) found two meshes
added and then modified every frame with 0 positions/uvs/colours and no indices: the lens flare's rays and sprites.
`flare::draw` replaced both meshes each frame, empty whenever the sun is off screen (most of the time).
Fix: an empty `Built` becomes one zero-area triangle (as `effects::fill` already does), and a hidden part's mesh is no
longer rewritten every frame.

## After
Menu → match (court 8) → menu → match (court 6) → menu, and menu → match (court 2) → menu: no Bevy warnings or
errors; only the mod-data warnings and the gilrs mapping notice above remain.

Harness note: a scripted `HST_KEYS` arrow is held for one frame, which at uncapped fps often falls between the pause
menu's 60 Hz steps; repeat the step at the same time (`33:ArrowUp` ×10) to hold it. Not a bug for real key presses.

## Not verified / not 1:1
- Only courts 2, 5, 6 and 8 and singles were run; other courts/doubles/online could have their own warnings. The flare
  is the only per-frame empty mesh the probe found on those runs.
- The `hst::mods` warnings are our own checker reporting the user's mod files and were left as they are.
