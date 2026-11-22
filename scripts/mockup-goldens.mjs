#!/usr/bin/env node
// scripts/mockup-goldens.mjs — golden data for ap-engine from the mockup's own JavaScript (SPEC REF-2).
//
//   node scripts/mockup-goldens.mjs            writes crates/ap-engine/tests/golden/*.json
//
// Node 22+, no dependencies. Nothing here re-implements the mockup: the `class Component`
// script of each screen is read verbatim out of docs/design/mockup/<Screen>.dc.html and run
// on a tiny stand-in for the mockup runtime (`DCLogic`: `props`, `state`, synchronous
// `setState`). We call the mockup's own methods (`place`/`seed`/`advance`, `seedGrid`/`run`/
// `rle`/`code`, `weave`/`setRule`) and read the values its `renderVals()` would show (mood,
// verdict, report, script, blurb). The only deviation is called out where it happens: the Rule
// lab's `rle()` truncates its body to 420 characters for display, so we also evaluate a copy of
// that method with the truncation switched off to get the full pattern.
//
// The one place the mockup is not exact is its repeat detection (hash only, no comparison);
// every Rule-lab case also records whether the mockup's reported repeat is a real one
// (`repeatExact`), computed here by comparing the two frames cell by cell.
import { readFileSync, writeFileSync, mkdirSync } from 'node:fs';
import { join, resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const OUT = join(ROOT, 'crates/ap-engine/tests/golden');

// The mockup runtime, reduced to what the logic classes use.
class DCLogic {
  constructor(props) { this.props = props ?? {}; this.state = {}; }
  setState(patch) { Object.assign(this.state, typeof patch === 'function' ? patch(this.state) : patch); }
}

/** Loads `class Component` from a mockup screen, verbatim. */
function loadComponent(screen) {
  const html = readFileSync(join(ROOT, 'docs/design/mockup', `${screen}.dc.html`), 'utf8');
  const m = html.match(/<script type="text\/x-dc" data-dc-script=""[^>]*>([\s\S]*?)<\/script>/);
  if (!m) throw new Error(`no logic script in ${screen}.dc.html`);
  const src = m[1];
  if (!/class Component extends DCLogic/.test(src)) throw new Error(`no class Component in ${screen}`);
  // eslint-disable-next-line no-new-func
  return new Function('DCLogic', `${src}\nreturn Component;`)(DCLogic);
}

const write = (name, data) => {
  const path = join(OUT, name);
  writeFileSync(path, JSON.stringify(data) + '\n');
  console.log(`wrote ${path.slice(ROOT.length + 1)}`);
};

mkdirSync(OUT, { recursive: true });

// ---------------------------------------------------------------- Tray (SCR-1, TRAY-7/8)
{
  const Tray = loadComponent('Tray');
  const t = new Tray({});
  const W = t.W, H = t.H;
  const snap = (st) => ({ age: st.cells.slice(), trail: st.trail.slice() });
  const v0 = t.renderVals();
  const gens = [];
  const snapshots = { 0: snap(t.state) };
  for (let g = 1; g <= 300; g++) {
    t.advance();
    const v = t.renderVals();
    gens.push({ gen: t.state.gen, pop: v.pop, born: t.state.born, died: t.state.died, mood: v.moodTitle, moodText: v.moodText });
    if (g === 1 || g === 30 || g === 300) snapshots[g] = snap(t.state);
  }
  // The classifier on synthetic histories too (hist set directly, as advance()/edit() would).
  let r = 12345;
  const rnd = () => { r = (Math.imul(r, 1664525) + 1013904223) >>> 0; return r / 4294967296; };
  const moods = [];
  const shapes = ['flat', 'zero', 'p2', 'p3', 'grow', 'fall', 'noise', 'short', 'flatThenZero', 'p2Short'];
  for (let k = 0; k < 400; k++) {
    const shape = shapes[k % shapes.length];
    const len = shape === 'short' || shape === 'p2Short' ? 1 + Math.floor(rnd() * 22) : 1 + Math.floor(rnd() * 130);
    const base = Math.floor(rnd() * 300);
    const hist = [];
    for (let i = 0; i < len; i++) {
      let v;
      switch (shape) {
        case 'flat': v = base; break;
        case 'zero': v = i === len - 1 ? 0 : Math.floor(rnd() * 50); break;
        case 'p2': case 'p2Short': v = base + (i % 2) * 7; break;
        case 'p3': v = base + (i % 3) * 5; break;
        case 'grow': v = Math.floor(base * Math.pow(1.03 + rnd() * 0.05, i)); break;
        case 'fall': v = Math.floor((base + 500) * Math.pow(0.97 - rnd() * 0.03, i)); break;
        case 'flatThenZero': v = i < len - 3 ? base : 0; break;
        default: v = Math.floor(rnd() * 400);
      }
      hist.push(v);
    }
    const tt = new Tray({});
    tt.setState({ hist: hist.slice(-120) });
    const v = tt.renderVals();
    moods.push({ hist: hist.slice(-120), mood: v.moodTitle, text: v.moodText });
  }
  write('tray.json', {
    source: 'docs/design/mockup/Tray.dc.html (Component: seed, advance, renderVals mood)',
    w: W, h: H, rule: v0.ruleCode, ruleName: v0.ruleName,
    initialPop: v0.pop, initialMood: v0.moodTitle,
    gens, snapshots, moods,
    carts: v0.carts.map((c) => ({ name: c.name, note: c.note, code: c.code })),
    stamps: t.stampList.map((s) => ({ id: s.id, name: s.name, rows: t.P[s.id] })),
  });
}

// ---------------------------------------------------------------- Rule lab (SCR-4, LAB-2/3/4/5)
{
  const RuleLab = loadComponent('RuleLab');
  // The display truncation in rle() (body > 420 chars → '…') switched off, nothing else changed.
  const rleSrc = RuleLab.prototype.rle.toString();
  const fullSrc = rleSrc.replace('body.length > 420 ?', 'false ?');
  if (fullSrc === rleSrc) throw new Error('RuleLab.rle truncation not found');
  // eslint-disable-next-line no-new-func
  const rleFull = new Function(`return function ${fullSrc}`)();

  const lab = new RuleLab({});
  const rules = {
    life: { b: [3], s: [2, 3] },
    highlife: { b: [3, 6], s: [2, 3] },
    seeds: { b: [2], s: [] },
    daynight: { b: [3, 6, 7, 8], s: [3, 4, 6, 7, 8] },
  };
  const cases = [];
  for (const seed of ['rpent', 'acorn', 'soup']) {
    for (const bound of ['torus', 'dead']) {
      for (const nk of Object.keys(lab.presets)) {
        for (const [rk, rule] of Object.entries(rules)) {
          const off = lab.presets[nk].off.slice();
          // Picking a preset filters the counts to ≤ N, as the mockup's preset buttons do.
          const b = rule.b.filter((n) => n <= off.length), s = rule.s.filter((n) => n <= off.length);
          lab.setState({ b, s, off, bound, seed, gen: 120, playing: false });
          const v = lab.renderVals();
          const fr = lab.firstRepeat, per = lab.period;
          const same = (a, c) => a.length === c.length && a.every((x, i) => x === c[i]);
          const rle = {};
          for (const g of [0, 120, 400]) rle[g] = rleFull.call(lab, lab.frames[g]);
          cases.push({
            seed, bound, neigh: nk, ruleKey: rk, b, s,
            code: v.code, nbName: v.nbName, boundName: v.boundName, nbCount: v.nbCount,
            pops: lab.pops.slice(), firstRepeat: fr, period: per,
            repeatExact: fr >= 0 ? same(lab.frames[fr], lab.frames[fr + per]) : null,
            report: v.report.map((x) => String(x.v)), verdict: v.verdict, script: v.script,
            popNow: v.popNow, rle, rleShown: v.rle,
          });
        }
      }
    }
  }
  write('rulelab.json', { source: 'docs/design/mockup/RuleLab.dc.html (Component: seedGrid, run, rle, code, renderVals)', w: lab.W, h: lab.H, gens: lab.G, cases });
}

// ---------------------------------------------------------------- Loom (SCR-2, LOOM-1/4)
{
  const Loom = loadComponent('Loom');
  const loom = new Loom({});
  const threads = [];
  for (const rule of [30, 90, 110, 184, 54, 150]) {
    loom.setState({ start: 'one' });
    loom.setRule(rule);
    const rows = [loom.state.rows[0].join('')];
    for (let k = 1; k < 100; k++) { loom.weave(); rows.push(loom.state.rows[loom.state.rows.length - 1].join('')); }
    const v = loom.renderVals();
    threads.push({ rule, rows, count: loom.state.count, kept: loom.state.rows.length, rowStr: v.rowStr, progress: v.progress, blurb: v.ruleBlurb, binary: v.binary });
  }
  const blurbs = [];
  for (let rule = 0; rule < 256; rule++) {
    loom.setState({ start: 'one' });
    loom.setRule(rule);
    const v = loom.renderVals();
    blurbs.push({ rule, blurb: v.ruleBlurb, binary: v.binary, keys: v.keys.map((k) => k.on) });
  }
  write('loom.json', { source: 'docs/design/mockup/Loom.dc.html (Component: fresh, weave, setRule, renderVals)', w: loom.W, keep: loom.ROWS, threads, blurbs, famous: loom.famous });
}

// ---------------------------------------------------------------- Terrarium (SCR-3): the deterministic bits
{
  const Terr = loadComponent('Terrarium');
  const terr = new Terr({});
  write('terrarium.json', {
    source: 'docs/design/mockup/Terrarium.dc.html (named map, starting rules; soups use Math.random so are not golden)',
    named: terr.named,
    start: terr.state.jars.map((j) => terr.code(j)),
    names: terr.renderVals().jars.map((j) => j.name),
  });
}
