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
window.poller = setInterval(() => { const g = navigator.getGamepads().find(g => g?.index === 16);
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

// B274: real multi-touch (each event carries every finger still down); the game polls only when the check asks (`now`)
await evaluate(`clearInterval(poller); window.now = () => { const g = navigator.getGamepads().find(g => g?.index === 16);
  return [g.axes[0], g.axes[1], ...g.buttons.map(b => b.pressed)]; }`);
const at = await evaluate(`Object.fromEntries([...document.querySelectorAll('#touch button')].map(b => {
  const r = b.getBoundingClientRect(); return [b.dataset.b, { x: r.x + r.width / 2, y: r.y + r.height / 2 }]; }))`);
const idx = { cross: 0, circle: 1, square: 2, triangle: 3, l1: 4, r1: 5, select: 8, start: 9 };
const names = Object.keys(idx);
const fingers = new Map();
let nextId = 10;
const land = async p => { const f = { ...p, id: nextId++ }; fingers.set(f.id, f); await touch('touchStart', ...fingers.values()); return f.id; };
const slide = async (fid, p) => { Object.assign(fingers.get(fid), p); await touch('touchMove', ...fingers.values()); };
const lift = async fid => { const f = fingers.get(fid); fingers.delete(fid); await touch('touchEnd', f); };
const frame = () => send('Runtime.evaluate', { expression: 'new Promise(r => requestAnimationFrame(() => requestAnimationFrame(r)))', awaitPromise: true });
const now = () => evaluate('now()');
let bad = [];
for (let round = 0; round < 3; round++) {
  // stick + each button: held together, the stick moving while the button is held, then the button tapped
  for (const k of names) {
    const s = await land(stick);
    await slide(s, { x: stick.x + 60 });
    const b = await land(at[k]);
    await frame();
    let v = await now();
    if (!(v[0] > 0.5 && v[2 + idx[k]])) bad.push(['stick+held', k, v]);
    await slide(s, { x: stick.x - 60, y: stick.y - 30 });
    await frame();
    v = await now();
    if (!(v[0] < -0.5 && v[1] < 0 && v[2 + idx[k]])) bad.push(['stick moves with held', k, v]);
    await lift(b);
    await frame(); await frame();
    v = await now();
    if (!(v[0] < -0.5 && !v[2 + idx[k]])) bad.push(['held lifts, stick stays', k, v]);
    const t = await land(at[k]);
    await lift(t);
    await frame();
    v = await now();
    if (!(v[0] < -0.5 && v[2 + idx[k]])) bad.push(['tap with stick', k, v]);
    await lift(s);
    await frame(); await frame();
    v = await now();
    if (!(v[0] === 0 && !v[2 + idx[k]])) bad.push(['all up', k, v]);
  }
  // every pair of buttons: both held, either lifts first; one held while the other is tapped
  for (const a of names) for (const b of names) {
    if (a === b) continue;
    const fa = await land(at[a]), fb = await land(at[b]);
    await frame();
    let v = await now();
    if (!(v[2 + idx[a]] && v[2 + idx[b]])) bad.push(['pair held', a, b, v]);
    await lift(fa);
    await frame(); await frame();
    v = await now();
    if (!(!v[2 + idx[a]] && v[2 + idx[b]])) bad.push(['first lifts, second held', a, b, v]);
    const ta = await land(at[a]);
    await lift(ta);
    await frame();
    v = await now();
    if (!(v[2 + idx[a]] && v[2 + idx[b]])) bad.push(['tap beside held', a, b, v]);
    await lift(fb);
    await frame(); await frame();
    v = await now();
    if (v.slice(2).some(x => x)) bad.push(['pair up', a, b, v]);
  }
}
check('stick + each button and every button pair, held and tapped, 3 rounds', !bad.length, bad.slice(0, 5));
// a cancel lets everything go, and the controls take fingers again after it
let s = await land(stick); await slide(s, { x: stick.x + 60 }); await land(at.cross);
await touch('touchCancel');
fingers.clear();
// (a press no poll has seen yet still counts once: B165)
await now(); await frame();
let v = await now();
check('a cancel lets the stick and ✕ go', v[0] === 0 && !v[2], v);
s = await land(stick); await slide(s, { x: stick.x + 60 }); let b = await land(at.cross);
await frame();
v = await now();
check('...and both work again after it', v[0] > 0.5 && v[2], v);
// a finger whose end never comes (the next touch event no longer lists it) lets its control go
await evaluate(`(() => { const t = new Touch({ identifier: 999, target: document.body, clientX: 400, clientY: 200 });
  document.body.dispatchEvent(new TouchEvent('touchend', { bubbles: true, touches: [], changedTouches: [t] })); })()`);
await frame(); await frame();
v = await now();
check('fingers missing from `touches` let go of their controls', v[0] === 0 && !v[2], v);
await touch('touchCancel');
fingers.clear();
s = await land(stick); await slide(s, { x: stick.x + 60 });
await frame();
check('...and the stick takes a new thumb after it', (await now())[0] > 0.5);
await touch('touchCancel');
fingers.clear();
// B118: 1v1 on this phone, held upright: a second pad (17) at the far end, turned round; its stick reads in the
// screen's directions, and both players' controls work at once
await send('Emulation.setDeviceMetricsOverride', { width: 390, height: 844, deviceScaleFactor: 2, mobile: false });
await evaluate('hstTouchPads(true, false)');
await frame(); await frame();
const pads = () => evaluate(`navigator.getGamepads().filter(g => g).map(g => [g.index, g.axes[0], g.axes[1], g.buttons[0].pressed])`);
check('both pads listed in 1v1', JSON.stringify((await pads()).map(g => g[0])) === '[16,17]', await pads());
const centre = sel => evaluate(`(() => { const r = document.querySelector('${sel}').getBoundingClientRect();
  return { x: r.x + r.width / 2, y: r.y + r.height / 2 }; })()`);
const nearX = await centre('#touch [data-b="cross"]'), farX = await centre('.touch.far [data-b="cross"]');
const farZone = await centre('.touch.far .zone');
check('the far pad sits at the top', farX.y < 422 && nearX.y > 422, [nearX, farX]);
const fs = await land(farZone); await slide(fs, { x: farZone.x + 60, y: farZone.y + 40 });
const ns = await land(stick.x < 390 ? { x: 80, y: 700 } : stick); await slide(ns, { x: 20, y: 700 });
await land(nearX); await land(farX);
await frame();
let g = await pads();
check('far stick reads screen right/down, near stick left, both ✕ down at once',
  g[1][1] > 0.5 && g[1][2] > 0.3 && g[0][1] < -0.5 && g[0][3] && g[1][3], g);
await touch('touchCancel');
fingers.clear();
await evaluate('hstTouchPads(false, false)');
await frame(); await frame();
check('back to one pad after', (await pads()).length === 1, await pads());
await touch('touchCancel');
process.exit(failed ? 1 : 0);
