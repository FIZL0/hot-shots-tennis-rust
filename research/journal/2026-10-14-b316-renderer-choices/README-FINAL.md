# B316 — Renderer row's choices from its own list

The visible bug (the web build's Renderer row listing Off/On) had already been fixed on main by B324, which added
a hardcoded `Renderer => ["Auto", "WebGPU", "WebGL2"]` arm to `choices`. What was still wrong was that the same words
were typed out twice: once in `choices` and again in `Settings::value`, separately from `RENDERERS`.

- `main_menu.rs`: `RENDERERS` is now `[(settings.txt word, row word); 3]`, so each value's id and its label
  sit side by side. settings.txt parse/write and `web::renderer` read `.0`.
- `options.rs`: Renderer's choices and value both read `RENDERERS[..].1`. Show FPS, Shadows and MSAA
  (the other index rows with words typed out twice) now share `SHOW_FPS_NAMES` / `SHADOW_NAMES` / `MSAA_NAMES`.
- Test `choices_are_each_rows_own_values`: for every row (Renderer included, which no native page shows),
  one round of stepping visits exactly that row's choices and comes back to where it started.
  It runs on native, so the wasm-only row is covered too.

Checks: `tools/check.sh -p hst --bin hst main_menu` passed apart from `mods_screen::upload_all`, which failed once
in that run and passed on its own (flaky and unrelated to this change). `cargo check --target wasm32` builds.

## Not verified / not 1:1
- No screenshot of the browser build's Settings → Display: the browser build was not run. The test covers
  the choices list, and the original game has no such row (front-end is ours to design).
