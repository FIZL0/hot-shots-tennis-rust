// B358: OPFS writes for a page without FileSystemFileHandle.createWritable (Safari before 26, iOS 18): a worker's
// sync access handle is the only way it writes a file. One message per file: { id, path: 'a/b/c', bytes }, answered
// with { id } or { id, error }; folders on the path are made as needed, the file replaced whole.
onmessage = async ({ data: { id, path, bytes } }) => {
  let h;
  try {
    const parts = path.split('/');
    let d = await navigator.storage.getDirectory();
    for (const s of parts.slice(0, -1)) d = await d.getDirectoryHandle(s, { create: true });
    h = await (await d.getFileHandle(parts.at(-1), { create: true })).createSyncAccessHandle();
    // (awaited: WebKit's first versions of these returned promises)
    await h.truncate(0);
    for (let at = 0; at < bytes.byteLength;) {
      const n = await h.write(bytes.subarray(at), { at });
      if (!(n > 0)) throw new Error(`wrote ${at} of ${bytes.byteLength} bytes`);
      at += n;
    }
    await h.flush();
    postMessage({ id });
  } catch (e) {
    postMessage({ id, error: String(e) });
  } finally {
    await h?.close();
  }
};
