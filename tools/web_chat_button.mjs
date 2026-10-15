// B312's check of web/touch.js's chat button alone (no game, no disc, no server): headless chromium with touch, the
// script on a blank page. The button is hidden until `hstTouchChat(true)`; then it sits on Select/Start's row right of
// Start (landscape) or centred under them (upright), as big as they are, overlapping no other control; a tap or a
// click on it is counted once by `hstChatTaps` and presses no pad button, a tap raises the keyboard (`hstChatKeys`); hidden again it counts nothing. Screenshots
// of both layouts go to DIR (default context/B312). Prints PASS/FAIL per check; exits 1 on any FAIL.
// Usage: node tools/web_chat_button.mjs [DIR]
import { spawn } from 'node:child_process';
import { readFileSync, mkdtempSync, mkdirSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

const out = process.argv[2] ?? new URL('../context/B312', import.meta.url).pathname;
mkdirSync(out, { recursive: true });
const CDP = process.env.HST_CDP_PORT ?? 9336;
const sleep = ms => new Promise(r => setTimeout(r, ms));
const chrome = spawn('chromium', ['--headless=new', '--mute-audio', `--remote-debugging-port=${CDP}`,
  `--user-data-dir=${mkdtempSync(join(tmpdir(), 'hst-chat-'))}`, '--window-size=844,390', 'about:blank'], { stdio: 'ignore' });
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
const frame = () => send('Runtime.evaluate', { expression: 'new Promise(r => requestAnimationFrame(() => requestAnimationFrame(r)))', awaitPromise: true });
let failed = false;
const check = (name, ok, detail) => { failed ||= !ok; console.log(`${ok ? 'PASS' : 'FAIL'} ${name}${ok ? '' : `: ${JSON.stringify(detail)}`}`); };

await send('Emulation.setDeviceMetricsOverride', { width: 844, height: 390, deviceScaleFactor: 2, mobile: true });
await send('Emulation.setTouchEmulationEnabled', { enabled: true, maxTouchPoints: 5 });
await evaluate('document.documentElement.requestFullscreen = undefined; document.body.style.background = "#264";'
  + readFileSync(new URL('../web/touch.js', import.meta.url), 'utf8'));
await sleep(300);
const chat = '#touch [data-chat]';
check('hidden until the game shows it', await evaluate(`document.querySelector('${chat}').offsetParent === null`));
await evaluate('hstTouchChat(true)');
await frame();
const rects = () => evaluate(`Object.fromEntries([...document.querySelectorAll('#touch button')].map(b => {
  const r = b.getBoundingClientRect(); return [b.dataset.b ?? 'chat', [r.left, r.top, r.right, r.bottom]]; }))`);
const overlap = (a, b) => a[0] < b[2] && b[0] < a[2] && a[1] < b[3] && b[1] < a[3];
const layout = async (name, w, h, where) => {
  await send('Emulation.setDeviceMetricsOverride', { width: w, height: h, deviceScaleFactor: 2, mobile: true });
  await frame();
  const r = await rects();
  w = await evaluate('innerWidth'); // a mobile page's layout viewport, not the device's
  const c = r.chat, s = r.start;
  check(`${name}: shown, as big as Start`, c[2] - c[0] > 0 && Math.abs(c[2] - c[0] - (s[2] - s[0])) < 0.5 && Math.abs(c[3] - c[1] - (s[3] - s[1])) < 0.5, r);
  check(`${name}: ${where}`, name === 'landscape' ? c[1] === s[1] && c[0] > s[2] : c[1] > s[3] && Math.abs((c[0] + c[2]) / 2 - w / 2) < 0.5, r);
  const hits = Object.entries(r).filter(([k, b]) => k !== 'chat' && overlap(b, c)).map(([k]) => k);
  check(`${name}: overlaps no other control`, !hits.length, hits);
  check(`${name}: on screen`, c[0] >= 0 && c[2] <= w && c[1] >= 0, c);
  check(`${name}: same style as Start (icon only)`, await evaluate(`(() => { const a = getComputedStyle(document.querySelector('${chat}')),
    b = getComputedStyle(document.querySelector('#touch [data-b="start"]'));
    return ['opacity', 'borderRadius', 'borderTopColor', 'backgroundColor'].every(k => a[k] === b[k])
      && document.querySelector('${chat}').textContent.trim() === '' && !!document.querySelector('${chat} svg'); })()`));
  const shot = await send('Page.captureScreenshot', { format: 'png' });
  writeFileSync(join(out, `chat-${name}.png`), Buffer.from(shot.result.data, 'base64'));
  return c;
};
await layout('portrait', 390, 844, 'centred under Select/Start');
const c = await layout('landscape', 844, 390, 'right of Start on its row');
const at = { x: (c[0] + c[2]) / 2, y: (c[1] + c[3]) / 2, id: 1 };
const pressed = () => evaluate('navigator.getGamepads().find(g => g?.index === 16).buttons.some(b => b.pressed)');
// index.html's hook, stubbed
await evaluate('window.keysUp = 0; window.hstChatKeys = () => keysUp++');
await evaluate('hstChatTaps()');
await send('Input.dispatchTouchEvent', { type: 'touchStart', touchPoints: [at] });
const down = await pressed();
await send('Input.dispatchTouchEvent', { type: 'touchEnd', touchPoints: [at] });
await frame();
check('a tap is counted once', (await evaluate('hstChatTaps()')) === 1);
check('...and raises the phone keyboard in its own event', (await evaluate('keysUp')) === 1);
check('...and presses no pad button', !down && !(await pressed()));
check('...and is taken once', (await evaluate('hstChatTaps()')) === 0);
await send('Input.dispatchMouseEvent', { type: 'mousePressed', x: at.x, y: at.y, button: 'left', clickCount: 1 });
await send('Input.dispatchMouseEvent', { type: 'mouseReleased', x: at.x, y: at.y, button: 'left', clickCount: 1 });
check('a click is counted too', (await evaluate('hstChatTaps()')) === 1);
await evaluate('hstTouchChat(false)');
await frame();
check('hidden again by the game', await evaluate(`document.querySelector('${chat}').offsetParent === null`));
await send('Input.dispatchTouchEvent', { type: 'touchStart', touchPoints: [at] });
await send('Input.dispatchTouchEvent', { type: 'touchEnd', touchPoints: [at] });
check('...and counts nothing then', (await evaluate('hstChatTaps()')) === 0);
console.log(`screenshots: ${out}`);
process.exit(failed ? 1 : 0);
