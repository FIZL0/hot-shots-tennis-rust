# B204 — stream mod store downloads, unpack from the stored file

Store downloads stream to disk and are hashed while they arrive: `root/.download.zip` on desktop, OPFS
`mod-download.zip` on web. The status line shows "Downloading X... a / b MB (n%)" (`modstore::Event::Progress`).
Unpacking reads the stored zip one entry at a time through an async `read(at, n)`. On desktop that is a File with
seek, run to completion by a noop-waker `modfs::now`; on web it is `Blob.slice`. There is no whole-zip copy in
memory and no staging clone. A failed hash or check removes the download file and the `.import` stage. The same
checks still run: HTTPS from the store only, sha256 and size checked before unpacking, and every modfs check
(`check_mod` + per-file `check_file`). A picked zip (desktop `pick`, web `import_mod(File)`) uses the same path.

Iterations: [1-STREAM-FINAL.md](1-STREAM-FINAL.md)
