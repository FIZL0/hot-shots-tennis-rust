// B456: the web build in Zen (Firefox) over WebDriver BiDi, the way web_check.mjs drives chromium over CDP: visit 1
// picks the ISO (import into OPFS), visit 2 (same profile) must start the game. Console + screenshots into OUT.
// Usage: node tools/web_zen.mjs ISO OUT [query] [seconds]  (serve web/ on :8765 first: python3 -m http.server -d web
// 8765; WEB_PORT/BIDI_PORT pick other ports). ISO `-`: OUT's profile already has the pack, only visit 2 runs.
// SHOT_EVERY=N a screenshot every N s (10). WEB_PRELOAD=js runs before the page's scripts (as web_check.mjs's; M5a put
// texture packs into OPFS with it). ZEN=path another Firefox-family browser. Headless; the page picks WebGL2 on Gecko unless the renderer says otherwise.
import { spawn } from 'node:child_process';
import { mkdirSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';

const [iso, out, query = 'play', secs = '40'] = process.argv.slice(2);
const every = +(process.env.SHOT_EVERY ?? 10), web = process.env.WEB_PORT ?? 8765, port = process.env.BIDI_PORT ?? 9334;
const sleep = ms => new Promise(r => setTimeout(r, ms));
mkdirSync(`${out}/profile`, { recursive: true });
// headless windows count as hidden: Gecko throttles their frames to 1 a second unless told otherwise
writeFileSync(`${out}/profile/user.js`, ['layout.frame_rate', 60, 'layout.throttled_frame_rate', 60, 'dom.timeout.enable_budget_throttling', false, 'dom.min_background_timeout_value', 4, 'webgl.force-enabled', true, 'dom.webgpu.enabled', true, 'gfx.webgpu.ignore-blocklist', true,
  'browser.shell.checkDefaultBrowser', false].reduce((a, v, i, l) => i % 2 ? a : a + `user_pref("${v}", ${l[i + 1]});\n`, ''));
const zen = spawn(process.env.ZEN ?? 'zen-browser', ['--headless', '--no-remote', '--profile', resolve(`${out}/profile`),
  '--remote-debugging-port', `${port}`, '--window-size', '1280,720', 'about:blank'], { stdio: 'ignore' });
const kill = () => { try { zen.kill('SIGKILL'); } catch {} };
process.on('exit', kill);
setTimeout(() => { log.push('TIMEOUT'); finish(1); }, (+secs * 2 + 600) * 1000).unref();

const log = [];
let ws;
for (let i = 0; i < 100 && !ws; i++) {
  await sleep(300);
  ws = await new Promise(r => { const s = new WebSocket(`ws://127.0.0.1:${port}/session`); s.onopen = () => r(s); s.onerror = () => r(null); });
}
if (!ws) { console.log('no BiDi socket'); process.exit(1); }
let id = 0;
const pending = new Map();
ws.onmessage = ({ data }) => {
  const m = JSON.parse(data);
  if (m.id !== undefined) { pending.get(m.id)?.(m); pending.delete(m.id); }
  else if (m.method === 'log.entryAdded') log.push(`[${m.params.level}] ${m.params.text}`);
};
const send = (method, params = {}) => new Promise(r => { const i = id++; pending.set(i, r); ws.send(JSON.stringify({ id: i, method, params })); });
const ok = async (method, params) => { const m = await send(method, params); if (m.type === 'error') throw new Error(`${method}: ${m.error} ${m.message}`); return m.result; };

await ok('session.new', { capabilities: {} });
await ok('session.subscribe', { events: ['log.entryAdded'] });
// the site's autoplay box ticked (straight into the game once a disc is in), the page's console, a count of the page's animation frames, logged with each screenshot (a black page that still draws vs a stopped one)
const renderer = process.env.RENDERER ? `try { const s = (localStorage.getItem(\'./settings.txt\') ?? \'\').replace(/^renderer.*\\n?/m, \'\'); localStorage.setItem(\'./settings.txt\', \'renderer = ${process.env.RENDERER}\\n\' + s); } catch {}` : '';
await ok('script.addPreloadScript', { functionDeclaration: '() => {' + renderer + (process.env.WEB_PRELOAD ?? '') + '; window.__log = []; for (const k of [\'log\', \'info\', \'warn\', \'error\']) { const f = console[k]; console[k] = (...a) => { window.__log.push(`[page ${k}] ${a.join(\' \')}`); f.apply(console, a); }; } try { localStorage.setItem(\'hst-autoplay\', \'1\'); } catch {} window.__frames = 0; const f = () => { window.__frames++; requestAnimationFrame(f); }; requestAnimationFrame(f); }' });
const ctx = (await ok('browsingContext.getTree', {})).contexts[0].context;
const evaluate = async expr => (await send('script.evaluate', { expression: expr, target: { context: ctx }, awaitPromise: true })).result?.result?.value;
const shot = async name => writeFileSync(`${out}/${name}.png`, Buffer.from((await ok('browsingContext.captureScreenshot', { context: ctx })).data, 'base64'));

async function finish(code) {
  writeFileSync(`${out}/console.txt`, log.join('\n') + '\n');
  kill();
  process.exit(code);
}

async function visit(n, pick) {
  log.push(`--- visit ${n}`);
  await ok('browsingContext.navigate', { context: ctx, url: `http://localhost:${web}/?${query}`, wait: 'interactive' });
  // a headless tab starts out hidden (rAF at 1 Hz) until it's activated
  await ok('browsingContext.activate', { context: ctx });
  if (pick) {
    // M107: the site's text shows from the start; wait for site.js's picker handler, not just a visible #disc
    for (let i = 0; i < 100 && !(await evaluate("!document.getElementById('disc')?.hidden && !!document.getElementById('iso')?.onchange")); i++) await sleep(200);
    const el = (await ok('script.evaluate', { expression: "document.getElementById('iso')", target: { context: ctx }, awaitPromise: false, resultOwnership: 'root' })).result;
    await ok('input.setFiles', { context: ctx, element: { sharedId: el.sharedId }, files: [resolve(iso)] });
    for (let i = 0; i < 600; i++) {
      const s = await evaluate("document.getElementById('status').textContent");
      if (i % 25 == 0) log.push(`[${n}] status: ${s}`);
      if (/Imported|Couldn't/.test(s)) { log.push(`[${n}] status: ${s}`); break; }
      await sleep(1000);
    }
    return;
  }
  for (let t = 1; t <= +secs; t++) {
    await sleep(1000);
    if (t % every == 0 || t == +secs) { await shot(`visit${n}-${t}s`); log.push(`[${n}] ${t}s frames ${await evaluate('window.__frames')}`); }
  }
  log.push(`[${n}] status: ${await evaluate("document.getElementById('status')?.textContent")}`, `[${n}] ${await evaluate('document.visibilityState')} url ${await evaluate('location.href')} text: ${(await evaluate('document.body.innerText'))?.slice(0, 300)}`);
  // the page's own console (BiDi's log events miss some of it)
  log.push(...(await evaluate('JSON.stringify(window.__log)').then(JSON.parse).catch(() => ['(no page log)'])));
}

if (iso !== '-') await visit(1, true);
await visit(2, false);
await finish(0);
