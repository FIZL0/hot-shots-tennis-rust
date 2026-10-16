// B453: the browser build in WebKit (the system's WebKitGTK MiniBrowser: Safari's engine — JavaScriptCore, its wasm
// tiers, OPFS — on Linux) with what each frame cost, for hangs that only Safari shows. Serves web/ itself and adds a
// script to index.html that sends the page's console and every main-thread block back here: `raf gap N ms` (two
// animation frames that far apart) and `blocked N ms` (a 50 ms timer that late: the thread was busy, frame or not).
// WebKitGTK's OPFS has no sync access handles (the page's import), so a fresh profile gets the native build's disc pack
// (~/.local/share/hst/disc.pack, same format) written into OPFS first. Needs WAYLAND_DISPLAY or DISPLAY (MiniBrowser
// opens a window). Usage: node tools/web_webkit.mjs PACK OUT [query] [seconds]
// KEYS="20:Enter,26:ArrowDown" presses keys at those seconds after the game's first frame. OUT/console.txt: the log,
// each line with ms since the page loaded. PROFILE=dir keeps WebKit's data (OPFS: the imported disc) elsewhere than
// OUT/profile, so later runs skip writing the pack. WEB_PORT picks the port (8766). WK_FEATURES: MiniBrowser's --features
// (default: OPFS on, which Safari has and WebKitGTK ships off; add ',+WebGPU' for its WebGPU).
import { spawn } from 'node:child_process';
import { createReadStream, mkdirSync, readFileSync, statSync, writeFileSync } from 'node:fs';
import { createServer } from 'node:http';
import { extname, join, resolve } from 'node:path';

const [pack, out, query = 'fps&webgl2', secs = '90'] = process.argv.slice(2);
mkdirSync(out, { recursive: true });
const web = resolve(new URL('../web', import.meta.url).pathname);
const port = +(process.env.WEB_PORT ?? 8766);
const keys = (process.env.KEYS ?? '').split(',').filter(Boolean).map(k => k.split(':')).map(([s, key]) => [+s, key]);
const log = [];
const types = { '.html': 'text/html', '.js': 'text/javascript', '.mjs': 'text/javascript', '.wasm': 'application/wasm', '.json': 'application/json', '.css': 'text/css', '.svg': 'image/svg+xml', '.png': 'image/png', '.webmanifest': 'application/manifest+json' };

// runs before the page's own scripts
const probe = `<script>
(() => {
  const t0 = performance.now(), buf = [];
  const send = () => { if (buf.length) fetch('/__log', { method: 'POST', body: buf.splice(0).join('\\n'), keepalive: true }).catch(() => {}); };
  const add = (type, args) => buf.push(\`\${Math.round(performance.now() - t0)}ms \${type}: \${[...args].map(a => typeof a === 'string' ? a.replace(/%c/g, '') : a instanceof Error ? a.stack || a.message : (() => { try { return JSON.stringify(a); } catch { return String(a); } })()).filter(a => !/^(color|font-style):/.test(a)).join(' ')}\`);
  for (const k of ['log', 'info', 'warn', 'error', 'debug']) { const f = console[k].bind(console); console[k] = (...a) => { add(k, a); f(...a); }; }
  addEventListener('error', e => add('exception', [e.message + ' ' + (e.filename ?? '') + ':' + (e.lineno ?? '')]));
  addEventListener('unhandledrejection', e => add('rejection', [String(e.reason) + ' ' + (e.reason?.stack ?? '')]));
  setInterval(send, 500);
  addEventListener('pagehide', send);
  try { localStorage.setItem('hst-autoplay', '1'); } catch {}
  let last = performance.now();
  const raf = t => { if (t - last > 100) add('probe', [\`raf gap \${Math.round(t - last)} ms\`]); last = t; requestAnimationFrame(raf); };
  requestAnimationFrame(raf);
  let beat = performance.now();
  setInterval(() => { const n = performance.now(); if (n - beat > 250) add('probe', [\`blocked \${Math.round(n - beat - 50)} ms\`]); beat = n; }, 50);
  // the boot and import status lines as they change
  let said;
  setInterval(() => { const s = [document.getElementById('status'), document.getElementById('step')].map(e => e?.textContent ?? '').join(' | '); if (s !== said) add('status', [said = s]); }, 1000);
  // keys from the game's first frame on
  const keys = ${JSON.stringify(keys)};
  const wait = setInterval(() => {
    const c = document.querySelector('canvas');
    if (!c) return;
    clearInterval(wait);
    add('probe', ['canvas up, keys start']);
    for (const [s, key] of keys) setTimeout(() => {
      add('probe', [\`key \${key}\`]);
      const code = key.length === 1 ? 'Key' + key.toUpperCase() : key;
      c.focus();
      c.dispatchEvent(new KeyboardEvent('keydown', { key, code, bubbles: true }));
      setTimeout(() => c.dispatchEvent(new KeyboardEvent('keyup', { key, code, bubbles: true })), 80);
    }, s * 1000);
  }, 200);
})();
</script>`;

const server = createServer((q, r) => {
  const u = new URL(q.url, 'http://x');
  if (u.pathname === '/__log') {
    let body = '';
    q.on('data', c => (body += c)).on('end', () => { log.push(...body.split('\n')); r.end(); });
    return;
  }
  if (u.pathname === '/__pack') return createReadStream(pack).pipe(r.writeHead(200, { 'Content-Type': 'application/octet-stream', 'Content-Length': statSync(pack).size }));
  // OPFS disc.pack from the native one unless it's there already, then the game
  if (u.pathname === '/__setup') return r.writeHead(200, { 'Content-Type': 'text/html' }).end(`<script type="module">
    const log = t => fetch('/__log', { method: 'POST', body: 'setup: ' + t });
    try {
      const dir = await navigator.storage.getDirectory();
      const have = await dir.getFileHandle('disc.pack').then(h => h.getFile()).then(f => f.size, () => 0);
      if (have !== ${statSync(pack).size}) {
        const w = await (await dir.getFileHandle('disc.pack', { create: true })).createWritable();
        await (await fetch('/__pack')).body.pipeTo(w);
        await log('pack written');
      }
      location = '/?${query}';
    } catch (e) { await log(String(e)); }
  </script>`);
  const path = join(web, u.pathname === '/' ? 'index.html' : decodeURIComponent(u.pathname));
  if (!path.startsWith(web)) return r.writeHead(403).end();
  try {
    let body = readFileSync(path);
    if (path.endsWith('index.html')) body = body.toString().replace('<head>', `<head>${probe}`);
    r.writeHead(200, { 'Content-Type': types[extname(path)] ?? 'application/octet-stream', 'Cache-Control': 'no-store' }).end(body);
  } catch {
    r.writeHead(404).end();
  }
}).listen(port, '127.0.0.1');

const data = resolve(process.env.PROFILE ?? join(out, 'profile'));
const browser = spawn('/usr/lib/webkit2gtk-4.1/MiniBrowser', ['--autoplay-policy=allow', `--features=${process.env.WK_FEATURES ?? '+StorageAPI,+StorageAPIEstimate,+FileSystem,+FileSystemWritableStream,+AccessHandle'}`, `http://localhost:${port}/__setup`], {
  stdio: 'ignore',
  env: { ...process.env, XDG_DATA_HOME: join(data, 'data'), XDG_CACHE_HOME: join(data, 'cache'), XDG_CONFIG_HOME: join(data, 'config') },
});
const stop = () => { browser.kill(); server.close(); writeFileSync(join(out, 'console.txt'), log.join('\n') + '\n'); process.exit(0); };
for (const s of ['SIGINT', 'SIGTERM', 'SIGHUP']) process.on(s, stop);
setTimeout(stop, +secs * 1000);
