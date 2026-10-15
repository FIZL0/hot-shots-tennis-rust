# P36 — shadow caster cost (from P35's profile)

P35 traced `write_binned_instance_buffers<Shadow>` at 619 µs and the shadow pass encode at 211 µs a frame for the
4 cascades. Two causes, both cut:

1. **Retained bins re-uploaded every frame.** Bevy 0.19 keeps each view's bins between frames, but
   `write_binned_instance_buffers` called `RawBufferVec::write_buffer` on both of every batch set's buffers (instance
   → input-uniform/bin, bin → indirect offset) for every view, every frame: one `queue.write_buffer` each, ×4 shadow
   views. `third_party/bevy_render` (already patched for B385) now keeps a `dirty` flag on
   `RenderMultidrawableBatchSetGpuBuffers`, set by every mutation (new instance, changed instance entry — compared,
   not blindly set —, swap-remove, new bin's indirect offset, removed bin) and the upload is skipped while clean and
   the GPU buffer exists. Applies to every binned phase (Opaque3d, AlphaMask3d too).
2. **Cascades.** High used Bevy's 4 with the first ending at 80 m and the maximum at 120 m, so cascades 1–3 were
   slivers over 80–120 m (80 / 91.6 / 104.8 / 120) that each drew nearly every caster again (their bounding spheres
   take in the whole venue). High is now 2: the same first cascade (0–80 m, unchanged projection) and one 80–120 m.

Static casters weren't split out further: with (1) an unchanged caster costs no upload at all, and Bevy already culls
casters per cascade (`check_dir_light_mesh_visibility`, per-cascade frusta).

## Numbers (`tools/profile.sh`, 4-bot doubles, court 1; traced, other agents loading the machine ~30–40, so
compare rows within a run, not fps)

| µs/frame | base | retained bins | + 2 cascades |
|---|---:|---:|---:|
| `write_binned_instance_buffers<Shadow>` | 1255 | 57 | < 40 (below the table's 80 rows) |
| `write_binned_instance_buffers<Opaque3d>` | 274 | < 40 | < 40 |
| `per_view_shadow_pass` encode | 429 | 400 | 316 |
| `unpack_bins` | 349 | 345 | 276 |
| Render schedule self | 7397 | 6146 | 5914 |

Tables: `context/perf/p36/{base,dirty,final}.txt`.

## A/B (`context/perf/p36/shots/`)

Court 1, `--shadows high --shot-at 2`, HST_WEATHER=0 (temporary env toggles, removed): base vs retained bins differ
only where the idle players animate (real-time shot moment); 2 cascades vs base: no difference on any static shadow
(trees, net, umpire's chair); 1 cascade to 120 m (Low's) moves tree and net shadow edges — rejected for High.
After 40 s of play on court 1 and 25 s on court 8 (jungle canopy on the far half): all shadows present, players' too.

## Not verified / not 1:1

- Exact same-frame pixel compare: the shots are taken at a real-time moment, so animated players differ between
  runs; static shadows were compared, moving ones by eye.
- `third_party/bevy_render`'s own unit tests (bin bookkeeping): cargo won't test a patched non-workspace crate
  with dev-dependencies; the bookkeeping itself is unchanged, only the flag was added.
- Beyond 80 m from the camera the far cascade has fewer texels a metre than the old three; nothing the match
  camera shows sits there (checked courts 1 and 8 only, not cut-aways looking out over a venue).
- fps not re-measured cleanly: the machine was loaded by other agents' builds throughout.
