// Drives headless chromium over CDP: visit 1 picks the ISO (import into OPFS), visit 2 (same profile, no file) must
// start the match. Screenshots + console into OUT. Usage: node tools/web_check.mjs ISO OUT [query] [seconds]
// (serve web/ on :8765 first: python3 -m http.server -d web 8765; WEB_PORT/CDP_PORT pick other ports). ISO `-`: OUT's
// profile already has the pack, only visit 2 runs. WEB_INPUT="15:click,20:Enter,21:ArrowDown" presses at those seconds of
// visit 2 (click = a tap in the middle); the audio contexts' state and the loudest sample handed to them are logged.
// WEB_STALL=s:ms blocks the page's main thread for ms at second s of visit 2; the audio log then also gives `rate`: seconds of
// audio buffers started per second of the context's clock since the previous log (1 = real time, 2 = buffers overlapping).
// PROFILE=dir another chromium profile. THROTTLE=n slows the page's CPU n× (a phone: 4–6) on visit 2; console lines then carry ms since load. MOBILE=1 a phone-sized touch screen. WEB_PRELOAD=js runs before the page's scripts (e.g. make a browser API hang). OUT/framesN.json: every rAF frame of visit N as [ms since load, frame ms]. SHOT_EVERY=s screenshots more often. WEB_ROTATE=ms turns a touch phone viewport portrait ↔ landscape every ms. WEB_CARDS=1 copies the baked select cards (OPFS previews/) to OUT/cards/. CHROME_FLAGS adds flags (e.g. "--enable-features=Vulkan" for WebGPU on SwiftShader). WEB_CPUPROFILE=s:n records the page's CPU profile from second s for n seconds of visit 2 into OUT/visit2.cpuprofile (B273; summed by research/b273_profile.py). WEB_HIDE=s:ms minimises the window at second s of visit 2 for ms (B260, below). WEB_TRACE=s:n logs the page's main-thread CPU ms per frame over n seconds from second s of visit 2 (thread time: steady under other load; B345).
import { spawn } from 'node:child_process';
import { writeFileSync, mkdirSync, readdirSync, readFileSync } from 'node:fs';
import { resolve } from 'node:path';

const [iso, out, query = 'play', secs = '60'] = process.argv.slice(2);
mkdirSync(out, { recursive: true });
const log = [];
let t0 = 0;
const web = process.env.WEB_PORT ?? 8765, cdp = process.env.CDP_PORT ?? 9333;
const sleep = ms => new Promise(r => setTimeout(r, ms));
// every chromium we start dies with this script (a crash, timeout or kill would otherwise leave a hidden hst playing music)
const kids = new Set();
process.on('exit', () => kids.forEach(c => c.kill()));
for (const s of ['SIGINT', 'SIGTERM', 'SIGHUP']) process.on(s, () => process.exit(1));

async function visit(n, pick) {
  const chrome = spawn('chromium', ['--headless=new', '--mute-audio', `--remote-debugging-port=${cdp}`, `--user-data-dir=${resolve(process.env.PROFILE ?? resolve(out, 'profile'))}`,
    '--enable-unsafe-webgpu', '--ignore-gpu-blocklist', '--window-size=1280,720', ...(process.env.CHROME_FLAGS ?? '').split(' ').filter(Boolean), 'about:blank'], { stdio: 'ignore' });
  kids.add(chrome);
  let targets;
  for (let i = 0; i < 50 && !targets; i++) {
    await sleep(200);
    targets = await fetch(`http://127.0.0.1:${cdp}/json`).then(r => r.json()).catch(() => null);
  }
  const ws = new WebSocket(targets.find(t => t.type === 'page').webSocketDebuggerUrl);
  await new Promise(r => (ws.onopen = r));
  let id = 0;
  const waiting = new Map();
  const trace = [];
  let traced, instr;
  ws.onmessage = ({ data }) => {
    const m = JSON.parse(data);
    if (m.id && waiting.has(m.id)) waiting.get(m.id)(m), waiting.delete(m.id);
    if (m.method === 'Runtime.consoleAPICalled') log.push(`[${n}]${t0 ? ' ' + (Date.now() - t0) + 'ms' : ''} ${m.params.type}: ${m.params.args.map(a => a.value ?? a.description).join(' ')}`);
    if (m.method === 'Tracing.dataCollected') trace.push(...m.params.value);
    if (m.method === 'Tracing.tracingComplete') traced?.();
    if (m.method === 'Runtime.exceptionThrown') log.push(`[${n}] exception: ${m.params.exceptionDetails.exception?.description ?? m.params.exceptionDetails.text}`);
  };
  const send = (method, params = {}) => new Promise(r => { waiting.set(++id, r); ws.send(JSON.stringify({ id, method, params })); });
  const evaluate = async e => (await send('Runtime.evaluate', { expression: e, returnByValue: true, awaitPromise: true })).result?.result?.value;
  await send('Runtime.enable');
  await send('Network.enable');
  await send('Page.enable');
  await send('Network.setCacheDisabled', { cacheDisabled: true });
  // MOBILE=1: a phone held sideways, touch only (as web_touch.mjs): the page's phone settings
  if (process.env.MOBILE) {
    await send('Emulation.setDeviceMetricsOverride', { width: 844, height: 390, deviceScaleFactor: 2, mobile: true, screenOrientation: { type: 'landscapePrimary', angle: 90 } });
    await send('Emulation.setTouchEmulationEnabled', { enabled: true, maxTouchPoints: 5 });
  }
  await send('Page.addScriptToEvaluateOnNewDocument', { source: `
    self.__ctx = []; self.__peak = 0; self.__frames = [];
    ${process.env.WEB_PRELOAD ?? ''}
    { let last; const f = t => { if (last !== undefined) __frames.push([Math.round(performance.now()), +(t - last).toFixed(1)]); last = t; requestAnimationFrame(f); }; requestAnimationFrame(f); }
    self.AudioContext = new Proxy(self.AudioContext, { construct: (t, a, nt) => { const c = Reflect.construct(t, a, nt); __ctx.push(c); return c; } });
    const copy = AudioBuffer.prototype.copyToChannel;
    AudioBuffer.prototype.copyToChannel = function (d, ...r) { for (const x of d) __peak = Math.max(__peak, Math.abs(x)); return copy.call(this, d, ...r); };
    self.__played = 0; self.__rate = [0, 0];
    const start = AudioBufferSourceNode.prototype.start;
    AudioBufferSourceNode.prototype.start = function (...a) { __played += this.buffer?.duration ?? 0; (self.__starts ??= []).push([+(a[0] ?? 0).toFixed(3), +this.context.currentTime.toFixed(3), +(this.buffer?.duration ?? 0).toFixed(4)]); return start.apply(this, a); };
    self.__audioRate = () => { const t = __ctx[0]?.currentTime ?? 0, r = (__played - __rate[0]) / Math.max(t - __rate[1], 1e-9); __rate = [__played, t]; return +r.toFixed(2); };
` });
  // WEB_ROTATE=ms: a phone (touch) turned portrait ↔ landscape every ms from the first navigation on (B121)
  const rotate = +process.env.WEB_ROTATE || 0;
  let turned = false, turns = 0;
  const turn = () => send('Emulation.setDeviceMetricsOverride', turned ? { width: 390, height: 844, deviceScaleFactor: 3, mobile: true, screenOrientation: { type: 'portraitPrimary', angle: 0 } } : { width: 844, height: 390, deviceScaleFactor: 3, mobile: true, screenOrientation: { type: 'landscapePrimary', angle: 90 } }).then(() => { turned = !turned; turns++; });
  if (rotate) { await send('Emulation.setTouchEmulationEnabled', { enabled: true, maxTouchPoints: 5 }); await send('Emulation.setEmitTouchEventsForMouse', { enabled: true }); await turn(); }
  const rotating = rotate && setInterval(turn, rotate);
  if (process.env.THROTTLE && !pick) await send('Emulation.setCPUThrottlingRate', { rate: +process.env.THROTTLE });
  t0 = process.env.THROTTLE ? Date.now() : 0;
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
    if (!pick && process.env.WEB_CPUPROFILE) {
      const [at, len] = process.env.WEB_CPUPROFILE.split(':').map(Number);
      if (t === at) { await send('Profiler.enable'); await send('Profiler.setSamplingInterval', { interval: 1000 }); await send('Profiler.start'); }
      if (t === at + len) { const r = await send('Profiler.stop'); if (r.result?.profile) writeFileSync(resolve(out, 'visit2.cpuprofile'), JSON.stringify(r.result.profile)); else log.push(`[${n}] cpu profile: ${JSON.stringify(r).slice(0, 300)}`); }
    }
    // WEB_TRACE=s:n (B345): a Chrome trace from second s for n seconds of visit 2; logs the page's main-thread CPU time
    // (thread time: not the time other processes or the throttle kept it off the core) per animation frame
    if (!pick && process.env.WEB_TRACE) {
      const [at, len] = process.env.WEB_TRACE.split(':').map(Number);
      if (t === at) {
        // and the instructions its busiest renderer's main thread retires meanwhile (tools/instr.py): steadier still
        const procs = readdirSync('/proc').filter(p => /^\d+$/.test(p)).map(p => { try { return [+p, readFileSync(`/proc/${p}/stat`, 'utf8')]; } catch { return null; } }).filter(Boolean);
        const stat = new Map(procs.map(([p, s]) => { const f = s.slice(s.lastIndexOf(')') + 2).split(' '); return [p, { ppid: +f[1], cpu: +f[11] + +f[12] }]; }));
        const ours = p => { for (let q = p; q > 1; q = stat.get(q)?.ppid ?? 0) if (q === chrome.pid) return true; return false; };
        const renderer = [...stat].filter(([p]) => ours(p) && readFileSync(`/proc/${p}/cmdline`, 'utf8').includes('--type=renderer')).sort((a, b) => b[1].cpu - a[1].cpu)[0]?.[0];
        instr = renderer && new Promise(r => { const c = spawn('python3', [resolve('tools/instr.py'), String(renderer), String(len)]); let o = ''; c.stdout.on('data', d => (o += d)); c.on('close', () => r(+o)); });
        await evaluate('self.__traceFrom = __frames.length');
        await send('Tracing.start', { categories: 'toplevel,__metadata', transferMode: 'ReportEvents' });
      }
      if (t === at + len) {
        const done = new Promise(r => (traced = r));
        const frames = await evaluate('__frames.length - __traceFrom');
        const ins = (await instr) || 0;
        await send('Tracing.end');
        await done;
        // the renderer main thread that worked the most (the page's)
        const main = new Set(trace.filter(e => e.ph === 'M' && e.name === 'thread_name' && e.args?.name === 'CrRendererMain').map(e => `${e.pid}:${e.tid}`));
        const cpu = {}, wall = {};
        for (const e of trace) if (e.ph === 'X' && e.name === 'ThreadControllerImpl::RunTask' && main.has(`${e.pid}:${e.tid}`)) {
          const k = `${e.pid}:${e.tid}`;
          cpu[k] = (cpu[k] ?? 0) + (e.tdur ?? 0) / 1000;
          wall[k] = (wall[k] ?? 0) + (e.dur ?? 0) / 1000;
        }
        const k = Object.keys(cpu).sort((a, b) => cpu[b] - cpu[a])[0];
        log.push(`[${n}] trace ${len} s: ${frames} frames, main thread cpu ${(cpu[k] ?? 0).toFixed(0)} ms (${((cpu[k] ?? 0) / frames).toFixed(2)} ms/frame), wall in tasks ${(wall[k] ?? 0).toFixed(0)} ms, ${(ins / frames / 1e6).toFixed(2)} M instructions/frame`);
      }
    }
    if (t === stallAt) {
      await evaluate(`{ const end = performance.now() + ${stallMs}; while (performance.now() < end); }`);
      log.push(`[${n}] ${t}s stalled ${stallMs} ms`);
    }
    // WEB_HIDE=s:ms (B260): at second s of visit 2 the window is minimised for ms (hidden: no animation frames); logs the
    // frames the page stepped while hidden (index.html steps them on a timer outside a match) and hstMatch
    const [hideAt, hideMs] = (pick ? '' : process.env.WEB_HIDE ?? '').split(':').map(Number);
    if (t === hideAt) {
      await evaluate("{ self.__n = 0; const r = requestAnimationFrame; requestAnimationFrame = f => r(x => { __n++; f(x); }); }");
      const { result: { windowId } } = await send('Browser.getWindowForTarget');
      await send('Browser.setWindowBounds', { windowId, bounds: { windowState: 'minimized' } });
      log.push(`[${n}] ${t}s hidden: ${await evaluate('document.visibilityState')}, match ${await evaluate('!!self.hstMatch')}`);
      await sleep(hideMs);
      log.push(`[${n}] frames while hidden ${hideMs} ms: ${await evaluate('__n')} (visibility ${await evaluate('document.visibilityState')})`);
      await send('Browser.setWindowBounds', { windowId, bounds: { windowState: 'normal' } });
      await sleep(1000);
      log.push(`[${n}] 1 s after showing: frames ${await evaluate('__n')}, ${await evaluate('document.visibilityState')}`);
    }
    if (t % (+process.env.SHOT_EVERY || 10) === 0) {
      const shot = await send('Page.captureScreenshot', { format: 'png' });
      writeFileSync(resolve(out, `visit${n}-${t}s.png`), Buffer.from(shot.result.data, 'base64'));
      log.push(`[${n}] ${t}s audio: ${await audio()}`);
    }
  }
  if (rotating) { clearInterval(rotating); log.push(`[${n}] turned ${turns} times`); }
  writeFileSync(resolve(out, `frames${n}.json`), await evaluate('JSON.stringify(__frames)') ?? '[]');
  if (stallMs) log.push(`[${n}] buffers started [when, context time, length]: ${await evaluate('JSON.stringify(__starts)')}`);
  log.push(`[${n}] canvas: ${await evaluate("JSON.stringify([...document.querySelectorAll('canvas')].map(c => [c.width, c.height]))")}`);
  log.push(`[${n}] opfs: ${await evaluate("navigator.storage.getDirectory().then(async d => { const r = []; for await (const [k, h] of d) r.push(k + ' ' + (await h.getFile()).size); return r.join(', '); })")}`);
  // WEB_CARDS=1: the select cards baked into OPFS `previews/` go to OUT/cards/ (B167: compare with the native bake)
  if (process.env.WEB_CARDS) {
    mkdirSync(resolve(out, 'cards'), { recursive: true });
    const dir = "navigator.storage.getDirectory().then(d => d.getDirectoryHandle('previews'))";
    const names = JSON.parse(await evaluate(`${dir}.then(async d => { const r = []; for await (const k of d.keys()) r.push(k); return JSON.stringify(r); })`) ?? '[]');
    for (const k of names) {
      const b = await evaluate(`${dir}.then(d => d.getFileHandle(${JSON.stringify(k)})).then(h => h.getFile()).then(f => f.arrayBuffer()).then(a => { const b = new Uint8Array(a); let s = ''; for (let i = 0; i < b.length; i += 8192) s += String.fromCharCode(...b.subarray(i, i + 8192)); return btoa(s); })`);
      if (b) writeFileSync(resolve(out, 'cards', k), Buffer.from(b, 'base64'));
    }
    const cards = names;
    log.push(`[${n}] cards: ${cards.length}`);
  }
  ws.close();
  chrome.kill();
  await sleep(1000);
}

if (iso !== '-') await visit(1, true);
await visit(2, false);
writeFileSync(resolve(out, 'console.txt'), log.join('\n') + '\n');
console.log(log.slice(-40).join('\n'));
