// Imports the picked ISO into OPFS `disc.pack` with the game's own pack writer (sync file APIs: workers only).
// B457: `{ file, memory: true }` (a browser without OPFS, such as a Firefox private window) writes it into a Blob
// instead, handed back as `pack` for this visit only.
import init, { import_disc, import_disc_to_memory } from './pkg/hst.js';

onmessage = async ({ data }) => {
  const { file, memory } = data instanceof Blob ? { file: data } : data;
  const progress = (i, n) => postMessage({ progress: [i, n] });
  try {
    await init();
    if (memory) {
      const parts = [];
      const [hash, head] = import_disc_to_memory(file, progress, c => parts.push(new Blob([c])));
      postMessage({ hash, pack: new Blob([head, ...parts]) });
      return;
    }
    const dir = await navigator.storage.getDirectory();
    const part = await dir.getFileHandle('disc.part', { create: true });
    const h = await part.createSyncAccessHandle();
    let hash;
    try {
      hash = import_disc(file, h, progress);
      h.flush();
    } finally {
      h.close();
    }
    // (directory, name): WebKit's move takes no name-only form ("TypeError: Not enough arguments")
    if (part.move) await part.move(dir, 'disc.pack');
    else {
      const out = await (await dir.getFileHandle('disc.pack', { create: true })).createWritable();
      await out.write(await part.getFile());
      await out.close();
      await dir.removeEntry('disc.part');
    }
    postMessage({ hash });
  } catch (e) {
    postMessage({ error: String(e) });
  }
};
