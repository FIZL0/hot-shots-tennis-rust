// M9's browser check, on a profile that already imported the disc (tools/web_check.mjs OUT): a phone-sized landscape
// viewport with touch, the menus tapped through to a match (B244: the controls up there too, the stick in its corner), the
// controls and the stick in the match (held right: 1P must move, `8-p1-*`), taps on ✕ there, the manifest's installability, then the same page again with the web server stopped (the service worker must start the game).
// Usage: node tools/web_touch.mjs OUT (serve web/ on :8765 first: python3 -m http.server -d web 8765 & echo $! > OUT/server.pid; this script
// kills the server whose pid is in OUT/server.pid before the offline visit).
import { spawn } from 'node:child_process';
import { writeFileSync, readFileSync } from 'node:fs';
import { resolve } from 'node:path';

const [out] = process.argv.slice(2);
const log = [];
// HST_WEB_PORT / HST_CDP_PORT: other ports (runs side by side)
const WEB = process.env.HST_WEB_PORT ?? 8765, CDP = process.env.HST_CDP_PORT ?? 9333;
// HST_WEB_QUERY: the page's query (`?webgl2` keeps to the WebGL2 build)
const Q = process.env.HST_WEB_QUERY ?? '';
const sleep = ms => new Promise(r => setTimeout(r, ms));
// every chromium we start dies with this script (a crash, timeout or kill would otherwise leave a hidden hst playing music)
const kids = new Set();
process.on('exit', () => kids.forEach(c => c.kill()));
for (const s of ['SIGINT', 'SIGTERM', 'SIGHUP']) process.on(s, () => process.exit(1));

async function open() {
  const chrome = spawn('chromium', ['--headless=new', '--mute-audio', `--remote-debugging-port=${CDP}`, `--user-data-dir=${resolve(out, 'profile')}`,
    '--ignore-gpu-blocklist', '--window-size=1280,720', ...(process.env.CHROME_FLAGS ?? '').split(' ').filter(Boolean), 'about:blank'], { stdio: 'ignore' });
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
await b.send('Page.navigate', { url: `http://localhost:${WEB}/${Q}` });
await sleep(25000);
await b.shot('1-menu');
log.push(`pads: ${await b.evaluate("JSON.stringify([...navigator.getGamepads()].filter(Boolean).map(g => [g.id, g.index]))")}`);
log.push(`controls shown in the menu (want true): ${await b.evaluate("!document.getElementById('touch').hidden")}`);
log.push(`settings: ${JSON.stringify(await b.evaluate("localStorage.getItem('./settings.txt')"))}`);
log.push(`canvas: ${await b.evaluate("JSON.stringify([...document.querySelectorAll('canvas')].map(c => [c.width, c.height]))")}`);
// B221: menus by touch alone, at the 640×448 point of the centred 4:3 screen (a landscape window: full height)
const at = async (name, x, y) => {
  const [w, h] = await b.evaluate('[innerWidth, innerHeight]');
  const p = [{ x: w / 2 + (x - 320) / 640 * h * 4 / 3, y: y / 448 * h, id: 3 }];
  await b.send('Input.dispatchTouchEvent', { type: 'touchStart', touchPoints: p });
  await sleep(120);
  await b.send('Input.dispatchTouchEvent', { type: 'touchEnd', touchPoints: [] });
  await sleep(2500);
  await b.shot(name);
};
await at('2-local', 320, 140);
await at('2-singles', 320, 170);
await at('2-assign', 320, 118);
await at('2-char', 435, 128);
await at('2-com', 232, 242);
await at('2-start', 190, 305);
await sleep(20000);
await b.shot('5-match');
log.push(`controls shown in the match: ${await b.evaluate("!document.getElementById('touch').hidden")}`);
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
// B244: the pad drives 1P even though the menus were tapped (the keyboard's seat): hold the stick right
await b.shot('8-p1-before');
const hold = [{ x: 150, y: 250, id: 4 }];
await b.send('Input.dispatchTouchEvent', { type: 'touchStart', touchPoints: hold });
await b.send('Input.dispatchTouchEvent', { type: 'touchMove', touchPoints: [{ x: 250, y: 250, id: 4 }] });
await sleep(1500);
await b.shot('8-p1-held');
await b.send('Input.dispatchTouchEvent', { type: 'touchEnd', touchPoints: [] });
for (let i = 0; i < 4; i++) {
  await tap(b, '[data-b="cross"]');
  await sleep(2500);
}
await b.shot('6-match-later');
const play = {};
// HST_WEB_MATCH_SECS: play on that long (✕ twice: a serve's toss and hit, or a swing), a shot a minute
for (let t = 0; t < +(process.env.HST_WEB_MATCH_SECS ?? 0); t += 2.5) {
  await tap(b, '[data-b="cross"]');
  await sleep(550);
  await tap(b, '[data-b="cross"]');
  await sleep(1800);
  if (t % 60 < 2.5) await b.shot(`6-match-${t | 0}s`);
  const st = await b.evaluate("JSON.stringify([document.getElementById('touch').hidden, document.getElementById('boot').className, document.getElementById('boot').hidden, document.getElementById('disc').hidden, [...navigator.getGamepads()].filter(Boolean).map(g => g.id)])");
  if (st !== play.last) log.push(`${t | 0}s pad state [hidden, boot class, boot hidden, disc hidden, pads]: ${(play.last = st)}`);
}
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
