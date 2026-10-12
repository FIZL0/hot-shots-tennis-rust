# B94 menu crashes — key-masher and first findings

- `HST_MASH=secs[,seed[,frames]]` (`crates/hst/src/mash.rs`): seeded random menu keys, the menu's own Quit swallowed
  until `secs`, then a normal `AppExit`. Run it from a scratch dir holding `game.iso` + `mods` symlinks: settings.txt and
  controls.txt are written next to the ISO path given, so the masher's random settings stay in the scratch dir.
  Always under a VRAM watchdog (memory note vram-watchdog): kill past +4 GB own or 12 GB system VRAM, `MemoryMax=6G`.
- The quit panic `Trying to destroy a SwapchainAcquireSemaphore that is still in use` (wgpu-hal 29.0.4) was not a normal
  quit: the log shows `radv: Not enough memory for command submission` → `DeviceLost` → Bevy quits → wgpu panics tearing
  down the lost swapchain. It came from the system-wide VRAM exhaustion of 2026-10-11 23:57 (B86 select stress).
- 8 runs of 40–60 s (incl. Steam overlay layer `ENABLE_VK_LAYER_VALVE_steam_overlay_1=1`): no panics, clean exits.
  One overlay run grew to +4.3 GB VRAM and hit the watchdog: the select-preview growth that B86 (slot s1) is fixing.
- B84's `Sway` panic: s3 already moved the inserts on fresh rigs to `try_insert`.

Next: after B86 and B84 land, re-run the masher (several seeds, with and without the overlay, ≥120 s) and quit from
the menu; done if no panic, no core dump and VRAM stays flat.
