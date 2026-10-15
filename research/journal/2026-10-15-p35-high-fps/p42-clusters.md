# P42 — light clustering off (2026-10-15)

The game lights everything with one `DirectionalLight` plus ambient. Nothing in `crates/` spawns a `PointLight`,
`SpotLight`, `LightProbe`/environment map, irradiance volume or `ClusteredDecal` (no effect, menu or cut-away does
either), so Bevy's clustered lighting did work for nothing. On a desktop GPU, Bevy uses GPU clustering. For every
active `Camera3d`, every frame, `prepare_clusters_for_gpu_clustering` creates new cluster and index buffers sized
for 65 536 indices, and `cluster_on_gpu` runs five passes: Z slice, count raster, two allocations and populate.

`ClusterConfig::None` doesn't work on that path by itself: it gives the view 0×0×0 clusters, and
`prepare_cluster_dummy_textures` would then ask for a 0×0 texture. So the fix has two parts (`graphics.rs`):

- `cpu_clusters` (PreStartup): `GlobalClusterSettings::gpu_clustering = None`. That is Bevy's CPU clustering, which
  Android and WebGL2 already use. Every user of the setting reads the resource each frame (run conditions, and
  `ExtractResource` for the render world), so changing it before the first frame is enough.
- `unclustered` (PostUpdate, before `AssignLightsToClusters`): while nothing clusterable exists, 3D cameras get
  `ClusterConfig::None`, which skips all of the assignment work. If something clusterable ever appears (a mod, a
  later feature), the cameras get the default config back, and lose it again once that thing is gone.

## Measured (`tools/profile.sh`, 4-bot doubles court 1, 1280×800, traced; other agents were running too)

| | before | after |
|---|---:|---:|
| `prepare_clusters_for_gpu_clustering` | 242 µs | — |
| `cluster_on_gpu` (encode) | 116 µs | — |
| `prepare_clustering_bind_groups` | 65 µs | — |
| `prepare_clusters_for_cpu_clustering` | — | 34 µs |
| `assign_objects_to_clusters` + extract + `unclustered` | (below top 80) | ~12 µs |

That is about 420 µs less per frame in the render world, which sets the frame rate. The GPU no longer runs the
clustering passes either.

A/B shot (`tools/shot.sh`, `HST_WEATHER_SEED=1234`, court 1 at 6 s, with a temporary env switch since removed): the
court, net, umpire and backdrop shade the same. The only differing pixels are on the edges of moving players, the
ball marks and foliage, because the two shots were taken at slightly different moments of the match.

### Not verified / not 1:1

- Rendering only: no sim, RNG or timing is touched, so there's no PCSX2 check.
- The A/B shots aren't pixel-identical, because the two runs were caught at different moments (wall-clock
  `--shot-at`). The static parts of the scene match by eye, and the difference image has no change in shading.
- Only checked on the desktop Vulkan GPU path. iOS, web and Android: Android/WebGL2 already ran CPU clustering, and
  `ClusterConfig::None` is Bevy's documented "disable cluster calculations" setting. iOS and WebGPU weren't run.
