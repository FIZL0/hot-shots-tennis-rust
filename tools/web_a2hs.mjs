// M123's check of the site's Add to Home Screen steps (index.html #a2hs, site.js homeScreen): headless chromium, web/
// served on WEB_PORT (default 8768, started here), the page loaded under each browser's user agent. The steps must show
// on iPhone and iPad Safari only: not in Chrome/Firefox for iOS, an in-app view, a desktop or Android browser, the
// home-screen app (navigator.standalone), or after Got it. Screenshots of the iPhone/iPad ones go to DIR (default
// context/M123). Prints PASS/FAIL per check; exits 1 on any FAIL.
// Usage: node tools/web_a2hs.mjs [DIR]
import { spawn } from 'node:child_process';
import { mkdtempSync, mkdirSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

const out = process.argv[2] ?? new URL('../context/M123', import.meta.url).pathname;
mkdirSync(out, { recursive: true });
const CDP = process.env.HST_CDP_PORT ?? 9338, PORT = process.env.WEB_PORT ?? 8768;
const sleep = ms => new Promise(r => setTimeout(r, ms));
const web = spawn('python3', ['-m', 'http.server', '-b', '127.0.0.1', '-d', new URL('../web', import.meta.url).pathname, `${PORT}`], { stdio: 'ignore' });
const chrome = spawn('chromium', ['--headless=new', '--mute-audio', `--remote-debugging-port=${CDP}`,
  `--user-data-dir=${mkdtempSync(join(tmpdir(), 'hst-a2hs-'))}`, '--window-size=1024,800', 'about:blank'], { stdio: 'ignore' });
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
const IPAD = 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.5 Safari/605.1.15';
const CASES = [
  // name, user agent, touch points, standalone, shows
  ['iPhone Safari', IPHONE, 5, false, true],
  ['iPad Safari (desktop UA)', IPAD, 5, false, true],
  ['iPhone home-screen app', IPHONE, 5, true, false],
  ['Chrome on iPhone', 'Mozilla/5.0 (iPhone; CPU iPhone OS 18_5 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) CriOS/138.0 Mobile/15E148 Safari/604.1', 5, false, false],
  ['Firefox on iPhone', 'Mozilla/5.0 (iPhone; CPU iPhone OS 18_5 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) FxiOS/140.0 Mobile/15E148 Safari/605.1.15', 5, false, false],
  ['in-app view on iPhone', 'Mozilla/5.0 (iPhone; CPU iPhone OS 18_5 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Mobile/15E148 Instagram 380.0', 5, false, false],
  ['Mac Safari', IPAD, 0, false, false],
  ['Android Chrome', 'Mozilla/5.0 (Linux; Android 14; Pixel 8) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/138.0 Mobile Safari/537.36', 5, false, false],
];
const base = `http://127.0.0.1:${PORT}/?site`;
const shown = `(() => { const b = document.getElementById('a2hs'); return !!b && !b.hidden && b.getBoundingClientRect().height > 0; })()`;
const load = async () => {
  await send('Page.navigate', { url: base });
  for (let i = 0; i < 50; i++) {
    await sleep(200);
    if (await evaluate(`!!document.querySelector('#disc') && !document.getElementById('disc').hidden && !!document.querySelector('#prefs *')`)) break;
  }
  await sleep(300);
};
await send('Page.enable');
let script;
for (const [name, ua, touch, standalone, want] of CASES) {
  const phone = ua.includes('iPhone') || ua.includes('Android');
  await send('Emulation.setUserAgentOverride', { userAgent: ua });
  await send('Emulation.setTouchEmulationEnabled', { enabled: touch > 0, maxTouchPoints: touch || 1 });
  await send('Emulation.setDeviceMetricsOverride', { width: phone ? 390 : 1024, height: phone ? 844 : 1366, deviceScaleFactor: 1, mobile: phone });
  if (script) await send('Page.removeScriptToEvaluateOnNewDocument', { identifier: script });
  script = (await send('Page.addScriptToEvaluateOnNewDocument', { source: `localStorage.removeItem('hst-a2hs');
    Object.defineProperty(Navigator.prototype, 'standalone', { get: () => ${standalone}, configurable: true });` })).result.identifier;
  await load();
  const got = await evaluate(shown);
  check(`${name}: steps ${want ? 'shown' : 'hidden'}`, got === want, { got });
  if (want) {
    await evaluate(`document.getElementById('a2hs').scrollIntoView()`);
    const vp = await evaluate(`(() => { const r = document.getElementById('a2hs').getBoundingClientRect(); return { x: 0, y: Math.max(0, r.top - 12), width: innerWidth, height: r.height + 24, scale: 1 }; })()`);
    const wide = await evaluate(`[...document.querySelectorAll('#a2hs *')].filter(e => e.getBoundingClientRect().right > innerWidth + 0.5).length`);
    check(`${name}: nothing past the screen's edge`, wide === 0, { wide });
    const shot = await send('Page.captureScreenshot', { format: 'png', clip: vp });
    writeFileSync(join(out, `${name.split(' ')[0].toLowerCase()}.png`), Buffer.from(shot.result.data, 'base64'));
  }
}
// Got it hides the steps for good (the script's removeItem runs on each load, so drop it first)
await send('Page.removeScriptToEvaluateOnNewDocument', { identifier: script });
await send('Emulation.setUserAgentOverride', { userAgent: IPHONE });
await load();
await evaluate(`document.getElementById('a2hs-ok').click()`);
check('Got it hides the steps', !(await evaluate(shown)));
await load();
check('dismissed steps stay hidden after a reload', !(await evaluate(shown)));
process.exit(failed ? 1 : 0);
