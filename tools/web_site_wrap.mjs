// B458's check of the site page (index.html ?site, site.js/site.css) at narrow widths: headless chromium, web/ served
// on WEB_PORT (default 8767, started here), a mod with a very long name and source put in OPFS first. At each width
// every element of #disc must stay inside its panel and the page (nothing spilling out of a card or past the screen's
// edge, no sideways scroll) and no text may be cut off (an element narrower than its own content with overflow
// hidden). The storage legend must take more than one line at 320 px. Screenshots of Settings & files go to DIR
// (default context/B458). Prints PASS/FAIL per check; exits 1 on any FAIL.
// Usage: node tools/web_site_wrap.mjs [DIR]
import { spawn } from 'node:child_process';
import { mkdtempSync, mkdirSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

const out = process.argv[2] ?? new URL('../context/B458', import.meta.url).pathname;
mkdirSync(out, { recursive: true });
const CDP = process.env.HST_CDP_PORT ?? 9337, PORT = process.env.WEB_PORT ?? 8767;
const sleep = ms => new Promise(r => setTimeout(r, ms));
const web = spawn('python3', ['-m', 'http.server', '-b', '127.0.0.1', '-d', new URL('../web', import.meta.url).pathname, `${PORT}`], { stdio: 'ignore' });
const chrome = spawn('chromium', ['--headless=new', '--mute-audio', `--remote-debugging-port=${CDP}`,
  `--user-data-dir=${mkdtempSync(join(tmpdir(), 'hst-site-'))}`, '--window-size=1024,800', 'about:blank'], { stdio: 'ignore' });
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
const load = async url => {
  await send('Page.enable');
  await send('Page.navigate', { url });
  for (let i = 0; i < 50; i++) {
    await sleep(200);
    if (await evaluate(`!!document.querySelector('#storage .bar') && !!document.querySelector('#modlist .mod')`)) return true;
  }
  return false;
};

const base = `http://127.0.0.1:${PORT}/`;
await load(`${base}?site`).catch(() => {});
await evaluate(`(async () => {
  const root = await navigator.storage.getDirectory();
  const mods = await root.getDirectoryHandle('mods', { create: true });
  const d = await mods.getDirectoryHandle('averyveryverylongmodidentifierwithoutanyspacesatall', { create: true });
  const w = await (await d.getFileHandle('mod.json', { create: true })).createWritable();
  await w.write(JSON.stringify({ name: 'A Very Long Character Mod Name That Goes On And On Without Stopping', kind: 'character',
    source: 'https://huggingface.co/datasets/someone/averyveryverylongrepositorynamewithoutanybreaks' }));
  await w.close();
})()`);
check('page loads with storage and a mod', await load(`${base}?site&again#settings`));

for (const w of [280, 320, 360, 414, 600, 1024]) {
  await send('Emulation.setDeviceMetricsOverride', { width: w, height: 900, deviceScaleFactor: 1, mobile: w < 600 });
  await evaluate('new Promise(r => requestAnimationFrame(() => requestAnimationFrame(r)))');
  const bad = await evaluate(`(() => {
    const disc = document.getElementById('disc'), vw = disc.clientWidth, bad = [];
    const name = e => e.tagName.toLowerCase() + (e.id ? '#' + e.id : '') + (e.className && typeof e.className === 'string' ? '.' + e.className.trim().replace(/\\s+/g, '.') : '')
      + ' "' + (e.textContent || '').trim().slice(0, 30) + '"';
    if (disc.scrollWidth > vw + 0.5) bad.push('page scrolls sideways: ' + disc.scrollWidth + ' > ' + vw);
    for (const e of disc.querySelectorAll('*')) {
      if (e.closest('[hidden]') || e.id === 'iso' || !e.getClientRects().length) continue; // #iso: the file input, 1 px and invisible
      const r = e.getBoundingClientRect();
      if (r.width === 0) continue;
      const p = e.parentElement.closest('.panel');
      if (p) {
        const pr = p.getBoundingClientRect();
        if (r.right > pr.right - 3.5 || r.left < pr.left + 3.5) bad.push('out of its panel: ' + name(e) + ' ' + [r.left, r.right] + ' in ' + [pr.left, pr.right]);
      } else if (!e.closest('.band') && (r.right > vw + 0.5 || r.left < -0.5)) bad.push('off screen: ' + name(e) + ' ' + [r.left, r.right]);
      const s = getComputedStyle(e);
      if (s.overflowX !== 'visible' && e.scrollWidth > e.clientWidth + 1 && e.tagName !== 'SELECT') bad.push('cut off: ' + name(e));
    }
    return bad;
  })()`);
  check(`${w} px: nothing spills out of a card or the page, nothing cut off`, !bad.length, bad.slice(0, 8));
  if (w === 320) {
    const lines = await evaluate(`new Set([...document.querySelectorAll('#storage .legend > span')].map(s => Math.round(s.getBoundingClientRect().top))).size`);
    check('320 px: the storage legend wraps onto more lines', lines > 1, lines);
  }
  for (const sel of ['#storage', '#modlist']) {
    const r = await evaluate(`(() => { const p = document.querySelector('${sel}').closest('.panel'); p.scrollIntoView(); const r = p.getBoundingClientRect();
      return { x: r.left, y: r.top + scrollY, width: r.width, height: r.height }; })()`);
    const vp = await evaluate(`(() => { const d = document.getElementById('disc'), p = document.querySelector('${sel}').closest('.panel'), r = p.getBoundingClientRect();
      return { x: r.left, y: r.top, width: r.width, height: Math.min(r.height, innerHeight - Math.max(0, r.top)), scale: 1 }; })()`);
    const shot = await send('Page.captureScreenshot', { format: 'png', clip: vp });
    if (shot.result) writeFileSync(join(out, `${sel.slice(1)}-${w}.png`), Buffer.from(shot.result.data, 'base64'));
    void r;
  }
}
process.exit(failed ? 1 : 0);
