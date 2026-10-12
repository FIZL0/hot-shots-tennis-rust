# B84 — `Sway` inserted on a despawned select preview

**Cause.** `noise::attach` runs on `Added<Rig>` and queues `insert(Sway)` on the rig. When the select's `sync`
despawns that preview in the same frame (the hover or costume changed one frame after it spawned), the despawn
lands first and the insert panics (`Encountered a panic when applying buffers for system hst::noise::attach`).

**Fix.** `try_insert` in `noise::attach` (hunk byte-identical to B86's in s1, user asked the two slots to sync).
The same race applies to every system that reacts to a new rig/mesh/camera with a deferred command, so those are
`try_*` too: `shade.rs` (part material swap, `Lit` on the root; the extra GS draw copies are now parented by a
queued closure that despawns them if the rig is gone, rather than spawning with `ChildOf` to a dead entity, which
only warns and leaves an orphan mesh), `hud_gamma::retarget`, `shadow::casters`. Inserts in `inspect::spawn*`,
`character::spawn` and `previews::sync` are on entities spawned by the same command buffer: no race.

**Repro / check.** A local `HST_SCROLL` driver in `previews.rs` (not committed: B86 rewrites that file and has its
own `HST_STRESS` driver; copy in `context/b84/previews-driver.rs`): every 48 frames player 1 hovers the next disc
character then the next mod (all of `GRID` and all 92 mods round and round), dwells, then flips the costume every
frame for 16 frames. Window via `HST_BG=1 HST_MENU=mods`, VRAM watchdog on the process's own amdgpu fdinfo.
- unfixed build: panic in `noise::attach` after ~3 s (`context/b84/old1/log`).
- fixed: 0 panics in runs of 235 s, 187 s, 166 s, 291 s (`context/b84/fix2..fix6`); the 291 s run is ~3 full passes
  over every disc character and every mod. Runs ended by the watchdog (own VRAM > 4 GB) or by the window closing.

## Not verified / not 1:1
- Memory: with the select's preview cache never evicting, own VRAM grows without bound under the scroll; even a
  12-entry cap (tried locally, not committed) creeps ~260 MB/min and spikes past 4 GB on some mods. Not B84's
  scope: B86 (s1) replaces the cache with an LRU + PNG previews. Until it lands, minutes-long scrolls can still
  exhaust VRAM (B94's DeviceLost).
- Pad-input scrolling: the check drives `Menu` state directly, not a real pad/keys (Hyprland `send_shortcut` taps
  are too short for the held-direction repeat). The panic is in the preview spawn/despawn path, which this covers.
- Nothing here is a port of the original (crash fix only).
