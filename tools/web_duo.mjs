// B118's browser check, on a profile that already imported the disc (tools/web_check.mjs ISO OUT): a touch phone, the
// main menu (held sideways)'s "1v1 on this phone" by the near pad, then upright: the far pad's ✕ taking 2P, both picking,
// the match started and both sticks held at once (both players must move: `4-held`), then ✕ on both for
// HST_WEB_MATCH_SECS (default 60), a shot every 30 s. Shots and console into OUT.
// Usage: node tools/web_duo.mjs OUT (serve web/ on :8765 first: python3 -m http.server -d web 8765)
import { spawn } from 'node:child_process';
import { writeFileSync } from 'node:fs';
import { resolve } from 'node:path';

const [out] = process.argv.slice(2);
const WEB = process.env.HST_WEB_PORT ?? 8765, CDP = process.env.HST_CDP_PORT ?? 9335;
const log = [];
const sleep = ms => new Promise(r => setTimeout(r, ms));
const chrome = spawn('chromium', ['--headless=new', '--mute-audio', `--remote-debugging-port=${CDP}`, `--user-data-dir=${resolve(out, 'profile')}`,
  '--ignore-gpu-blocklist', '--window-size=390,844', ...(process.env.CHROME_FLAGS ?? '').split(' ').filter(Boolean), 'about:blank'], { stdio: 'ignore' });
process.on('exit', () => chrome.kill());
for (const s of ['SIGINT', 'SIGTERM', 'SIGHUP']) process.on(s, () => process.exit(1));
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
// the menus are sideways (portrait menus are B112's), the 1v1 upright
const held = up => send('Emulation.setDeviceMetricsOverride', up
  ? { width: 390, height: 844, deviceScaleFactor: 2, mobile: true, screenOrientation: { type: 'portraitPrimary', angle: 0 } }
  : { width: 844, height: 390, deviceScaleFactor: 2, mobile: true, screenOrientation: { type: 'landscapePrimary', angle: 90 } });
await held(false);
await send('Emulation.setTouchEmulationEnabled', { enabled: true, maxTouchPoints: 5 });

const centre = sel => evaluate(`(() => { const r = document.querySelector('${sel}').getBoundingClientRect(); return { x: r.x + r.width / 2, y: r.y + r.height / 2 }; })()`);
const NEAR = '#touch', FAR = '.touch.far';
let fid = 10;
// one finger on each `sel` (moved by its [dx, dy] in screen px), all down at once for `ms`, then all lifted
const press = async (fingers, ms = 150) => {
  const p = [];
  for (const [sel, dx = 0, dy = 0] of fingers) p.push({ ...(await centre(sel)), id: fid++, dx, dy });
  await send('Input.dispatchTouchEvent', { type: 'touchStart', touchPoints: p.map(({ x, y, id }) => ({ x, y, id })) });
  if (p.some(f => f.dx || f.dy)) {
    await sleep(30);
    await send('Input.dispatchTouchEvent', { type: 'touchMove', touchPoints: p.map(({ x, y, id, dx, dy }) => ({ x: x + dx, y: y + dy, id })) });
  }
  await sleep(ms);
  const pads = await evaluate("JSON.stringify(navigator.getGamepads().filter(Boolean).map(g => [g.index, g.axes[0].toFixed(2), g.axes[1].toFixed(2), g.buttons[0].pressed]))");
  await send('Input.dispatchTouchEvent', { type: 'touchEnd', touchPoints: p.map(({ x, y, id }) => ({ x, y, id })) });
  await sleep(400);
  return pads;
};
const cross = side => `${side} [data-b="cross"]`, zone = side => `${side} .zone`;

await send('Page.navigate', { url: `http://localhost:${WEB}/?webgl2` });
await sleep(25000);
await shot('1-menu');
log.push(`pads: ${await evaluate("JSON.stringify(navigator.getGamepads().filter(Boolean).map(g => g.index))")}`);
// the menu's last row: the near stick down past the others
for (let i = 0; i < 4; i++) await press([[zone(NEAR), 0, 60]]);
await shot('2-duo-row');
await press([[cross(NEAR)]]);
await sleep(1500);
await held(true);
await sleep(1500);
log.push(`after the entry: pads ${await evaluate("JSON.stringify(navigator.getGamepads().filter(Boolean).map(g => g.index))")}, far shown ${await evaluate("!document.querySelector('.touch.far').hidden")}`);
await shot('3-select');
await press([[cross(FAR)]]);
await sleep(1000);
await shot('3-select-2p');
await press([[cross(NEAR)], [cross(FAR)]]);
await sleep(1500);
await shot('3-picked');
for (let i = 0; i < 4; i++) {
  await press([[cross(NEAR)]]);
  await sleep(1500);
  await shot(`3-confirm-${i}`);
}
await sleep(20000);
await shot('4-match');
await shot('4-before');
log.push(`both sticks held (near right, far down): ${await press([[zone(NEAR), 80, 0], [zone(FAR), 0, 80]], 1500)}`);
await shot('4-held');
const secs = +(process.env.HST_WEB_MATCH_SECS ?? 60);
for (let t = 0; t < secs; t += 2.5) {
  await press([[cross(NEAR)], [cross(FAR)]]);
  await sleep(550);
  await press([[cross(NEAR)], [cross(FAR)]]);
  await sleep(1000);
  if (t % 30 < 2.5) await shot(`5-match-${t | 0}s`);
}
await shot('6-end');
writeFileSync(resolve(out, 'console.txt'), log.join('\n') + '\n');
console.log(log.filter(l => !/^(log|debug|info): /.test(l)).slice(-40).join('\n'));
process.exit(0);
