// B464's check of web/index.html's Copy Link card alone (no game, no disc, no server): headless chromium with touch,
// the card's markup, style and script cut out of index.html onto a blank page with web/touch.js and a canvas. With no
// touch yet `hstCopySheet` leaves the copy to the game; after a touch it shows the card, and the canvas tap that asked
// for it doesn't also close it. Copy's own tap puts the text on the clipboard and closes the card; Share shows only
// where the browser has navigator.share and is handed the link; Done and a tap beside the card close it. Screenshots go
// to DIR (default context/B464). Prints PASS/FAIL per check; exits 1 on any FAIL.
// Usage: node tools/web_copy_sheet.mjs [DIR]
import { spawn } from 'node:child_process';
import { readFileSync, mkdtempSync, mkdirSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

const out = process.argv[2] ?? new URL('../context/B464', import.meta.url).pathname;
mkdirSync(out, { recursive: true });
const CDP = process.env.HST_CDP_PORT ?? 9337;
const sleep = ms => new Promise(r => setTimeout(r, ms));
const chrome = spawn('chromium', ['--headless=new', '--mute-audio', `--remote-debugging-port=${CDP}`,
  `--user-data-dir=${mkdtempSync(join(tmpdir(), 'hst-copy-'))}`, '--window-size=844,390', 'about:blank'], { stdio: 'ignore' });
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
const evaluate = async e => (await send('Runtime.evaluate', { expression: e, returnByValue: true, awaitPromise: true })).result?.result?.value;
const frame = () => evaluate('new Promise(r => requestAnimationFrame(() => requestAnimationFrame(r)))');
let failed = false;
const check = (name, ok, detail) => { failed ||= !ok; console.log(`${ok ? 'PASS' : 'FAIL'} ${name}${ok ? '' : `: ${JSON.stringify(detail)}`}`); };

// the card's pieces of index.html
const html = readFileSync(new URL('../web/index.html', import.meta.url), 'utf8');
const div = html.match(/<div id="copysheet"[^]*?<\/div><\/div>/)[0];
const js = html.match(/ {2}\/\/ B464: Copy Link[^]*?(?=\n {2}\/\/ a LAN answer)/)[0];
// a secure origin (navigator.clipboard), granted the clipboard as a tap would on a phone
await send('Page.enable');
await send('Page.navigate', { url: `http://127.0.0.1:${CDP}/json/version` });
await sleep(500);
await send('Browser.grantPermissions', { permissions: ['clipboardReadWrite', 'clipboardSanitizedWrite'], origin: `http://127.0.0.1:${CDP}` });
const style = html.match(/<style>([^]*?)<\/style>/)[1];
await evaluate(`document.head.innerHTML = '<meta name="viewport" content="width=device-width, initial-scale=1">';
  document.head.append(Object.assign(document.createElement('style'), { textContent: ${JSON.stringify(style)} }));
  document.body.innerHTML = '<canvas width="844" height="390" style="background:#264"></canvas>' + ${JSON.stringify(div)};`);
await evaluate(readFileSync(new URL('../web/touch.js', import.meta.url), 'utf8'));
await evaluate(`(() => { ${js} })()`);
await sleep(300);
const LINK = 'https://opentennis.example/?code=TEST-LOBBY-42';
check('a mouse and keyboard: the game copies itself', (await evaluate(`hstCopySheet(${JSON.stringify(LINK)})`)) === false && (await evaluate(`document.getElementById('copysheet').hidden`)));
await send('Emulation.setDeviceMetricsOverride', { width: 844, height: 390, deviceScaleFactor: 2, mobile: true });
await send('Emulation.setTouchEmulationEnabled', { enabled: true, maxTouchPoints: 5 });
const shown = () => evaluate(`!document.getElementById('copysheet').hidden`);
const centre = sel => evaluate(`(() => { const r = document.querySelector('${sel}').getBoundingClientRect(); return { x: (r.left + r.right) / 2, y: (r.top + r.bottom) / 2, w: r.width, h: r.height, l: r.left, t: r.top, r: r.right, b: r.bottom }; })()`);
const tap = async (p, between) => {
  await send('Input.dispatchTouchEvent', { type: 'touchStart', touchPoints: [{ x: p.x, y: p.y, id: 1 }] });
  if (between) await between();
  await send('Input.dispatchTouchEvent', { type: 'touchEnd', touchPoints: [] });
  await sleep(100);
};

// the lobby's Copy Link tapped on the canvas: the game asks for the card while the finger is still down (or a frame
// after it lifts), at a spot that ends up on the card's backdrop
await tap({ x: 20 + 40, y: 30 }, async () => check('after a touch the card holds it', (await evaluate(`hstCopySheet(${JSON.stringify(LINK)})`)) === true));
await sleep(300);
check("...and the tap that asked for it doesn't close it", await shown());
check('...the link is in the field', (await evaluate(`document.getElementById('copytext').value`)) === LINK);
check('...Share hidden without navigator.share', await evaluate(`!navigator.share ? document.getElementById('sharebtn').hidden : true`));
for (const [name, w, h] of [['landscape', 844, 390], ['portrait', 390, 844]]) {
  await send('Emulation.setDeviceMetricsOverride', { width: w, height: h, deviceScaleFactor: 2, mobile: true });
  await frame();
  const c = await centre('#copysheet > div'), vw = await evaluate('innerWidth'), vh = await evaluate('innerHeight');
  check(`${name}: the card is on screen and centred`, c.l >= 0 && c.r <= vw && c.t >= 0 && c.b <= vh && Math.abs(c.x - vw / 2) < 1, { c, vw, vh });
  const shot = await send('Page.captureScreenshot', { format: 'png' });
  writeFileSync(join(out, `copy-${name}.png`), Buffer.from(shot.result.data, 'base64'));
}
await evaluate(`navigator.clipboard.writeText('old')`);
await tap(await centre('#copybtn'));
check('Copy: the link is on the clipboard', (await evaluate('navigator.clipboard.readText()')) === LINK, await evaluate('navigator.clipboard.readText()'));
check('...the button says so', (await evaluate(`document.getElementById('copybtn').textContent`)) === 'Copied');
await sleep(900);
check('...and the card closes', !(await shown()));

await evaluate(`hstCopySheet('LAN-OFFER-CODE')`);
await sleep(500); // past the guard against the opening tap's own click
check('a code: says so', (await evaluate(`document.getElementById('copywhat').textContent`)).includes('code'));
await tap(await centre('#copydone'));
check('Done closes it', !(await shown()));
await evaluate(`hstCopySheet(${JSON.stringify(LINK)})`);
await sleep(500);
await tap({ x: 60, y: 40 }); // clear of the 20 px edges, where touch.js swallows touches
check('a tap beside the card closes it', !(await shown()));

await evaluate(`navigator.share = d => (window.shared = d, Promise.resolve())`);
await evaluate(`hstCopySheet(${JSON.stringify(LINK)})`);
await sleep(500);
check('Share shown with navigator.share', await evaluate(`!document.getElementById('sharebtn').hidden`));
await tap(await centre('#sharebtn'));
check('...hands it the link', (await evaluate('window.shared?.url')) === LINK, await evaluate('window.shared'));
check('...and closes', !(await shown()));
console.log(`screenshots: ${out}`);
process.exit(failed ? 1 : 0);
