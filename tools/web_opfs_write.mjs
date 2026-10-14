// B358's check of the page's OPFS writes (web/index.html writeFile + web/opfs.js) alone (no game, no disc): headless
// chromium on a local server, the helper cut out of index.html. Run twice: with createWritable (Chromium's own path)
// and with it deleted from FileSystemFileHandle (Safari before 26 / iOS 18, the worker's sync access handle). Each
// run writes a 3 MB file in nested folders, a text file, then rewrites the text file shorter, all at once, and
// reads them back. Prints PASS/FAIL per run; exits 1 on any FAIL.
// Usage: node tools/web_opfs_write.mjs
import { spawn } from 'node:child_process';
import { readFileSync, mkdtempSync } from 'node:fs';
import { createServer } from 'node:http';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

const web = new URL('../web/', import.meta.url);
const html = readFileSync(new URL('index.html', web), 'utf8');
const helper = html.slice(html.indexOf('\nlet writer;'), html.indexOf('\n};\n', html.indexOf('const writeFile')) + 3);
const page = `<script type="module">
const dir = await navigator.storage.getDirectory();
for (const n of ['mods', 'previews']) await dir.removeEntry(n, { recursive: true }).catch(() => {});
if (location.hash === '#safari') delete FileSystemFileHandle.prototype.createWritable;
${helper}
const read = async path => { let d = dir; const p = path.split('/'); for (const s of p.slice(0, -1)) d = await d.getDirectoryHandle(s);
  return new Uint8Array(await (await (await d.getFileHandle(p.at(-1))).getFile()).arrayBuffer()); };
let out;
try {
  const big = new Uint8Array(3e6).map((_, i) => i & 255);
  await Promise.all([writeFile('mods/w/a/b/big.bin', big), writeFile('mods/w/mod.json', '{"id":"w","long":1}'), writeFile('previews/c.png', new Uint8Array([1, 2, 3]))]);
  await writeFile('mods/w/mod.json', '{}');
  const b = await read('mods/w/a/b/big.bin'), j = new TextDecoder().decode(await read('mods/w/mod.json')), c = await read('previews/c.png');
  const ok = b.length === 3e6 && b.every((v, i) => v === (i & 255)) && j === '{}' && c.join() === '1,2,3' && big.length === 3e6;
  out = (ok ? 'PASS' : 'FAIL') + \` createWritable=\${'createWritable' in FileSystemFileHandle.prototype} big=\${b.length} json=\${j} card=\${c.join()}\`;
} catch (e) { out = 'FAIL ' + e; }
document.title = 'DONE ' + out;
</script>`;
const server = createServer((q, r) => {
  if (q.url.startsWith('/opfs.js')) r.writeHead(200, { 'Content-Type': 'text/javascript' }).end(readFileSync(new URL('opfs.js', web)));
  else r.writeHead(200, { 'Content-Type': 'text/html' }).end(page);
}).listen(0, '127.0.0.1');
await new Promise(r => server.on('listening', r));
const url = `http://127.0.0.1:${server.address().port}/t.html`;

const CDP = process.env.HST_CDP_PORT ?? 9335;
const sleep = ms => new Promise(r => setTimeout(r, ms));
const chrome = spawn('chromium', ['--headless=new', '--mute-audio', `--remote-debugging-port=${CDP}`,
  `--user-data-dir=${mkdtempSync(join(tmpdir(), 'hst-opfs-'))}`, 'about:blank'], { stdio: 'ignore' });
process.on('exit', () => chrome.kill());
let targets;
for (let i = 0; i < 50 && !targets; i++) {
  await sleep(200);
  targets = await fetch(`http://127.0.0.1:${CDP}/json`).then(r => r.json()).catch(() => null);
}
const ws = new WebSocket(targets.find(t => t.type === 'page').webSocketDebuggerUrl);
await new Promise(r => (ws.onopen = r));
let id = 0;
const waiting = new Map();
ws.onmessage = ({ data }) => {
  const m = JSON.parse(data);
  if (m.id && waiting.has(m.id)) waiting.get(m.id)(m), waiting.delete(m.id);
};
const send = (method, params = {}) => new Promise(r => { waiting.set(++id, r); ws.send(JSON.stringify({ id, method, params })); });
const evaluate = async e => (await send('Runtime.evaluate', { expression: e, returnByValue: true })).result?.result?.value;

let failed = false;
for (const hash of ['', '#safari']) {
  await send('Page.navigate', { url: 'about:blank' });
  await sleep(200);
  await send('Page.navigate', { url: url + hash });
  let title = '';
  for (let i = 0; i < 100 && !title?.startsWith('DONE'); i++) { await sleep(200); title = await evaluate('document.title'); }
  const line = title?.startsWith('DONE') ? title.slice(5) : 'FAIL timed out';
  failed ||= !line.startsWith('PASS');
  console.log(`${hash ? 'worker (no createWritable)' : 'createWritable'}: ${line}`);
}
server.close();
process.exit(failed ? 1 : 0);
