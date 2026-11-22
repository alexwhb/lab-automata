# Automata Playground — specification

The definition of done. Every requirement has an ID; tests and commits cite them.

Automata Playground is the fourth bench in the lab series ("SPICE for X"): Breadbox (circuits),
Chem Playground (chemistry), Bio Playground (planned), and this one, about **cellular
automata**. It is a native macOS app written in Rust (eframe/egui 0.37, wgpu, AccessKit), in
the same family as its siblings: the same greys, graph-paper ground, header, 2.5D bench and
fonts, plus one magenta accent ramp, a cell-age scale and cell-glyph icons.

The mockup in `docs/design/` is the spec for layout, copy, interactions and look:
`mockup/<Screen>.dc.html` (markup + CSS + a small `class Component` with the logic),
`screenshots/<Screen>-1440.png` / `-390.png` (rendered references, animated, so layout
references rather than pixel goldens) and `bundles/<Screen>.html` (open in Chrome to interact).
They run on a mockup runtime (`{{holes}}`, `<sc-for>`, `<sc-if>`) we don't have: read them as
a spec, not as code to copy.

## Screens

| ID | Mode › tab | Mockup | Board |
|---|---|---|---|
| SCR-1 | Playground › Tray · 2D | `Tray.dc.html` | 1440 × 900 |
| SCR-2 | Playground › Loom · 1D | `Loom.dc.html` | 1440 × 900 |
| SCR-3 | Playground › Terrarium · breed | `Terrarium.dc.html` | 1440 × 900 |
| SCR-4 | Advanced › Rule lab | `RuleLab.dc.html` | 1440 × 960 |
| SCR-5 | Series (reached from the app switcher) | `Series.dc.html` | 1440 × 640 |

## Navigation and header (NAV)

- **NAV-1** Every screen has the header: logo mark, "Automata Playground", `/`, the document
  name (`glider-garden`, `loom`, `terrarium`, `glider-garden.rule`, `series`); the **mode
  switcher** (Playground · Advanced, with their icons); the **app switcher** (Breadbox, Chem
  Playground, Bio Playground, Automata Playground (current)); and the screen's own actions on
  the right.
- **NAV-2** Playground screens have the bench tab row: Tray · 2D, Loom · 1D, Terrarium · breed,
  with the active tab underlined in the accent. Playground opens the last-used bench (Tray at
  first start); Advanced opens the Rule lab.
- **NAV-3** The app switcher: the Automata icon is "current" (pressed look) and goes to the
  Tray. The other three go to the Series screen with that app highlighted; from there "Open"
  launches the sibling app if it is installed (macOS bundle id or the lab-series `dist/`
  folder), and otherwise says it isn't installed (Bio Playground: "planned").
- **NAV-4** Cross-screen actions carry a rule (and pattern) with them: Rule lab "Open in
  Playground" → Tray with that rule and the scrubbed generation's cells; Terrarium "Plant in the
  Tray" / "Analyse in Rule lab" → the first kept jar's rule (or jar 1 if none are kept).
- **NAV-5** Keyboard: Tab / Shift-Tab reach every control in reading order; Enter / Space
  activate; ⌘1–⌘3 switch benches, ⌘4 Rule lab; Space toggles Run/Pause when focus isn't in a
  text field; `S` steps the Tray.

## Tray · 2D (TRAY)

- **TRAY-1** A 40 × 26 torus of Life-like cells on a Petri tray drawn in 2.5D: tilted with
  `perspective(1600px) rotateX(44deg) scale(.96)` (origin bottom centre) over a tilted floor
  grid, with the tray's engraved labels "PETRI TRAY · 40 × 26 · TORUS" and the rule code. The
  "Top-down view" toggle animates (0.4 s ease) to `rotateX(0) scale(.9)`.
- **TRAY-2** Cells use the cell-age scale: newborn (age 1, glowing, white-pink), alive (2–10),
  elder (> 10), each lifted 6 px with its darker side; empty cells; with "Ghost trails" on, a
  dead cell leaves a trail that fades over 3 generations. A legend shows the four.
- **TRAY-3** Draw / Erase / Stamp tools. Draw: press on a cell to toggle it and drag to paint
  that value; Erase: drag to clear; Stamp: click to place the selected stamp centred on the
  cell (wrapping). The hint chip reads "Click or drag to draw" / "Drag to erase" / "Click to
  place <Stamp>". Hovering highlights the cell under the pointer (through the perspective).
- **TRAY-4** Rules list: six cartridges (Life B3/S23 "The classic", HighLife B36/S23 "Has a
  replicator", Seeds B2/S "Explodes", Day & Night B3678/S34678 "Light–dark symmetric", Maze
  B3/S12345 "Grows corridors", Coral B3/S45678 "Slow reef"); picking one sets the rule; the
  cartridge matching the current rule is highlighted (none for a custom rule).
- **TRAY-5** Stamps: Glider, Spaceship (LWSS), Pulsar, Pentadecathlon, R-pentomino, Acorn,
  Glider gun (Gosper), Replicator (HighLife), each previewed as its cells; picking one selects
  the Stamp tool.
- **TRAY-6** Rule panel: the rule code (e.g. `B3/S23`) and name ("Life" or "Custom rule"); the
  Born-with and Survives-with bit rows 0–8 (toggle buttons, `aria-pressed`); "8 neighbours ·
  edges wrap" with its 3 × 3 glyph.
- **TRAY-7** Run / Pause, Step (pauses, then advances one generation), Restart (the starting
  garden: a Gosper gun at (1,1) and a pulsar at (26,12), rule Life), Random soup (30 %
  density), Clear, a `gen 0000` counter, Speed 1–30 gen/s (default 10). Runs on start.
- **TRAY-8** Readouts: a population sparkline of the last 120 generations; Generation, Alive,
  Born, Died tiles; the "what is happening" callout with the mockup's classifier: Extinct,
  Settled, Oscillating, Booming, Collapsing, Churning, with its sentence.
- **TRAY-9** Share copies the tray as an RLE pattern (with `rule =`) to the clipboard and
  confirms with a toast. Pasting an RLE pattern (⌘V) or dropping a `.rle` / `.cells` file arms
  it as a stamp.
- **TRAY-10** Keyboard drawing: the grid is focusable; arrow keys move a cell cursor (drawn as
  a focus ring), Space / Enter applies the current tool at the cursor.

## Loom · 1D (LOOM)

- **LOOM-1** Elementary (Wolfram) cellular automaton: 121 cells, wrapping; each row is woven
  from the one above. The newest 72 rows are drawn as fabric on a board tilted
  `perspective(1400px) rotateX(36deg)`, the newest row in newborn white-pink, older rows in the
  accent, with the fade at the far edge.
- **LOOM-2** The print head bar: two spools that spin while weaving (paused when paused),
  "PRINT HEAD · 121 CELLS · WRAPS", a progress bar (rows filled / 72) and "Rule N".
- **LOOM-3** Eight rule keys, 111 → 000: each shows its three-cell pattern, an arrow and the
  output cell; pressing a key flips that bit of the rule (pressed keys sink 4 px and glow).
- **LOOM-4** Rule panel: the rule number, its 8-bit binary, the blurb ("Chaos · Random-looking"
  for famous rules, "Uncharted rule" otherwise), −1 / +1 / Surprise me. Changing the rule
  restarts the thread.
- **LOOM-5** Famous threads: 30 Chaos, 90 Sierpiński, 110 Universal, 184 Traffic, 54 Particles,
  150 Lace; the current one is highlighted.
- **LOOM-6** First row: Single cell / Random; Speed 2–40 rows/s (default 14); Weave / Pause;
  New thread; `row 0000` counter; Share copies "Rule N" plus the visible rows as text.

## Terrarium · breed (TERR)

- **TERR-1** Six jars, each a 32 × 32 torus running its own Life-like rule (Moore) from a soup
  (38 % density in the centre 20 × 20), drawn in 2.5D (`perspective(900px) rotateX(52deg)
  scale(.86)`, gently bobbing), each in its own ink. Starting rules: Life, HighLife,
  Day & Night, Maze, 2×2, Move. Below each: the rule code, its name (or "Mutant"), Keep /
  Kept (heart), the liveliness meter (mean fraction of cells changing over the last 12 ticks
  × 6, capped at 1; grey when quiet, coral when frantic) and "N alive". The jar must not cover
  the code (the mockup overlaps them; fix it).
- **TERR-2** Breeding: Mutation 1–4 bit flips (default 2); "Breed brood N+1": kept jars carry
  over unchanged (fresh soup), the rest of the six are children whose B1–8 / S0–8 bits are each
  taken from a random parent, then `mutation` random bits flipped (a child with no birth
  counts gets B3). Parents are the kept jars, or all six when none are kept; the note says
  which.
- **TERR-3** Lineage lists each brood's parents' codes, newest first, last 8; "Nothing bred
  yet." before the first.
- **TERR-4** Run / Pause, Fresh soup (re-seed every jar, same rules), `brood N`, Share (copies
  the six rule codes with names).

## Rule lab (LAB)

- **LAB-1** Rule spec: Family (Life-like on; Generations, Larger than Life, Lenia shown
  disabled with an explanation); the 5 × 5 neighbourhood editor (click cells to toggle
  neighbours; the centre is "this cell") with presets Moore r1, von Neumann, Hexagonal,
  Moore r2 and a live "N cells" count; Born / Survives bits 0…N (bits above N are dropped when
  N shrinks); Boundary Torus / Dead edge on a 64 × 40 grid; Seed R-pentomino / Acorn / Soup.
- **LAB-2** Centre: the rule code, neighbourhood name, boundary ("wraps at edges" / "dead
  edges"), `gen 120 / 400 · N alive`; a top-down view of the grid at the scrubbed generation
  (born this generation in newborn colour, alive in the accent, died this generation as
  ghosts); Play / Pause (loops 0 → 400) and the Scrub slider; the population plot over 400
  generations with the stable band from the first repeat, and the playhead.
- **LAB-3** Analysis: Peak population (`N @ gen`), Final population, First repeat (`gen N` or
  `none`), Period; the verdict ("Dies out by gen N." / "Freezes into still lifes at gen N." /
  "Settles into a period-P cycle at gen N." / "No repeat within 400 gens."). Repeats are
  detected exactly (hash, then full comparison).
- **LAB-4** The rule script is the netlist of this mode, and it is **editable**: editing the
  text updates the spec (rule, neighbourhood, grid boundary, seed, gens) and editing the spec
  rewrites the text, in the mockup's format (`* automata netlist · glider-garden.rule`,
  `.rule life_like B3/S23`, `.neigh moore n=8`, `.grid 64x40 boundary=torus`,
  `.seed r_pentomino @ (32,20)`, `.run gens=400`, `.probe population -> plot1`,
  `.probe period -> report`, `.end`). Errors are shown per line and never panic; the last
  valid spec stays in effect.
- **LAB-5** Pattern export: the scrubbed generation as RLE (`x = …, y = …, rule = …`), shown
  truncated in the panel; Export offers Copy RLE, Copy script, Save .rle…, Save .rule…;
  "Open in Playground" (NAV-4).

## Series (SER)

- **SER-1** "SPICE-for-X series · fourth bench", "Same family, one new accent", the four app
  identity cards with their accent / ink / tint / edge ramps (Automata tagged "new"), the
  cell-age scale card, "Shared, unchanged" list and the cell-glyph icon family (five 3 × 3
  glyphs), as in the mockup; plus NAV-3's Open buttons.

## Engine (ENG)

- **ENG-1** `ap-engine` has no UI dependencies: Life-like rules on any neighbourhood inside
  5 × 5 (up to 24 neighbours), torus or dead boundary, any grid size; cell ages and ghost
  trails exactly as the Tray mockup computes them; born / died / population per step.
- **ENG-2** Rulestrings: `B3/S23` (also `b3s23`, `B3/S23/` variants and the old `23/3`
  S/B form) parse and print canonically; counts ≥ 10 are comma-separated.
- **ENG-3** Patterns: RLE read/write (standard, 70-column lines, `#N` / `#C` comments, `rule =`
  header) and plaintext `.cells` read; the eight built-in stamps; place with wrap or clip.
- **ENG-4** Elementary CA: any rule 0–255, wrapping rows, single-cell or random start.
- **ENG-5** Analysis: run N generations keeping every frame; population, peak, final, exact
  first repeat and period; the verdict text.
- **ENG-6** The Tray's mood classifier, the Terrarium's jars, liveliness and breeding (seeded,
  deterministic RNG; the app seeds from the clock, tests seed explicitly), the Rule-lab soup
  (the mockup's LCG, computed in f64 like JavaScript so seed 7 gives the mockup's soup).
- **ENG-7** The rule script parses and prints (LAB-4) with line-numbered errors.
- **ENG-8** No panics on any user input (fuzz-style tests over random strings and edits).

## Correctness (REF)

- **REF-1** Known facts hold: a glider moves (+1, +1) every 4 generations; blinker, pulsar and
  pentadecathlon have periods 2, 3 and 15; the Gosper gun has period 30 and emits a glider
  every 30 generations; the HighLife replicator copies itself; Rule 30's centre column from a
  single cell starts 1,1,0,1,1,1,0,0,1,1,0,0,0,1,0,1,1,0,0,1 (OEIS A051023); Rule 90 row n
  has 2^popcount(n) live cells (Gould's sequence); Rule 184 conserves the number of cells.
- **REF-2** The mockup's own numbers reproduce exactly, from golden data generated by running
  the mockup's JavaScript logic (`scripts/mockup-goldens.mjs`): the Rule lab with
  R-pentomino on the 64 × 40 torus, B3/S23 (148 alive at gen 120; peak 221 @ 148; final 46;
  first repeat gen 390, period 2; "Settles into a period-2 cycle at gen 388."), plus every
  seed × boundary × neighbourhood preset; the Tray's starting garden (gen 30: 90 alive, 27
  born, 57 died) and its population history; the RLE at gen 120 (compared as cell sets).

## Accessibility (A11Y)

- **A11Y-1** Real buttons and links (AccessKit roles Button / Link / Tab / CheckBox / Slider /
  TextInput) with names; toggles report pressed state; disabled controls report disabled.
- **A11Y-2** Body text ≥ 4.5:1 against its background (tested on the tokens); the tray's
  engraved labels are decorative and repeated in accessible text.
- **A11Y-3** Pointer targets: controls drawn smaller than 44 px (the mockup's 32 px buttons,
  28 px bits) keep their drawn size and extend their hit area where padding allows; canvases
  are keyboard operable (TRAY-10, LOOM-3 keys are buttons, jars' Keep buttons).
- **A11Y-4** Every canvas has an accessible description that updates (e.g. "Cell tray, 40 by
  26, generation 30, 90 alive").

## Done means (DONE)

- **DONE-1** Each screen matches its mockup at 1440 px (compared side by side with
  `docs/design/screenshots/*-1440.png`), and still works at 390 px wide: panels stack and the
  page scrolls.
- **DONE-2** Every number shown comes from the engine; nothing is hard-coded from the mockup.
- **DONE-3** What the mockup leaves as placeholders is real here: Share, Export, Open in
  Playground, Plant in the Tray, Analyse in Rule lab, the app switcher, the editable script.
  Generations / Larger than Life / Lenia stay disabled and say so.
- **DONE-4** Tests: engine unit + reference (REF) tests; headless end-to-end scenarios; GUI
  tests that click, drag and type through every screen with `egui_kittest` via AccessKit;
  snapshot renders at 1440 and 390; MCP tests that spawn the real server. `scripts/test-all.sh`
  runs fmt, clippy `-D warnings` and the whole suite, and is green.
- **DONE-5** MCP: `automata-mcp` (stdio, labkit's `lab-mcp`) with engine tools (run a rule on a
  pattern, analyse, elementary rows, parse rule / RLE) and app tools over the live bridge
  (status, navigate, screenshot, describe, set rule, load pattern, run / pause / step, read
  the tray as RLE).
- **DONE-6** `scripts/bundle-macos.sh` builds `dist/Automata Playground.app`.
- **DONE-7** README: what it is, how to run, test, add a rule preset / stamp / famous thread.
- **DONE-8** No sparkle / magic-wand / four-point-star icons anywhere (✨ ✦ ✧ included).
