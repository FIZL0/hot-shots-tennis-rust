# B103 — ISO import on WebKit: "TypeError: Not enough arguments"

Cause: `web/worker.js` renamed the finished pack with `part.move('disc.pack')`. Chromium (and Firefox) accept a
name-only `FileSystemHandle.move`; WebKit only has `move(directory, name)` and throws
`TypeError: Not enough arguments` — the user's exact message. Fix: `part.move(dir, 'disc.pack')`, which every engine
takes.

Checked with a small worker page served on localhost (scratch, not committed):
- WebKitGTK MiniBrowser (`--features=+FileSystem,+StorageAPI,+FileSystemWritableStream,+AccessHandle`):
  `move('a.pack')` → `TypeError: Not enough arguments` (reproduced); `move(dir, 'b.pack')` → ok, file readable
  under the new name with its bytes.
- Chromium 152 headless: both forms ok, incl. `createSyncAccessHandle` + `truncate(0)` / `write(buf, {at})` /
  `flush` / `getSize` / `close` (the calls `import_disc` makes).

The other suspects in the brief don't throw that error: the sync-handle calls in `web.rs` always pass their
arguments, and the `createWritable` fallback only runs where `move` is missing (Safari has `move`).

## Not verified / not 1:1
- Import on a real iPhone (iOS Safari) and Android Chrome up to the menu: no devices here. WebKitGTK's OPFS backend
  doesn't implement sync access handles ("Backend does not support this operation"), so the full import couldn't be
  run in a WebKit engine locally; only the failing call was. Android Chrome uses the same Chromium calls checked above.
  If iOS still fails, the next message will name the next call.
- Not 1:1: n/a (browser shell, not gameplay).
