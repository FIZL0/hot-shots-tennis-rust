// B539's check of iPhone Safari's swipe-up mode (index.html #swipe): headless chromium posing as an iPhone Safari tab
// (its user agent, `navigator.standalone` false, a touch screen), web/ served on WEB_PORT (default 8768). While the site
// shows nothing changes; once the game is up the page scrolls (taller than the screen), #swipe shows at the top, a
// scroll hides it and isn't undone by the page's size keeping (B333), a tap closes it, and a home-screen start
// (`standalone` true) or another phone gets none of it. Only the page's side: whether Safari really hides its bars on
// that scroll needs an iPhone. Prints PASS/FAIL per check; exits 1 on any FAIL. Usage: node tools/web_swipe.mjs
import { spawn } from 'node:child_process';
import { mkdtempSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

const CDP = process.env.HST_CDP_PORT ?? 9338, PORT = process.env.WEB_PORT ?? 8768;
const sleep = ms => new Promise(r => setTimeout(r, ms));
const web = spawn('python3', ['-m', 'http.server', '-b', '127.0.0.1', '-d', new URL('../web', import.meta.url).pathname, `${PORT}`], { stdio: 'ignore' });
const chrome = spawn('chromium', ['--headless=new', '--mute-audio', `--remote-debugging-port=${CDP}`,
  `--user-data-dir=${mkdtempSync(join(tmpdir(), 'hst-swipe-'))}`, '--window-size=844,390', 'about:blank'], { stdio: 'ignore' });
process.on('exit', () => { chrome.kill(); web.kill(); });
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
const evaluate = async e => (await send('Runtime.evaluate', { expression: e, returnByValue: true, awaitPromise: true })).result?.result?.value;
let failed = false;
const check = (name, ok, detail) => { failed ||= !ok; console.log(`${ok ? 'PASS' : 'FAIL'} ${name}${ok ? '' : `: ${JSON.stringify(detail)}`}`); };

const IPHONE = 'Mozilla/5.0 (iPhone; CPU iPhone OS 18_5 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.5 Mobile/15E148 Safari/604.1';
const ANDROID = 'Mozilla/5.0 (Linux; Android 14; Pixel 8) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0 Mobile Safari/537.36';
let script;
const open = async (ua, standalone) => {
  await send('Page.enable');
  await send('Emulation.setUserAgentOverride', { userAgent: ua });
  await send('Emulation.setTouchEmulationEnabled', { enabled: true, maxTouchPoints: 5 });
  await send('Emulation.setDeviceMetricsOverride', { width: 844, height: 390, deviceScaleFactor: 1, mobile: true });
  if (script) await send('Page.removeScriptToEvaluateOnNewDocument', { identifier: script });
  script = (await send('Page.addScriptToEvaluateOnNewDocument', { source: standalone === undefined ? ''
    : `Object.defineProperty(Navigator.prototype, 'standalone', { get: () => ${standalone}, configurable: true });` })).result.identifier;
  await send('Page.navigate', { url: `http://127.0.0.1:${PORT}/?site` });
  for (let i = 0; i < 50 && !(await evaluate(`document.readyState === 'complete' && document.getElementById('disc')?.hidden === false`)); i++) await sleep(200);
};
// the site closes as Play does (site.js), without starting the game's download
const game = () => evaluate(`document.getElementById('disc').hidden = true; new Promise(r => setTimeout(r, 100))`);
const state = () => evaluate(`({ swipe: document.documentElement.classList.contains('swipe'), shown: document.getElementById('swipe')?.hidden === false,
  scrollable: document.scrollingElement.scrollHeight > innerHeight + 100, y: scrollY })`);

await open(IPHONE, false);
let s = await state();
check('iPhone tab, site up: no swipe mode', !s.swipe && !s.shown, s);
await game();
s = await state();
check('iPhone tab, game up: page scrolls, #swipe shows', s.swipe && s.shown && s.scrollable, s);
await evaluate(`scrollTo(0, 120); new Promise(r => setTimeout(r, 200))`);
await evaluate(`dispatchEvent(new Event('resize')); new Promise(r => setTimeout(r, 200))`);
s = await state();
check('a scroll hides #swipe and stays scrolled', s.swipe && !s.shown && s.y > 0, s);
await evaluate(`scrollTo(0, 0); new Promise(r => setTimeout(r, 200))`);
s = await state();
check('back at the top: #swipe again', s.shown, s);
await evaluate(`document.getElementById('swipe').click(); new Promise(r => setTimeout(r, 100))`);
s = await state();
check('a tap closes it, page still scrolls', !s.shown && s.swipe, s);
const canvasTouch = await evaluate(`getComputedStyle(document.body.appendChild(document.createElement('canvas'))).touchAction`);
check('the canvas still takes no pans', canvasTouch === 'none', canvasTouch);

await open(IPHONE, true);
await game();
s = await state();
check('home-screen app: no swipe mode', !s.swipe && !s.shown, s);
await open(ANDROID);
await game();
s = await state();
check('Android: no swipe mode', !s.swipe && !s.shown, s);
process.exit(failed ? 1 : 0);
