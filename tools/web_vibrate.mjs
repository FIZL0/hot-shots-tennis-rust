// B329's check of web/touch.js's press buzz alone (no game, no disc, no server): headless chromium with touch, the
// script on a blank page, `navigator.vibrate` recorded. A tap on ✕ buzzes (15 ms) with Vibration on and not with it
// off; then as an iPhone (no `navigator.vibrate`) a tap flips the hidden switch checkbox (iOS 18's haptic tick) and
// the page says `ios` to the setting's text. Prints PASS/FAIL per check; exits 1 on any FAIL.
// Usage: node tools/web_vibrate.mjs
import { spawn } from 'node:child_process';
import { readFileSync, mkdtempSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

const CDP = process.env.HST_CDP_PORT ?? 9335;
const sleep = ms => new Promise(r => setTimeout(r, ms));
const chrome = spawn('chromium', ['--headless=new', '--mute-audio', `--remote-debugging-port=${CDP}`,
  `--user-data-dir=${mkdtempSync(join(tmpdir(), 'hst-vib-'))}`, '--window-size=844,390', 'about:blank'], { stdio: 'ignore' });
process.on('exit', () => chrome.kill());
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
const evaluate = async e => (await send('Runtime.evaluate', { expression: e, returnByValue: true })).result?.result?.value;
let failed = false;
const check = (name, ok, detail) => { failed ||= !ok; console.log(`${ok ? 'PASS' : 'FAIL'} ${name}${ok ? '' : `: ${JSON.stringify(detail)}`}`); };

await send('Emulation.setDeviceMetricsOverride', { width: 844, height: 390, deviceScaleFactor: 2, mobile: true });
await send('Emulation.setTouchEmulationEnabled', { enabled: true, maxTouchPoints: 5 });
const touchJs = readFileSync(new URL('../web/touch.js', import.meta.url), 'utf8');
const tapCross = async fid => {
  const c = await evaluate(`(() => { const r = document.querySelector('[data-b="cross"]').getBoundingClientRect();
    return { x: r.x + r.width / 2, y: r.y + r.height / 2 }; })()`);
  await send('Input.dispatchTouchEvent', { type: 'touchStart', touchPoints: [{ ...c, id: fid }] });
  await sleep(50);
  await send('Input.dispatchTouchEvent', { type: 'touchEnd', touchPoints: [] });
  // the press stays down until a frame has seen it
  await evaluate('navigator.getGamepads()');
  await sleep(100);
};

// Android: navigator.vibrate, recorded
await evaluate(`document.documentElement.requestFullscreen = undefined; window.buzzes = [];
  navigator.vibrate = ms => (buzzes.push(ms), true);` + touchJs);
await sleep(300);
check('the page says it vibrates', (await evaluate('hstVibration()')) === 'vibrate', await evaluate('hstVibration()'));
await tapCross(1);
check('a tap on ✕ buzzes 15 ms', JSON.stringify(await evaluate('buzzes')) === '[15]', await evaluate('buzzes'));
await evaluate('hstTouchBuzz(false)');
await tapCross(2);
check('...and not with Vibration off', (await evaluate('buzzes.length')) === 1, await evaluate('buzzes'));

// iPhone: no navigator.vibrate; the switch checkbox's haptic tick instead
await send('Emulation.setUserAgentOverride', { userAgent: 'Mozilla/5.0 (iPhone; CPU iPhone OS 18_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.0 Mobile/15E148 Safari/604.1' });
await send('Page.navigate', { url: 'about:blank' });
await sleep(500);
await evaluate(`document.documentElement.requestFullscreen = undefined; delete Navigator.prototype.vibrate;
  window.clicks = 0; addEventListener('click', () => clicks++);` + touchJs);
await sleep(300);
check('an iPhone says ios', (await evaluate('hstVibration()')) === 'ios', await evaluate('hstVibration()'));
const flipped = () => evaluate(`document.querySelector('input[switch]')?.checked`);
const before = await flipped();
await tapCross(3);
check('a tap on ✕ flips the hidden switch', before === false && (await flipped()) === true, [before, await flipped()]);
check('...without its click reaching the page', (await evaluate('clicks')) === 0, await evaluate('clicks'));
await evaluate('hstTouchBuzz(false)');
await tapCross(4);
check('...and not with Vibration off', (await flipped()) === true);
process.exit(failed ? 1 : 0);
