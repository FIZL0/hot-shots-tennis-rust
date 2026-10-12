# M13a handoff (s3 → s2, 2026-10-12)

Stopped on the user's request (slot 2 takes M13a after M2c). Nothing ticked or blocked; `task/M13a` reset to main.
Everything is on branch `handoff/M13a` (one WIP commit + this note).

## Done (on this branch)
- `crates/hst-data/src/iso.rs`
  - `Iso` reads through `Box<dyn Source>` (`Source` = any `Read + Seek`), not `File`. `Iso::from_source(box)` opens an
    ISO or a pack from any reader. Iso is never stored in a Bevy resource, so no `Send` bound was needed.
  - `iso::mount(path, Arc<[u8]>)`: a global path → bytes table; `Iso::open(path)` checks it first and reads a
    `Cursor` over the shared bytes. So **every existing `Iso::open(&args.iso)` caller (≈30) works unchanged** in
    the browser once the pack is mounted as `disc.pack`.
  - `write_pack` split: `write_pack_to(&mut impl Write + Seek, progress) -> hash` does the work (flushes, writes the
    hash at offset 8); `write_pack(path)` is the `.part` + fsync + rename wrapper around it. Pack bytes unchanged.
- `crates/hst-data/tests/pack.rs`: also checks `write_pack_to` into a `Cursor` gives the same bytes as the file, and a
  mounted pack opens by its mount path with the same hash/contents. **Passes** (`tools/check.sh -p hst-data --test pack`,
  with the ISO present).
- `crates/hst/src/main.rs` (small): `mod web` (wasm32 only), `mod disc` gated to non-wasm; `main()` body moved into
  `fn start(argv)`; native `main` = args → `disc::resolve` → `start`; wasm `main` is empty (wasm-bindgen runs a bin's
  `main` on init, also in the worker, so it must not start Bevy).
- `crates/hst/src/web.rs` (new, wasm32 only):
  - `import_disc(file: File, out: FileSystemSyncAccessHandle, progress: Function) -> Result<String hash hex>` — for a
    **Web Worker**: `Picked` reads the picked `File` by extent (`File.slice` + `FileReaderSync`, sync in workers only;
    the 1.4 GB image is never read whole), `Opfs` writes via the sync access handle (`write` with `{at}`), through a
    1 MB `BufWriter`. Truncates `out` first. The disc check (`exe::Game::new`) is inside `write_pack_to`.
  - `run(pack: Uint8Array)` — for the page: copies the pack straight into one `Arc<[u8]>` (`new_uninit_slice` +
    `copy_to_uninit`, so it's held once in wasm memory), mounts it as `disc.pack`, builds argv from the URL query
    (`?play&stage=1` → `--play --stage 1`; values split on whitespace, `+` = space) and calls `crate::start`.
- `crates/hst/Cargo.toml`: wasm32-only deps `wasm-bindgen 0.2`, `js-sys 0.3`, `web-sys 0.3` (features Blob, File,
  FileReaderSync, FileSystemSyncAccessHandle, FileSystemReadWriteOptions, Location, UrlSearchParams, Window); they
  resolve to the versions already in Cargo.lock (0.2.129 / 0.3.106).
- `.gitignore`: `target-wasm/`.
- **Builds**: `CARGO_TARGET_DIR=$PWD/target-wasm cargo build -p hst --target wasm32-unknown-unknown --release` OK
  (~3 min), only the existing `human_mate` warning in play.rs. Native build not re-run after the main.rs change
  (it's a pure move; run `tools/check.sh`).

## Left to do
1. **Page + worker** (`web/index.html`, `web/worker.js`) and a build script (`tools/web.sh`: cargo build as above, then
   `wasm-bindgen --target web --out-dir web/pkg target-wasm/wasm32-unknown-unknown/release/hst.wasm`; optionally
   `wasm-opt -O`). Tools present on this machine: `~/.local/bin/wasm-bindgen` 0.2.129 (matches the lock),
   `/usr/bin/wasm-opt`, `/usr/bin/chromium` 152, `node` 25, `deno`. No trunk, no playwright.
   - Page: `navigator.storage.getDirectory()` → try `getFileHandle('disc.pack')`; found → `new Uint8Array(await
     (await h.getFile()).arrayBuffer())` → `init()` → `run(bytes)`. Not found → show `<input type=file accept=.iso>`;
     on pick, post the `File` to the worker.
   - Worker (module worker, `import init, {import_disc} from './pkg/hst.js'`): `getFileHandle('disc.part',
     {create:true})` → `createSyncAccessHandle()` → `import_disc(file, h, (i,n)=>postMessage(...))` → `h.flush();
     h.close()` → `part.move('disc.pack')` (or copy+remove if `move` is missing) → tell the page, which loads as above.
   - "Offer to forget the ISO": the browser never keeps the ISO (the `File` is gone after the import); say "the ISO is
     no longer needed" after import, and offer a "forget the imported disc" button (`removeEntry('disc.pack')`) for
     re-import. (My reading; not confirmed with the user.)
   - Show import errors (wrong disc: the `exe` check's message comes back as the rejection string).
2. **Run it in a browser** — never done yet. Serve `web/` with `python3 -m http.server` (OPFS needs a secure context;
   localhost is one). Drive headless chromium over CDP (node 25 has a global `WebSocket`): `--headless=new
   --remote-debugging-port=… --user-data-dir=<fresh dir in context/>`, `DOM.setFileInputFiles` with the ISO for the
   first visit, then reload with the same profile and **no file** → `?play&stage=1` should start a match →
   `Page.captureScreenshot` into `context/m13a/`. That's the task's done condition.
3. **Expected runtime problems** (found by grep, not yet hit):
   - `graphics.rs limit`: `std::time::Instant::now()` panics on wasm32-unknown-unknown — only reached when `fps != 0`
     (default 0, so probably fine unless settings/flags set it); `std::thread::sleep` too.
   - `play/main_menu/previews.rs`: `std::thread::spawn` (panics on wasm) — the menu path only; `?play` avoids it but
     the menu (no query) will hit it.
   - `std::time::SystemTime::now()` in `main.rs:~380` (skip seed) and `play/weather/rain.rs:264` — panics on wasm.
   - `std::fs` reads/writes (settings.txt, controls, mods, shade cache) return errors on wasm → defaults; check none
     `.unwrap()`s on the play path (`controls.rs`, `main_menu.rs` look error-tolerant).
   - Rendering: custom materials (`gs`, `shade`, `shadow`) on WebGL2 (bevy default features; no storage buffers in
     WebGL2) are untested; WebGPU might need the `webgpu` bevy feature. This overlaps M3.
   - Audio: needs a user gesture to unlock (M3).
   - Memory: pack 171 MB in JS (ArrayBuffer) + 171 MB in wasm during `run`; the JS copy can be dropped after.
   Fixing these is arguably M3's job; M13a's done condition (a match starts) needs at least the play path working.

## Design decisions
- Mount table in `hst-data` instead of threading an `Iso`/bytes through ~30 call sites: callers keep `Iso::open(path)`.
- Import runs in a worker with sync APIs (FileReaderSync, OPFS sync handle) so `write_pack_to` stays the same sync
  Rust code as native — no JS port of the ISO9660 walk, the disc check or the XXH3 hash.
- The pack is read whole into memory at start (option b of the task line), not via a sync handle per read: simpler,
  and every `Iso::open` stays sync on the main thread.
- The same `hst.wasm` serves page and worker (main is empty on wasm; `run` starts the game).

## Open questions
- What "offer to forget the ISO" means on the web (see above).
- Whether `?flags` in the URL is the wanted way to pass startup flags, or the menu only (no query → main menu).
