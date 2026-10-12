// Drives headless chromium over CDP: visit 1 picks the ISO (import into OPFS), visit 2 (same profile, no file) must
// start the match. Screenshots + console into OUT. Usage: node tools/web_check.mjs ISO OUT [query] [seconds]
// (serve web/ on :8765 first: python3 -m http.server -d web 8765; WEB_PORT/CDP_PORT pick other ports). ISO `-`: OUT's
// profile already has the pack, only visit 2 runs. WEB_INPUT="15:click,20:Enter,21:ArrowDown" presses at those seconds of
// visit 2 (click = a tap in the middle); the audio contexts' state and the loudest sample handed to them are logged.
// WEB_STALL=s:ms blocks the page's main thread for ms at second s of visit 2; the audio log then also gives `rate`: seconds of
// audio buffers started per second of the context's clock since the previous log (1 = real time, 2 = buffers overlapping).
// PROFILE=dir another chromium profile. OUT/framesN.json: every rAF frame of visit N as [ms since load, frame ms]. SHOT_EVERY=s screenshots more often. CHROME_FLAGS adds flags (e.g. "--enable-features=Vulkan" for WebGPU on SwiftShader).
import { spawn } from 'node:child_process';
import { writeFileSync, mkdirSync } from 'node:fs';
import { resolve } from 'node:path';

const [iso, out, query = 'play', secs = '60'] = process.argv.slice(2);
mkdirSync(out, { recursive: true });
const log = [];
const web = process.env.WEB_PORT ?? 8765, cdp = process.env.CDP_PORT ?? 9333;
const sleep = ms => new Promise(r => setTimeout(r, ms));

async function visit(n, pick) {
  const chrome = spawn('chromium', ['--headless=new', `--remote-debugging-port=${cdp}`, `--user-data-dir=${resolve(process.env.PROFILE ?? resolve(out, 'profile'))}`,
    '--enable-unsafe-webgpu', '--ignore-gpu-blocklist', '--window-size=1280,720', ...(process.env.CHROME_FLAGS ?? '').split(' ').filter(Boolean), 'about:blank'], { stdio: 'ignore' });
  let targets;
  for (let i = 0; i < 50 && !targets; i++) {
    await sleep(200);
    targets = await fetch(`http://127.0.0.1:${cdp}/json`).then(r => r.json()).catch(() => null);
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
  await send('Network.enable');
  await send('Page.enable');
  await send('Network.setCacheDisabled', { cacheDisabled: true });
  await send('Page.addScriptToEvaluateOnNewDocument', { source: `
    self.__ctx = []; self.__peak = 0; self.__frames = [];
    { let last; const f = t => { if (last !== undefined) __frames.push([Math.round(performance.now()), +(t - last).toFixed(1)]); last = t; requestAnimationFrame(f); }; requestAnimationFrame(f); }
    self.AudioContext = new Proxy(self.AudioContext, { construct: (t, a, nt) => { const c = Reflect.construct(t, a, nt); __ctx.push(c); return c; } });
    const copy = AudioBuffer.prototype.copyToChannel;
    AudioBuffer.prototype.copyToChannel = function (d, ...r) { for (const x of d) __peak = Math.max(__peak, Math.abs(x)); return copy.call(this, d, ...r); };
    self.__played = 0; self.__rate = [0, 0];
    const start = AudioBufferSourceNode.prototype.start;
    AudioBufferSourceNode.prototype.start = function (...a) { __played += this.buffer?.duration ?? 0; (self.__starts ??= []).push([+(a[0] ?? 0).toFixed(3), +this.context.currentTime.toFixed(3), +(this.buffer?.duration ?? 0).toFixed(4)]); return start.apply(this, a); };
    self.__audioRate = () => { const t = __ctx[0]?.currentTime ?? 0, r = (__played - __rate[0]) / Math.max(t - __rate[1], 1e-9); __rate = [__played, t]; return +r.toFixed(2); };
` });
  await send('Page.navigate', { url: `http://localhost:${web}/?${query}` });
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
  const input = pick ? [] : (process.env.WEB_INPUT ?? '').split(',').filter(Boolean).map(e => e.split(':'));
  const audio = () => evaluate("JSON.stringify([__ctx.map(c => c.state), __peak, 'rate', __audioRate()])");
  const [stallAt, stallMs] = (pick ? '' : process.env.WEB_STALL ?? '').split(':').map(Number);
  for (let t = 1; t <= +secs; t++) {
    await sleep(1000);
    for (const [at, key] of input.filter(([at]) => +at === t)) {
      if (key === 'click') {
        log.push(`[${n}] ${t}s click, audio before: ${await audio()}`);
        for (const type of ['mousePressed', 'mouseReleased']) await send('Input.dispatchMouseEvent', { type, x: 640, y: 300, button: 'left', clickCount: 1 });
      } else for (const type of ['rawKeyDown', 'keyUp']) {
        await send('Input.dispatchKeyEvent', { type, key, code: key.length === 1 ? `Key${key.toUpperCase()}` : key, windowsVirtualKeyCode: { Enter: 13, ArrowDown: 40, ArrowUp: 38, ArrowLeft: 37, ArrowRight: 39, Escape: 27, Space: 32 }[key] ?? key.toUpperCase().charCodeAt(0) });
        await sleep(80);
      }
    }
    if (t === stallAt) {
      await evaluate(`{ const end = performance.now() + ${stallMs}; while (performance.now() < end); }`);
      log.push(`[${n}] ${t}s stalled ${stallMs} ms`);
    }
    if (t % (+process.env.SHOT_EVERY || 10) === 0) {
      const shot = await send('Page.captureScreenshot', { format: 'png' });
      writeFileSync(resolve(out, `visit${n}-${t}s.png`), Buffer.from(shot.result.data, 'base64'));
      log.push(`[${n}] ${t}s audio: ${await audio()}`);
    }
  }
  if (stallMs) log.push(`[${n}] buffers started [when, context time, length]: ${await evaluate('JSON.stringify(__starts)')}`);
  writeFileSync(resolve(out, `frames${n}.json`), await evaluate('JSON.stringify(__frames)') ?? '[]');
  log.push(`[${n}] canvas: ${await evaluate("JSON.stringify([...document.querySelectorAll('canvas')].map(c => [c.width, c.height]))")}`);
  log.push(`[${n}] opfs: ${await evaluate("navigator.storage.getDirectory().then(async d => { const r = []; for await (const [k, h] of d) r.push(k + ' ' + (await h.getFile()).size); return r.join(', '); })")}`);
  ws.close();
  chrome.kill();
  await sleep(1000);
}

if (iso !== '-') await visit(1, true);
await visit(2, false);
writeFileSync(resolve(out, 'console.txt'), log.join('\n') + '\n');
console.log(log.slice(-40).join('\n'));
