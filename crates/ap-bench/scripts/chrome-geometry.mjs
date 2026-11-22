#!/usr/bin/env node
// crates/ap-bench/scripts/chrome-geometry.mjs — records where Chrome itself puts the mockup's
// 2.5D bench elements, as golden data for ap-bench's projection tests.
//
//   node crates/ap-bench/scripts/chrome-geometry.mjs
//
// Opens the mockup bundles (docs/design/bundles/{Tray,Loom,Terrarium}.html) in headless Chrome
// at 1440 × 900 over the DevTools protocol (Node 22+ has a global WebSocket), freezes every
// animation, and records for each transformed element (the tray, a few of its cells, the floor,
// the loom board, a terrarium jar) its untransformed layout box (offset* inside its offset
// parent) plus its projected border quad (`DOM.getBoxModel`, 4 corners clockwise from the
// top-left) and `getBoundingClientRect()`, all relative to the `.graph` stage it sits on.
// The Tray is recorded tilted (44°) and flat (the "Top-down view" toggle). Writes
// crates/ap-bench/tests/data/chrome-geometry.json.
import { spawn } from 'node:child_process';
import { mkdtempSync, writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve, dirname } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(HERE, '../../..');
const OUT = resolve(HERE, '../tests/data/chrome-geometry.json');
const CHROME = process.env.CHROME ?? '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome';
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

const profile = mkdtempSync(join(tmpdir(), 'ap-geom-'));
const port = 9300 + Math.floor(Math.random() * 500);
const chrome = spawn(CHROME, [
  '--headless=new', `--remote-debugging-port=${port}`, `--user-data-dir=${profile}`,
  '--hide-scrollbars', '--force-device-scale-factor=1', '--allow-file-access-from-files', 'about:blank',
], { stdio: 'ignore' });

async function target() {
  for (let i = 0; i < 100; i++) {
    try {
      const list = await (await fetch(`http://127.0.0.1:${port}/json/list`)).json();
      const page = list.find((t) => t.type === 'page');
      if (page) return page.webSocketDebuggerUrl;
    } catch { /* not up yet */ }
    await sleep(100);
  }
  throw new Error('Chrome did not start');
}

const ws = new WebSocket(await target());
await new Promise((r) => ws.addEventListener('open', r, { once: true }));
let nextId = 1;
const pending = new Map();
ws.addEventListener('message', (e) => {
  const m = JSON.parse(e.data);
  if (m.id && pending.has(m.id)) { pending.get(m.id)(m); pending.delete(m.id); }
});
const send = (method, params = {}) => new Promise((res, rej) => {
  const id = nextId++;
  pending.set(id, (m) => (m.error ? rej(new Error(`${method}: ${m.error.message}`)) : res(m.result)));
  ws.send(JSON.stringify({ id, method, params }));
});

/** Evaluates `expr` in the page and returns its JSON value. */
async function evaluate(expr) {
  const r = await send('Runtime.evaluate', { expression: expr, returnByValue: true, awaitPromise: true });
  if (r.exceptionDetails) throw new Error(`evaluate: ${r.exceptionDetails.exception?.description ?? r.exceptionDetails.text}`);
  return r.result.value;
}

/** The projected border quad of the element `window.__pick[i]` (8 numbers, clockwise from top-left). */
async function quad(i) {
  const { result } = await send('Runtime.evaluate', { expression: `window.__pick[${i}]` });
  const { model } = await send('DOM.getBoxModel', { objectId: result.objectId });
  return model.border;
}

// In-page helpers: find transformed elements by their inline transform, freeze animations.
const HELPERS = `
  window.__freeze = () => {
    // stop the mockup's simulation timers so cells don't change between measurements
    for (let i = 1; i < 10000; i++) { clearInterval(i); }
    // finish transitions (the tilt), park looping animations (spools, bob) at t = 0
    for (const a of document.getAnimations()) {
      const end = a.effect?.getComputedTiming().endTime;
      if (Number.isFinite(end)) a.finish(); else { a.pause(); a.currentTime = 0; }
    }
  };
  window.__byTransform = (needle) => [...document.querySelectorAll('div')].filter((d) => d.style.transform.includes(needle));
  window.__graph = (el) => { let p = el; while (p && !p.classList.contains('graph')) p = p.parentElement; return p; };
  window.__box = (el) => ({ left: el.offsetLeft, top: el.offsetTop, width: el.offsetWidth, height: el.offsetHeight });
  window.__rel = (r, g) => ({ x: r.x - g.x, y: r.y - g.y, width: r.width, height: r.height });
  true;
`;

async function open(name) {
  await send('Emulation.setDeviceMetricsOverride', { width: 1440, height: 900, deviceScaleFactor: 1, mobile: false });
  await send('Page.navigate', { url: pathToFileURL(join(ROOT, 'docs/design/bundles', `${name}.html`)).href });
  for (let i = 0; i < 100; i++) {
    await sleep(200);
    const ok = await evaluate(`!!document.querySelector('.graph')`).catch(() => false);
    if (ok) break;
  }
  await sleep(1500); // fonts, first frames
  await evaluate(HELPERS);
}

/**
 * Records the element(s) `window.__pick` (set by `setup`) relative to the `.graph` stage:
 * `box` (layout box in its offset parent), `quad` (projected corners), `bbox`.
 */
async function record(setup) {
  const meta = await evaluate(`(() => {
    ${setup}
    window.__freeze();
    const g = window.__graph(window.__pick[0]).getBoundingClientRect();
    return {
      graph: { x: g.x, y: g.y, width: g.width, height: g.height },
      items: window.__pick.map((el) => ({
        box: window.__box(el),
        bbox: window.__rel(el.getBoundingClientRect(), g),
        className: el.className || '',
        transform: el.style.transform || '',
      })),
    };
  })()`);
  for (let i = 0; i < meta.items.length; i++) {
    const q = await quad(i);
    meta.items[i].quad = q.map((v, k) => (k % 2 ? v - meta.graph.y : v - meta.graph.x));
  }
  return meta;
}

await send('Page.enable');
await send('DOM.enable');
await send('Runtime.enable');
const out = { chrome: (await send('Browser.getVersion')).product, viewport: [1440, 900] };

// ---- Tray: tilted (default 44°) and flat ----
await open('Tray');
const TRAY_CELLS = [0, 39, 520, 1000, 1039, 21 * 40 + 7];
const traySetup = `
  const tray = window.__byTransform('perspective(1600px)')[0];
  const grid = tray.querySelector('[role=grid]');
  const cells = [...grid.children];
  const container = tray.parentElement;
  // plus the first live (lifted, translateY(-6px)) cell
  const live = cells.findIndex((c) => /\\bc[nao]\\b/.test(c.className));
  const picks = [...${JSON.stringify(TRAY_CELLS)}, live];
  window.__pick = [tray, ...picks.map((i) => cells[i]), window.__byTransform('rotateX(62deg)')[0]];
  window.__extra = { container: window.__box(container), grid: window.__box(grid), cells: picks };
`;
out.tray_tilted = await record(traySetup);
out.tray_tilted.cells = await evaluate('window.__extra.cells');
out.tray_tilted.container = await evaluate('window.__extra.container');
out.tray_tilted.grid = await evaluate('window.__extra.grid');
await evaluate(`[...document.querySelectorAll('button')].find((b) => b.textContent.trim() === 'Top-down view').click(); true`);
await sleep(900);
out.tray_flat = await record(traySetup);
out.tray_flat.cells = await evaluate('window.__extra.cells');

// ---- Loom board ----
await open('Loom');
out.loom = await record(`
  const board = window.__byTransform('perspective(1400px)')[0];
  window.__pick = [board, window.__byTransform('rotateX(62deg)')[0]];
`);
out.loom.container = await evaluate(`window.__box(window.__byTransform('perspective(1400px)')[0].parentElement)`);
// flat layout of the print head and the keys, relative to the 860 px container
out.loom.layout = await evaluate(`(() => {
  const container = window.__byTransform('perspective(1400px)')[0].parentElement;
  const c = container.getBoundingClientRect();
  const rel = (el) => { const r = el.getBoundingClientRect(); return { x: r.x - c.x, y: r.y - c.y, width: r.width, height: r.height }; };
  const head = [...container.children].find((d) => d.style.top === '462px');
  const keys = [...container.querySelectorAll('button')];
  return {
    head: rel(head),
    head_children: [...head.children].map(rel),
    keys: keys.map(rel),
    key_on: keys.map((k) => k.className.includes('key-on')),
    key0_children: [...keys[0].children].map(rel),
  };
})()`);

// ---- Terrarium: the first jar (its bob wrapper frozen at translateY(0)) ----
await open('Terrarium');
out.jar = await record(`
  const jar = window.__byTransform('rotateX(52deg)')[0];
  window.__pick = [jar, jar.parentElement, jar.parentElement.parentElement];
`);

writeFileSync(OUT, JSON.stringify(out, null, 2) + '\n');
console.log(`wrote ${OUT}`);
ws.close();
chrome.kill();
await sleep(300);
rmSync(profile, { recursive: true, force: true });
