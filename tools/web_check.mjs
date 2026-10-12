// Drives headless chromium over CDP: visit 1 picks the ISO (import into OPFS), visit 2 (same profile, no file) must
// start the match. Screenshots + console into OUT. Usage: node tools/web_check.mjs ISO OUT [query] [seconds]
// (serve web/ on :8765 first: python3 -m http.server -d web 8765).
import { spawn } from 'node:child_process';
import { writeFileSync, mkdirSync } from 'node:fs';
import { resolve } from 'node:path';

const [iso, out, query = 'play', secs = '60'] = process.argv.slice(2);
mkdirSync(out, { recursive: true });
const log = [];
// HST_WEB_PORT / HST_CDP_PORT: other ports (runs side by side)
const WEB = process.env.HST_WEB_PORT ?? 8765, CDP = process.env.HST_CDP_PORT ?? 9333;
const sleep = ms => new Promise(r => setTimeout(r, ms));

async function visit(n, pick) {
  const chrome = spawn('chromium', ['--headless=new', `--remote-debugging-port=${CDP}`, `--user-data-dir=${resolve(out, 'profile')}`,
    '--enable-unsafe-webgpu', '--ignore-gpu-blocklist', '--window-size=1280,720', 'about:blank'], { stdio: 'ignore' });
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
    if (m.method === 'Runtime.consoleAPICalled') log.push(`[${n}] ${m.params.type}: ${m.params.args.map(a => a.value ?? a.description).join(' ')}`);
    if (m.method === 'Runtime.exceptionThrown') log.push(`[${n}] exception: ${m.params.exceptionDetails.exception?.description ?? m.params.exceptionDetails.text}`);
  };
  const send = (method, params = {}) => new Promise(r => { waiting.set(++id, r); ws.send(JSON.stringify({ id, method, params })); });
  const evaluate = async e => (await send('Runtime.evaluate', { expression: e, returnByValue: true, awaitPromise: true })).result?.result?.value;
  await send('Runtime.enable');
  await send('Page.enable');
  await send('Page.navigate', { url: `http://localhost:${WEB}/?${query}` });
  if (pick) {
    for (let i = 0; i < 100 && !(await evaluate("!document.getElementById('disc')?.hidden")); i++) await sleep(200);
    const doc = await send('DOM.getDocument');
    const node = await send('DOM.querySelector', { nodeId: doc.result.root.nodeId, selector: '#iso' });
    await send('DOM.setFileInputFiles', { nodeId: node.result.nodeId, files: [resolve(iso)] });
    for (let i = 0; i < 600; i++) {
      const s = await evaluate("document.getElementById('status').textContent");
      if (i % 25 == 0) log.push(`[${n}] status: ${s}`);
      if (/Imported|Couldn't/.test(s)) { log.push(`[${n}] status: ${s}`); break; }
      await sleep(1000);
    }
  }
  for (let t = 0; t < +secs; t += 10) {
    await sleep(10000);
    const shot = await send('Page.captureScreenshot', { format: 'png' });
    writeFileSync(resolve(out, `visit${n}-${t + 10}s.png`), Buffer.from(shot.result.data, 'base64'));
  }
  log.push(`[${n}] canvas: ${await evaluate("JSON.stringify([...document.querySelectorAll('canvas')].map(c => [c.width, c.height]))")}`);
  log.push(`[${n}] opfs: ${await evaluate("navigator.storage.getDirectory().then(async d => { const r = []; for await (const [k, h] of d) r.push(k + ' ' + (await h.getFile()).size); return r.join(', '); })")}`);
  ws.close();
  chrome.kill();
  await sleep(1000);
}

await visit(1, true);
await visit(2, false);
writeFileSync(resolve(out, 'console.txt'), log.join('\n') + '\n');
console.log(log.slice(-40).join('\n'));
