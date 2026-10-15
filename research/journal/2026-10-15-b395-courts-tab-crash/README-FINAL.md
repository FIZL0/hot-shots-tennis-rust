# B395 — "crash on pressing Courts in Mods" (`render_phase/mod.rs:1241` index out of bounds, len 0, index 0)

## What the core dump is
`coredumpctl list` has two release `hst` aborts that evening, 23:43:46 and 23:46:38 (`target/release/hst --windowed`,
cwd the main checkout). Their environment (strings in the dump) has `HST_WEBGL2=1`, `HST_WINDOW=1280 720` and
`CLAUDECODE=1`: they were started from a Claude Code session, the B385 agent's own reproduction of the web panic
(its journal: "`HST_WEBGL2=1` … `HST_WINDOW="1280 720" hst --windowed` … panicked in 2 of 3 runs"). The panic text
in both is the unpatched `dynamic_offsets[entity_index]` of `BinnedRenderPhase::render`, on a compute-pool thread.
B385's fix (`third_party/bevy_render`, committed 00:07 on 10-15) postdates both dumps. So the dump the report points at
isn't a Courts-tab crash; it is the CPU-batching gap B385 already closed. No other `hst` dump exists from that time.

## What changed anyway
The GPU-preprocessing path (desktop Vulkan, `batching/gpu_preprocessing.rs`, unbatchables = skinned meshes) had the
same hole as B385's CPU path: an unbatchable entity with no binned index this frame was `continue`d without a slot,
so the bin's index list came up short and the entities after it drew with the next one's instance /
indirect-parameters index (a `Dense` list left over from an earlier frame would also panic exactly like this). It now
calls `buffer_indices.skip()` too, so `render` skips that entity and the rest keep their own indices.

## Checked
Release build, `HST_MENU=browsemods|addmods|uploadmods`, `HST_KEYS="2:Digit3 3:Digit3 4:Digit1 5:Digit3"` (Characters
→ Courts → Umpires → Courts → Umpires), with and without `HST_WEBGL2=1` (logs: "GPU preprocessing is fully supported"
vs "Falling back to CPU"): 6 runs, no panic, screenshots in `context/b395/run1/`.

## Not verified / not 1:1
- The user's exact click sequence: no dump of a non-agent run exists, so if the user did see a crash on Courts it left
  no trace; scripted key runs on every page in both batching modes don't crash.
- The GPU-path skip itself isn't hit by these runs (no unbatchable entity lost its binned index), so it is checked
  by review and by the same code path B385 exercises on the CPU side.
