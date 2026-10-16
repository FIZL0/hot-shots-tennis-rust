// B452's check of the page's WebGPU error handling (web/index.html's boot script) alone (no game, no disc): Android
// Chrome rejected a pending popErrorScope with "Instance dropped in popErrorScope" and the page stopped the game on it.
// A stand-in GPUDevice whose popErrorScope rejects that way: before the first frame the page must ask for the WebGL2
// build (window.hstWebGL2, stubbed to record its call) and not stop; after it, keep running without a reload; a stray
// rejection of the same kind likewise; any other unhandled rejection must still stop the game with its message.
// Headless chromium. Prints PASS/FAIL per case; exits 1 on any FAIL.
// Usage: node tools/web_pop_error_scope.mjs
import { spawn } from 'node:child_process';
import { readFileSync, mkdtempSync } from 'node:fs';
import { createServer } from 'node:http';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

const html = readFileSync(new URL('../web/index.html', import.meta.url), 'utf8');
const markup = html.slice(html.indexOf('<div id="boot">'), html.indexOf('</div></div>', html.indexOf('<div id="boot">')) + 12);
const from = html.lastIndexOf('<script>', html.indexOf('// Boot status'));
const boot = html.slice(from, html.indexOf('</script>', from) + 9);

// a GPUDevice stand-in, defined before the boot script wraps its createShaderModule
const stub = `<script>
window.fellBack = '';
window.hstWebGL2 = why => { fellBack = String(why ?? 'yes'); return true; };
window.GPUDevice = class {
  pushErrorScope() {}
  popErrorScope() { return Promise.reject(new DOMException('Instance dropped in popErrorScope', 'OperationError')); }
  createShaderModule(d) { return { label: d.label }; }
};
</script>`;
const cases = {
  // the page's shader-module wrapper before the first frame / after it
  early: 'new GPUDevice().createShaderModule({ label: "s", code: "" });',
  late: 'hstShown(); new GPUDevice().createShaderModule({ label: "s", code: "" });',
  // a cancelled WebGPU promise nobody handles (not ours)
  stray: 'hstShown(); Promise.reject(new DOMException("Instance dropped in popErrorScope", "OperationError"));',
  // anything else unhandled still stops the game
  other: 'hstShown(); Promise.reject(new Error("rejected by a test script"));',
};
const want = {
  early: g => g.startsWith('running ') && g.includes('fell back: Instance dropped'),
  late: g => g.startsWith('running ') && g.includes('fell back: none'),
  stray: g => g.startsWith('running ') && g.includes('fell back: none'),
  other: g => g.startsWith('stopped ') && g.includes('rejected by a test script'),
};

const results = new Map();
const server = createServer((q, r) => {
  const u = new URL(q.url, 'http://x');
  if (u.pathname === '/done') {
    let body = '';
    q.on('data', c => (body += c)).on('end', () => { results.set(u.searchParams.get('k'), body); r.end(); });
    return;
  }
  r.writeHead(200, { 'Content-Type': 'text/html' }).end(`<!doctype html><meta charset="utf-8"><title>t</title>${markup}${stub}${boot}
<script>${cases[u.searchParams.get('case')]}</script>
<script>setTimeout(() => fetch('/done?k=' + encodeURIComponent(location.search), { method: 'POST',
  body: (boot.classList.contains('err') ? 'stopped' : 'running') + ' | fell back: ' + (fellBack || 'none') + ' | ' + step.textContent }), 300);</script>`);
}).listen(0, '127.0.0.1');
await new Promise(r => server.listening ? r() : server.on('listening', r));
const base = `http://127.0.0.1:${server.address().port}/t.html`;

const sleep = ms => new Promise(r => setTimeout(r, ms));
let failed = false;
for (const c of Object.keys(cases)) {
  const k = `?case=${c}`;
  const p = spawn('chromium', ['--headless=new', '--mute-audio', `--user-data-dir=${mkdtempSync(join(tmpdir(), 'hst-pes-'))}`,
    base + k], { stdio: 'ignore' });
  for (let i = 0; i < 100 && !results.has(k); i++) await sleep(100);
  p.kill();
  const got = results.get(k) ?? 'timed out';
  const ok = want[c](got);
  failed ||= !ok;
  console.log(`${c}: ${ok ? 'PASS' : 'FAIL'} (${got.slice(0, 160)})`);
}
server.close();
process.exit(failed ? 1 : 0);
