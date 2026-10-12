// Imports the picked ISO into OPFS `disc.pack` with the game's own pack writer (sync file APIs: workers only).
import init, { import_disc } from './pkg/hst.js';

onmessage = async ({ data: file }) => {
  try {
    await init();
    const dir = await navigator.storage.getDirectory();
    const part = await dir.getFileHandle('disc.part', { create: true });
    const h = await part.createSyncAccessHandle();
    let hash;
    try {
      hash = import_disc(file, h, (i, n) => postMessage({ progress: [i, n] }));
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
