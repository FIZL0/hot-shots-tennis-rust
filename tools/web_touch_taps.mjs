// B165's check of web/touch.js alone (no game, no disc, no server): headless chromium with touch, the script on a blank
// page and a stand-in game that polls `getGamepads()` only every 200 ms (a busy phone). With the stick held:
// a 20 ms tap on ✕ between two polls must still be seen down by one, the stick must stay tilted through it, and two
// fingers on ✕ must keep it down until both lift. Prints PASS/FAIL per check; exits 1 on any FAIL.
// Usage: node tools/web_touch_taps.mjs
import { spawn } from 'node:child_process';
import { readFileSync, mkdtempSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

const CDP = process.env.HST_CDP_PORT ?? 9334;
const sleep = ms => new Promise(r => setTimeout(r, ms));
const chrome = spawn('chromium', ['--headless=new', '--mute-audio', `--remote-debugging-port=${CDP}`,
  `--user-data-dir=${mkdtempSync(join(tmpdir(), 'hst-taps-'))}`, '--window-size=844,390', 'about:blank'], { stdio: 'ignore' });
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

await send('Emulation.setDeviceMetricsOverride', { width: 844, height: 390, deviceScaleFactor: 2, mobile: true });
await send('Emulation.setTouchEmulationEnabled', { enabled: true, maxTouchPoints: 5 });
// a classic script on the blank page (touch.js has no imports); no fullscreen on the first tap, which would move the
// buttons
await evaluate('document.documentElement.requestFullscreen = undefined;' + readFileSync(new URL('../web/touch.js', import.meta.url), 'utf8') + `
window.polls = [];
setInterval(() => { const g = navigator.getGamepads().find(g => g?.index === 16);
  polls.push(g ? [g.buttons[0].pressed, g.axes[0]] : null); }, 200);`);
await sleep(500);
const cross = await evaluate(`(() => { const r = document.querySelector('[data-b="cross"]').getBoundingClientRect();
  return { x: r.x + r.width / 2, y: r.y + r.height / 2 }; })()`);
const stick = { x: 120, y: 300, id: 1 };
// CDP lands, moves and lifts only the points it is given (its pointer ids are its own)
const touch = (type, ...p) => send('Input.dispatchTouchEvent', { type, touchPoints: p });
const down = (...p) => touch('touchStart', ...p), up = (...p) => touch('touchEnd', ...p);
const since = async n => (await evaluate('polls')).slice(n);
const count = () => evaluate('polls.length');
let failed = false;
const check = (name, ok, detail) => { failed ||= !ok; console.log(`${ok ? 'PASS' : 'FAIL'} ${name}${ok ? '' : `: ${JSON.stringify(detail)}`}`); };

console.log(`touch pad listed: ${await evaluate('navigator.getGamepads().some(g => g?.index === 16)')}`);
const tilted = { ...stick, x: 170 };
await down(stick);
await touch('touchMove', tilted);
await sleep(450);
// a quick tap with the stick held, just after a poll
let n = await count();
while ((await count()) === n) await sleep(5);
n = await count();
const c2 = { ...cross, id: 2 }, c3 = { ...cross, x: cross.x + 5, id: 3 };
await down(c2);
await sleep(20);
await up(c2);
await sleep(700);
let p = await since(n);
check('a 20 ms tap between two polls is seen down', p.filter(x => x?.[0]).length >= 1, p);
check('...and let go after', p.at(-1)?.[0] === false, p);
check('the stick stays tilted through it', p.every(x => x?.[1] > 0.5), p);
// two fingers on ✕: down until both lift
n = await count();
await down(c2);
await down(c3);
await sleep(300);
await up(c2);
await sleep(700);
p = await since(n);
check('a second finger keeps ✕ down when the first lifts', p.every(x => x?.[0]), p);
n = await count();
await up(c3);
await sleep(700);
p = await since(n);
check('✕ goes up when the last finger lifts', p.at(-1)?.[0] === false, p);
// a touch just past ✕'s edge
n = await count();
const r = await evaluate(`document.querySelector('[data-b="cross"]').getBoundingClientRect().width / 2`);
const edge = { x: cross.x, y: cross.y - r - 3, id: 4 };
await down(edge);
await sleep(450);
p = await since(n);
check('a touch 3 px past ✕\'s edge hits it', p.some(x => x?.[0]), p);
await up(edge);
check('the stick is still tilted', (await evaluate('polls.at(-1)'))?.[1] > 0.5);
await up(tilted);
await sleep(500);
check('the stick centres when it lifts', (await evaluate('polls.at(-1)'))?.[1] === 0);
process.exit(failed ? 1 : 0);
