// M9's browser check, on a profile that already imported the disc (tools/web_check.mjs OUT): a phone-sized landscape
// viewport with touch, the main menu with the on-screen controls, taps on ✕ (must move the menu on), the manifest's
// installability, then the same page again with the web server stopped (the service worker must start the game).
// Usage: node tools/web_touch.mjs OUT (serve web/ on :8765 first: python3 -m http.server -d web 8765 & echo $! > OUT/server.pid; this script
// kills the server whose pid is in OUT/server.pid before the offline visit).
import { spawn } from 'node:child_process';
import { writeFileSync, readFileSync } from 'node:fs';
import { resolve } from 'node:path';

const [out] = process.argv.slice(2);
const log = [];
// HST_WEB_PORT / HST_CDP_PORT: other ports (runs side by side)
const WEB = process.env.HST_WEB_PORT ?? 8765, CDP = process.env.HST_CDP_PORT ?? 9333;
const sleep = ms => new Promise(r => setTimeout(r, ms));
// every chromium we start dies with this script (a crash, timeout or kill would otherwise leave a hidden hst playing music)
const kids = new Set();
process.on('exit', () => kids.forEach(c => c.kill()));
for (const s of ['SIGINT', 'SIGTERM', 'SIGHUP']) process.on(s, () => process.exit(1));

async function open() {
  const chrome = spawn('chromium', ['--headless=new', '--mute-audio', `--remote-debugging-port=${CDP}`, `--user-data-dir=${resolve(out, 'profile')}`,
    '--ignore-gpu-blocklist', '--window-size=1280,720', 'about:blank'], { stdio: 'ignore' });
  kids.add(chrome);
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
    if (m.method === 'Runtime.consoleAPICalled') log.push(`${m.params.type}: ${m.params.args.map(a => a.value ?? a.description).join(' ').slice(0, 300)}`);
    if (m.method === 'Runtime.exceptionThrown') log.push(`exception: ${m.params.exceptionDetails.exception?.description ?? m.params.exceptionDetails.text}`);
  };
  const send = (method, params = {}) => new Promise(r => { waiting.set(++id, r); ws.send(JSON.stringify({ id, method, params })); });
  const evaluate = async e => (await send('Runtime.evaluate', { expression: e, returnByValue: true, awaitPromise: true })).result?.result?.value;
  const shot = async name => {
    const s = await send('Page.captureScreenshot', { format: 'png' });
    writeFileSync(resolve(out, `${name}.png`), Buffer.from(s.result.data, 'base64'));
  };
  await send('Runtime.enable');
  await send('Page.enable');
  // an iPhone-sized screen held sideways, touch only
  await send('Emulation.setDeviceMetricsOverride', { width: 844, height: 390, deviceScaleFactor: 2, mobile: true, screenOrientation: { type: 'landscapePrimary', angle: 90 } });
  await send('Emulation.setTouchEmulationEnabled', { enabled: true, maxTouchPoints: 5 });
  await send('Emulation.setEmitTouchEventsForMouse', { enabled: true, configuration: 'mobile' });
  return { chrome, ws, send, evaluate, shot };
}

async function tap(b, sel) {
  const r = await b.evaluate(`(() => { const r = document.querySelector('${sel}').getBoundingClientRect(); return [r.x + r.width / 2, r.y + r.height / 2]; })()`);
  const p = [{ x: r[0], y: r[1], id: 1 }];
  await b.send('Input.dispatchTouchEvent', { type: 'touchStart', touchPoints: p });
  await sleep(150);
  await b.send('Input.dispatchTouchEvent', { type: 'touchEnd', touchPoints: [] });
}

// online: menu, controls, taps, manifest
let b = await open();
await b.send('Page.navigate', { url: `http://localhost:${WEB}/` });
await sleep(25000);
await b.shot('1-menu');
log.push(`pads: ${await b.evaluate("JSON.stringify([...navigator.getGamepads()].filter(Boolean).map(g => [g.id, g.index]))")}`);
log.push(`controls shown: ${await b.evaluate("!document.getElementById('touch').hidden")}`);
log.push(`settings: ${JSON.stringify(await b.evaluate("localStorage.getItem('./settings.txt')"))}`);
log.push(`canvas: ${await b.evaluate("JSON.stringify([...document.querySelectorAll('canvas')].map(c => [c.width, c.height]))")}`);
for (let i = 0; i < 3; i++) {
  await tap(b, '[data-b="cross"]');
  await sleep(3000);
  await b.shot(`2-cross${i + 1}`);
}
// the stick: drag down from the left half, then let go
const drag = [{ x: 150, y: 250, id: 2 }];
await b.send('Input.dispatchTouchEvent', { type: 'touchStart', touchPoints: drag });
await b.send('Input.dispatchTouchEvent', { type: 'touchMove', touchPoints: [{ x: 150, y: 330, id: 2 }] });
await sleep(100);
log.push(`stick held: ${await b.evaluate("JSON.stringify(navigator.getGamepads().find(g => g?.index === 16)?.axes)")}`);
await b.shot('3-stick');
await sleep(500);
await b.send('Input.dispatchTouchEvent', { type: 'touchEnd', touchPoints: [] });
log.push(`stick released: ${await b.evaluate("JSON.stringify(navigator.getGamepads().find(g => g?.index === 16)?.axes)")}`);
await sleep(1500);
await b.shot('4-after-stick');
// on through the menus into a match (✕ every 3 s), then a few swings
for (let i = 0; i < 8; i++) {
  await tap(b, '[data-b="cross"]');
  await sleep(3000);
}
await sleep(20000);
await b.shot('5-match');
for (let i = 0; i < 4; i++) {
  await tap(b, '[data-b="cross"]');
  await sleep(2500);
}
await b.shot('6-match-later');
const m = await b.send('Page.getAppManifest');
log.push(`manifest errors: ${JSON.stringify(m.result?.errors)}`);
log.push(`installability errors: ${JSON.stringify((await b.send('Page.getInstallabilityErrors')).result)}`);
log.push(`service worker: ${await b.evaluate("navigator.serviceWorker.getRegistration().then(r => r?.active?.state ?? 'none')")}`);
log.push(`cached: ${await b.evaluate("caches.open('hst').then(c => c.keys()).then(k => k.map(r => new URL(r.url).pathname).join(' '))")}`);
b.ws.close();
b.chrome.kill();
await sleep(1500);

// offline: the server's gone, the service worker serves the page and the game
try { process.kill(+readFileSync(resolve(out, 'server.pid'), 'utf8')); } catch (e) { log.push(`no server to stop: ${e.message}`); }
await sleep(500);
log.push(`server down: ${await fetch(`http://localhost:${WEB}/`).then(() => false, () => true)}`);
b = await open();
await b.send('Page.navigate', { url: `http://localhost:${WEB}/` });
await sleep(30000);
await b.shot('7-offline');
log.push(`offline canvas: ${await b.evaluate("JSON.stringify([...document.querySelectorAll('canvas')].map(c => [c.width, c.height]))")}`);
b.ws.close();
b.chrome.kill();
writeFileSync(resolve(out, 'console.txt'), log.join('\n') + '\n');
console.log(log.filter(l => !/^(log|debug|info): /.test(l)).slice(-60).join('\n'));
