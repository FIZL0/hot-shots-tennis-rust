// B330's check of the page's error handler (web/index.html's boot script) alone (no game, no disc): a script from
// another origin that throws (what iOS Safari's share extensions do when its Share button is tapped) reaches the page
// as a bare "Script error." and must leave it running; a same-origin one that throws must still stop it with its
// message. Each page reports back to the local server. Headless chromium always; WebKit too with `webkit` as the
// argument (the system's WebKitGTK MiniBrowser, which opens a window: needs WAYLAND_DISPLAY or DISPLAY).
// Prints PASS/FAIL per case and browser; exits 1 on any FAIL.
// Usage: node tools/web_script_error.mjs [webkit]
import { spawn } from 'node:child_process';
import { readFileSync, mkdtempSync } from 'node:fs';
import { createServer } from 'node:http';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

const html = readFileSync(new URL('../web/index.html', import.meta.url), 'utf8');
const markup = html.slice(html.indexOf('<div id="boot">'), html.indexOf('</div></div>', html.indexOf('<div id="boot">')) + 12);
const from = html.lastIndexOf('<script>', html.indexOf('// Boot status'));
const boot = html.slice(from, html.indexOf('</script>', from) + 9);
const thrower = 'throw new Error("thrown by a test script");';

const results = new Map();
const other = createServer((q, r) => r.writeHead(200, { 'Content-Type': 'text/javascript' }).end(thrower)).listen(0, '127.0.0.1');
const server = createServer((q, r) => {
  const u = new URL(q.url, 'http://x');
  if (u.pathname === '/x.js') return r.writeHead(200, { 'Content-Type': 'text/javascript' }).end(thrower);
  if (u.pathname === '/done') {
    let body = '';
    q.on('data', c => (body += c)).on('end', () => { results.set(u.searchParams.get('k'), body); r.end(); });
    return;
  }
  const src = u.searchParams.get('case') === 'cross' ? `http://127.0.0.1:${other.address().port}/x.js` : '/x.js';
  r.writeHead(200, { 'Content-Type': 'text/html' }).end(`<!doctype html><meta charset="utf-8"><title>t</title>${markup}${boot}
<script src="${src}"></script>
<script>setTimeout(() => fetch('/done?k=' + encodeURIComponent(location.search), { method: 'POST',
  body: (boot.classList.contains('err') ? 'stopped' : 'running') + ' | ' + step.textContent }), 300);</script>`);
}).listen(0, '127.0.0.1');
await Promise.all([server, other].map(s => new Promise(r => s.listening ? r() : s.on('listening', r))));
const base = `http://127.0.0.1:${server.address().port}/t.html`;

const sleep = ms => new Promise(r => setTimeout(r, ms));
const browsers = [['chromium', url => spawn('chromium', ['--headless=new', '--mute-audio',
  `--user-data-dir=${mkdtempSync(join(tmpdir(), 'hst-err-'))}`, url], { stdio: 'ignore' })]];
if (process.argv[2] === 'webkit') browsers.push(['webkit', url => spawn('/usr/lib/webkit2gtk-4.1/MiniBrowser', [url], { stdio: 'ignore' })]);
let failed = false;
for (const [name, open] of browsers) {
  for (const [c, want] of [['cross', 'running'], ['same', 'stopped']]) {
    const k = `?case=${c}&b=${name}`;
    const p = open(base + k);
    for (let i = 0; i < 100 && !results.has(k); i++) await sleep(100);
    p.kill();
    const got = results.get(k) ?? 'timed out';
    const ok = got.startsWith(want + ' ') && (c === 'cross' || got.includes('thrown by a test script'));
    failed ||= !ok;
    console.log(`${name} ${c}-origin throw: ${ok ? 'PASS' : 'FAIL'} (${got})`);
  }
}
server.close();
other.close();
process.exit(failed ? 1 : 0);
