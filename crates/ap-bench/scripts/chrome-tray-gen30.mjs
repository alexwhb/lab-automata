#!/usr/bin/env node
// crates/ap-bench/scripts/chrome-tray-gen30.mjs — the Tray mockup as Chrome draws it in a known
// state, to compare the bench's look (cell glows, sides, trails) pixel for pixel:
//
//   node crates/ap-bench/scripts/chrome-tray-gen30.mjs [out.png]
//
// Opens docs/design/bundles/Tray.html in headless Chrome at 1440 × 900 (DPR 1), stops the
// mockup's timer, presses Restart and Step 30 times (the starting garden at gen 30: 90 alive,
// the same cells as the app's Tray after `step_n(30)`), finishes every CSS transition, and
// writes a screenshot (default target/compare/Tray-1440-chrome-gen30.png). The animated
// reference docs/design/screenshots/Tray-1440.png catches cells mid-transition (box-shadow and
// background .2s, transform .14s), so it shows more glow than any settled state.
import { spawn } from 'node:child_process';
import { mkdtempSync, mkdirSync, writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve, dirname } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(HERE, '../../..');
const OUT = resolve(process.argv[2] ?? join(ROOT, 'target/compare/Tray-1440-chrome-gen30.png'));
const CHROME = process.env.CHROME ?? '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome';
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const profile = mkdtempSync(join(tmpdir(), 'ap-glow-'));
const port = 9300 + Math.floor(Math.random() * 500);
const chrome = spawn(CHROME, ['--headless=new', `--remote-debugging-port=${port}`, `--user-data-dir=${profile}`,
  '--hide-scrollbars', '--force-device-scale-factor=1', '--allow-file-access-from-files', 'about:blank'], { stdio: 'ignore' });
let wsUrl;
for (let i = 0; i < 100 && !wsUrl; i++) {
  try { wsUrl = (await (await fetch(`http://127.0.0.1:${port}/json/list`)).json()).find((t) => t.type === 'page')?.webSocketDebuggerUrl; } catch {}
  if (!wsUrl) await sleep(100);
}
const ws = new WebSocket(wsUrl);
await new Promise((r) => ws.addEventListener('open', r, { once: true }));
let nextId = 1; const pending = new Map();
ws.addEventListener('message', (e) => { const m = JSON.parse(e.data); if (m.id && pending.has(m.id)) { pending.get(m.id)(m); pending.delete(m.id); } });
const send = (method, params = {}) => new Promise((res, rej) => { const id = nextId++; pending.set(id, (m) => (m.error ? rej(new Error(m.error.message)) : res(m.result))); ws.send(JSON.stringify({ id, method, params })); });
const evaluate = async (expression) => { const r = await send('Runtime.evaluate', { expression, returnByValue: true, awaitPromise: true }); if (r.exceptionDetails) throw new Error(JSON.stringify(r.exceptionDetails)); return r.result.value; };
await send('Page.enable'); await send('Runtime.enable');
await send('Emulation.setDeviceMetricsOverride', { width: 1440, height: 900, deviceScaleFactor: 1, mobile: false });
await send('Page.navigate', { url: pathToFileURL(join(ROOT, 'docs/design/bundles/Tray.html')).href });
for (let i = 0; i < 100; i++) { await sleep(200); if (await evaluate(`!!document.querySelector('.graph')`).catch(() => false)) break; }
await sleep(1500);
const info = await evaluate(`(async () => {
  for (let i = 1; i < 10000; i++) clearInterval(i);
  const btn = (t) => [...document.querySelectorAll('button')].find((b) => b.textContent.trim() === t);
  const wait = () => new Promise((r) => setTimeout(r, 30));
  btn('Restart').click(); await wait();
  for (let i = 0; i < 30; i++) { btn('Step').click(); await wait(); }
  await new Promise((r) => setTimeout(r, 600));
  for (const a of document.getAnimations()) { const end = a.effect?.getComputedTiming().endTime; if (Number.isFinite(end)) a.finish(); else { a.pause(); a.currentTime = 0; } }
  await new Promise((r) => setTimeout(r, 100));
  const txt = document.body.innerText;
  return { gen: (txt.match(/gen \\d+/) || [''])[0], alive: (txt.match(/(\\d+) alive/) || [''])[0] };
})()`);
console.log(JSON.stringify(info));
const shot = await send('Page.captureScreenshot', { format: 'png' });
mkdirSync(dirname(OUT), { recursive: true });
writeFileSync(OUT, Buffer.from(shot.data, 'base64'));
console.log(`wrote ${OUT}`);
ws.close();
const exited = new Promise((r) => chrome.once('exit', r));
chrome.kill();
await exited;
rmSync(profile, { recursive: true, force: true, maxRetries: 5, retryDelay: 200 });
