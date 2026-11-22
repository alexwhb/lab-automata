# Architecture

```
lab-series/
  labkit/                      shared crates (another repo — never edit from here)
    lab-theme                  fonts, Palette + egui style builder, contrast helpers
    lab-ui                     layout maths (flex_wrap, auto_grid), graph-paper painting, galleys
    lab-mcp                    stdio MCP server + live bridge (Unix socket) to the running app
  automata-playground/
    crates/ap-theme            Automata tokens: greys, magenta ramp, cell-age scale, identities
    crates/ap-engine           the automata engine (no UI): rules, grids, patterns, RLE,
                               elementary CA, analysis, mood, breeding, rule script
    crates/ap-bench            2.5D bench drawing (egui + ap-theme, no engine): CSS-style
                               perspective projection with inverse hit-testing, floor, tray,
                               loom fabric + print head + keys, jars, top-down grid
    crates/automata-playground the egui app: header, router, widgets, five screens, MCP bridge
    crates/ap-mcp              the MCP server binary `automata-mcp`
    tests/e2e  (optional)      headless scenarios across crates
```

Dependency direction: `ap-theme` ← `ap-bench` ← app → `ap-engine`. `ap-engine` and `ap-bench`
never depend on each other: screens adapt engine state into bench scene inputs.

## `ap-engine` contract

Built (wave 1). The rustdoc is the reference (`cargo doc -p ap-engine --open`; every public
item is documented, `#![warn(missing_docs)]`); this is the map. Everything commonly used is
re-exported at the crate root (`use ap_engine::{Grid, LifeRule, …}`). No UI dependencies, only
`thiserror`. Nothing panics on any input: parsers return errors with line numbers, edits ignore
out-of-range coordinates and counts, sizes are clamped (grids 1–1024, runs 1–256 × 0–2000
gens, weaves 1–4096).

- **`rule`** — `Neighbourhood` (24-bit mask of offsets in the 5 × 5 square, never (0,0); `Copy`,
  `Eq`, `Hash`): consts `MOORE`, `VON_NEUMANN`, `HEXAGONAL`, `MOORE2`, `empty()`,
  `from_offsets(iter) -> Result<_, NeighbourhoodError>`, `contains/set/toggle(dx, dy)`, `len()`,
  `offsets()` (row-major), `radius()`, `preset() -> Option<NeighbourhoodPreset>` (recognises a
  custom set equal to a preset), `key()` (`moore`/`vn`/`hex`/`moore2`/`custom`), `name()`
  ("Moore r1" … or "Custom (N cells)"). `NeighbourhoodPreset { Moore, VonNeumann, Hexagonal,
  Moore2 }` with `ALL`, `key()`, `name()`, `neighbourhood()`, `from_key()`. `CountSet` (counts
  0–24; `of(&[..])`, `contains`, `set`, `toggle`, `iter`, `len`, `max`, `up_to`).
  `LifeRule { birth, survive, neighbourhood }` (`Copy`, `Eq`, `Hash`; counts never exceed N):
  `LIFE`, `new(b, s, nb)`, `moore(&[b], &[s])`, `parse(text)` (Moore) / `parse_with(text, nb)`
  → `Result<_, RuleParseError>` (`B3/S23`, `b3s23`, `B3/S23/`, `S23/B3`, `23/3`, Golly `:T…`
  suffix ignored; comma mode `B3,10/S2,3`), `code()` canonical (`Display` / `FromStr` too),
  `born_with(n)`, `survives_with(n)`, `set_birth/set_survive/toggle_birth/toggle_survive(n)`
  (no-op above N), `with_neighbourhood(nb)` / `set_neighbourhood(nb)` (drop counts > N, as the
  Rule lab does), `toggle_neighbour(dx, dy)` (the 5 × 5 editor's click). `Boundary { Torus,
  Dead }` with `ALL`, `key()` (`torus`/`dead`), `label()` ("Torus"/"Dead edge"),
  `description()` ("wraps at edges"/"dead edges"), `from_key()`.
- **`presets`** — `Cartridge { id, name, note, rule }`, `CARTRIDGES` (the Tray's six),
  `cartridge(id)`, `matching_cartridge(&rule)` (the lit cartridge, `None` = "Custom rule");
  `NAMED` (the Terrarium names map incl. "2×2", "Move"), `rule_name(&rule) -> Option<&str>`;
  `TERRARIUM_START`; `TRAY_W` (40), `TRAY_H` (26), `TRAY_BOUNDARY` (torus),
  `starting_garden() -> Grid` (gun at (1,1) + pulsar at (26,12); run it with `LifeRule::LIFE`).
- **`grid`** — `Grid` (`Clone`, `Eq`): `new(w, h)`, `random(w, h, density, &mut Rng)`,
  `width/height/len`, `age(x, y) -> u16` (0 dead, 1 newborn, +1 per survived gen, saturating),
  `trail(x, y) -> u8` (3 on death, −1 per gen, 0 while alive), `is_alive`, `cell_state(x, y) ->
  CellState { Empty, Trail(1..=3), Newborn, Alive, Elder }` (TRAY-2; `ELDER_AGE` = 10),
  `ages()`, `trails()`, `set(x, y, alive)` (alive = newborn, like Draw), `toggle(x, y) ->
  Option<bool>`, `clear()`, `clear_trails()`, `population()`, `alive_cells()`, `bbox() ->
  Option<BBox { x, y, w, h }>`, `place(&Pattern, x: i64, y: i64, Anchor::{TopLeft, Center},
  boundary) -> usize` (Center = top-left at (x − ⌊w/2⌋, y − ⌊h/2⌋); wraps on torus, clips on
  dead), `step(&rule, boundary) -> StepStats { born, died, population }` (exactly the Tray
  mockup's `advance()`; branch-free shifted-plane counting, any neighbourhood).
- **`pattern`** — `Pattern` (w × h + sorted live cells; `new`, `from_rows(&["..O", …])`,
  `from_grid(&grid)` (bbox-trimmed), `from_grid_region(&grid, x, y, w, h)`, `width/height/cells/
  population/contains`, `trimmed()`, `to_rows()`). `PatternFile { pattern, name, author,
  comments, rule: Option<String> }` with `life_rule()`. `PatternError { line, message }`.
  `library::{Stamp { id, name, rows }, STAMPS, stamp(id), pattern(id)}` — the eight stamps
  verbatim (`glider`, `lwss` "Spaceship", `pulsar`, `penta` "Pentadecathlon", `rpent`
  "R-pentomino", `acorn`, `gun` "Glider gun", `rep` "Replicator"). `rle::parse(text) ->
  Result<PatternFile, _>`, `rle::write(&pattern, Option<&LifeRule>, Option<name>) -> String`
  (standard, `n$` runs, ≤ 70-column body lines), `rle::write_file(&PatternFile)`,
  `rle::is_header`. `cells::parse` / `cells::write` (plaintext). `parse_any(text)` (detects RLE
  vs plaintext, for paste), `parse_file(file_name, text)` (for drop), `detect(text) -> Format`.
- **`elementary`** — `Elementary(u8)`: `output(l, c, r)`, `output_for(p)`, `toggled(p)` (a key
  press), `next_row(&[bool]) -> Vec<bool>` (wraps), `binary()`, `blurb()`, `famous()`; free
  `next_row(rule, row)`, `key_pattern(p) -> [bool; 3]`, `FAMOUS` (`FamousThread { rule, name,
  note }` × 6), `famous(rule)`, `blurb(rule)` ("Chaos · Random-looking" / "Uncharted rule").
  `Weave` (`LOOM_WIDTH` 121, `LOOM_ROWS` 72): `new(rule, WeaveStart::{Single, Random(seed)})`,
  `with_size(width, keep, rule, start)`, `weave() -> &[bool]`, `rows()` (oldest first),
  `newest()`, `len()`, `count()` (rows since start incl. the first: the `row 0000` counter),
  `progress()` (0–1), `set_rule` / `set_start` / `restart` (all restart the thread),
  `rows_text(alive, dead)`, `share_text()` ("Rule N" + rows as `#`/`.`).
- **`analysis`** — `RunSpec { rule, boundary, w, h, seed, gens }` (`Default` = the mockup: Life,
  torus, `LAB_W` 64 × `LAB_H` 40, R-pentomino, `LAB_GENS` 400), `validate() -> Result<(),
  SpecError>`, `clamped()`. `Seed { RPentomino, Acorn, Soup { seed: u32, density: f64 } }`
  (`MOCKUP_SOUP` = seed 7 / 0.33; `key()`, `label()`, `centre(w, h)`, `grid(w, h, boundary)`):
  stamps centred at (⌊w/2⌋, ⌊h/2⌋) = the mockup's (31,19) / (29,19); soup = `js_lcg_next` (the
  mockup LCG in f64, rounding like JavaScript) in w/4 < x < 3w/4, h/5 < y < 4h/5 (`soup_grid`).
  `run(&spec) -> Run` (clamped spec): `frames()` / `frame(g) -> Option<&Frame>` (bitsets:
  `get`, `population`, `alive_cells`, `to_pattern`, `trimmed_pattern`, `to_grid`),
  `populations()`, `population(g)`, `first_repeat() -> Option<Repeat { start, period }>` (exact:
  hash then full compare; `seen_at()` = start + period), `cell(g, x, y) -> CellChange { Empty,
  Born, Alive, Ghost }` (top-down view), `pattern(g)`, `rle(g)` (full RLE with `rule =`),
  `report() -> Report { peak, peak_gen, final_pop, first_repeat, first_repeat_gen, period,
  extinct_gen, verdict }` with `peak_text()` ("221 @ 148"), `first_repeat_text()` ("gen 390" /
  "none"), `period_text()` ("2" / "—").
- **`mood`** — `Mood { Extinct, Settled, Oscillating, Booming, Collapsing, Churning }`:
  `classify(&[usize])` (the Tray mockup's classifier verbatim), `title()`, `text()`. `History`
  (last `HISTORY_LEN` = 120 populations): `new(pop)`, `push(pop)` (after a step),
  `replace_last(pop)` (after an edit), `reset(pop)`, `values()`, `latest()`, `scale_max()`
  (≥ 10, the sparkline's scale), `mood()`.
- **`breed`** — `Jar` (32 × 32 torus, Moore): `new(rule, &mut Rng)` (soup 0.38 in 5 < x,y < 26),
  `reseed`, `tick() -> f64` (changed fraction), `activity()` (last 12), `liveliness()`
  (min(1, mean × 6)), `population()`, `grid()`, `rule()`, `name()` (or "Mutant").
  `Liveliness::of(v) -> { Quiet (< .08), Lively, Frantic (> .8) }` (meter colour).
  `child(parents, mutation, &mut Rng) -> LifeRule`, `breed(rules, kept, mutation, &mut Rng) ->
  Brood { rules, parents }` (TERR-2). `Lineage` (`push`, `newest_first()`, `len`, `is_empty`;
  last 8) of `LineageEntry { brood, parents: Vec<String> }`. `Terrarium` (the whole bench,
  seeded): `new(seed)`, `with_rules(&rules, seed)`, `jars()`, `jar(i)`, `toggle_keep(i)`,
  `is_kept(i)`, `kept()`, `tick()`, `reseed()`, `breed()`, `brood()`, `lineage()`,
  `mutation()` / `set_mutation(1..=4)`, `parents()`, `breed_label()` ("Breed brood N+1"),
  `keep_note()`, `chosen_rule()` (NAV-4: first kept jar, else jar 1), `share_text()`.
- **`script`** — `RuleScript { name, rule, w, h, boundary, seed, gens, probes: Vec<Probe {
  quantity: ProbeQuantity, target }> }`: `Default` (the mockup's script), `from_spec(name,
  &spec)`, `spec() -> RunSpec`, `set_spec(&spec)`, `to_text()` (exactly the mockup's format, no
  trailing newline), `parse(text) -> Result<RuleScript, Vec<ScriptError { line, message }>>`
  (lenient spacing/case, `*` comments, all errors with lines, sorted; `.rule .neigh .grid .seed
  .run` required, `.probe` / `.end` optional). Custom neighbourhoods print and parse as `.neigh
  custom n=3 (0,-1) (-2,1) (2,2)`; stamp seeds always sit at the grid centre (`@ (x,y)`
  optional, must be the centre). `parse(to_text(s)) == s` for every valid script.
- **`rng`** — `Rng` (SplitMix64): `new(seed)`, `from_clock()`, `next_u64`, `next_f64`,
  `below(n)`, `chance(p)`, `seed()`.

Golden data (SPEC REF-2): `node scripts/mockup-goldens.mjs` runs each mockup's `class
Component` verbatim (loaded from `docs/design/mockup/*.dc.html`) and writes
`crates/ap-engine/tests/golden/*.json`; `tests/golden.rs` must match exactly.

## `ap-bench` contract

Pure drawing + geometry with egui types; inputs are plain data so it knows nothing about the
engine. The rustdoc is the reference (`cargo doc -p ap-bench --open`);
`cargo run -p ap-bench --example scenes` shows every scene interactively.

- **Projection** (`projection.rs`): `Tilt { perspective, angle_deg, scale, origin }` (CSS
  `perspective(P) rotateX(θ) scale(s)` + `transform-origin` as fractions; `Tilt::lerp` = CSS
  transition interpolation), `View { origin, zoom }` (stage CSS px → screen points; zoom
  shrinks a scene on narrow windows), `Projection::new(tilt, rect_in_stage, view)` with
  `project(local) -> Pos2`, `unproject(screen) -> Option<local>` (ray–plane; `None` above the
  horizon / behind the eye), `unproject_inside`, `quad(local_rect) -> [Pos2; 4]` (clockwise
  from top-left), `bounds`, `magnification`. Validated against Chrome
  (`tests/chrome_geometry.rs`, data from `crates/ap-bench/scripts/chrome-geometry.mjs`).
- **Drawing** (`draw.rs`): `Batch::new(&painter)` collects projected shapes in paint order into
  one `Mesh` (shares the font atlas): `fill_rect`, `fill_rrect(_with)` (gradients),
  `fill_poly`, `fill_circle`, `border` (CSS border), `ring`, `stroke_rrect`, `stroke_circle`,
  `line`, `text` (galley glyphs projected vertex by vertex), `blurred_rect` (box-shadow glow,
  σ = blur/2), `blurred_ellipse` (`filter: blur`, cached), then `paint(&painter)`. Helpers:
  `layout_text` (letter-spacing), `galley_top_for_css_top`, `css_line_metrics`.
- **Animation** (`anim.rs`): `cubic_bezier`, `ease`, `ease_in_out`, `ease_out`;
  `Transition::new(value, secs, ease)` + `set_target` / `step(dt) -> animating` / `value()`
  (the tray tilt: 0.4 s ease); `spool_angle(secs)` (2.4 s/turn); `jar_bob(secs, index)`.
- **Ground** (`stage.rs`): `stage(painter, rect)` paints `.graph` (screens call it first);
  `floor(batch, floor_rect, view, clip)`, `floor_band`, `contact_shadow`, `FLOOR_TILT`.
- **Tray** (`tray.rs`): `TrayScene { cols, rows, cell: &dyn Fn(col,row) -> CellVisual { Empty,
  Trail(1..=3), Live(age) }, tilt (0 flat … 1 tilted), tilt_deg (44), hover, cursor,
  label_left, label_right, zoom }` with `TrayScene::new`, `fit_zoom(avail)`,
  `content_height()`, `geometry(stage) -> TrayGeometry` (cheap; hit-test before painting),
  `paint(painter, stage) -> TrayGeometry`, `description(gen, alive)`. `TrayGeometry`:
  `cell_at(screen)` (exact: lifted faces win, gaps and vacated sides are `None`),
  `cell_near(screen)` (nearest cell while on the grid — use while drag-painting),
  `cell_quad` / `cell_rect` (visible face, for chips and focus rings), `tray_rect`,
  `grid_rect`, `projection()`. `tray_tilt(t, deg)` gives the interpolated transform.
- **Loom** (`loom.rs`): `LoomScene { rows: &[&[bool]] (oldest first, newest drawn newborn),
  width (121), capacity (72), head_label, rule_label, progress, spool_angle, zoom }` with
  `fit_zoom`, `geometry(stage) -> LoomGeometry { board: Projection, head, keys: [Rect; 8]
  (111 → 000), caption }`, `paint(painter, stage)` (floor, board, fade, head; not the keys),
  `description(rule, rows)`. Keys: `paint_key(painter, rect, KeyVisual { pattern, on, hovered,
  pressed, focused })` (`KeyVisual::for_rule(rule, p)`); `LoomGeometry::key_target(i)`
  includes the 7 px drop for the button's hit area; `fabric_cell_at`.
- **Jar** (`jar.rs`): `JarScene { size (32), cell: &dyn Fn(x,y) -> bool, ink, bob }` with
  `paint(painter, area)` where `area` is the card's jar area; reserve
  `JarScene::FOOTPRINT` (272 × 250) rather than the mockup's 228 px so the slab clears the
  rule code (TERR-1). `JarScene::description(code, alive)`.
- **Rule lab** (`topdown.rs`): `TopDownGrid { cols, rows, cell: &dyn Fn(x,y) -> TopCell
  { Empty, Born, Alive, Ghost } }` with `size_for(avail_w)`, `paint(painter, min, avail_w) ->
  TopDownGeometry { well, scale }` (`cell_at`, `cell_rect`), `description`.
- **A11y** (`a11y.rs`): spec-worded descriptions (`tray`, `loom`, `jar`, `top_down`) and
  `fill(template, values)` for custom wording; the screens set them on the canvas response.
- **Tokens** (`tokens.rs`): re-exports the `ap_theme` colours the bench uses plus a marked
  "local tokens" block (bench-only values with their CSS source; candidates for `ap-theme`).
- Tests: projection maths, Chrome goldens (max deviation < 0.01 px), hit-testing at tilts
  0 / 0.25 / 0.5 / 0.75 / 1 and zooms, snapshots in `crates/ap-bench/tests/snapshots/`
  (`UPDATE_SNAPSHOTS=1` to refresh), an ignored release perf test for the frame mesh build.

## The app (`automata-playground`)

- `router.rs`: `Mode { Playground, Advanced }`, `Route { Tray, Loom, Terrarium, RuleLab,
  Series }` with `mode()`, tab label, document name, mockup file, board size.
- `app.rs`: `AutomataApp` (route, per-screen state, toasts, clipboard / save hooks),
  `AppOptions { start, rng_seed, autoplay, out_dir, mcp_socket }` (`AppOptions::test()` is
  deterministic: fixed seed, paused, files into a temp dir).
- `widgets/`: the mockup's components — header (brand, mode switcher, app switcher, actions),
  bench tab row, `.ghost` / `.prim` / `.seg` / `.mini` / `.tog` / `.keep` buttons, `.bit`,
  `.cart`, `.stamp`, `.lbl`, `.card`, stat tile, callout, chip, slider, sparkline / population
  plot, icons (the mockup's SVG paths drawn as egui shapes), toasts.
- `screens/`: `tray.rs`, `loom.rs`, `terrarium.rs`, `rule_lab.rs`, `series.rs`, each with a
  `State` and `fn ui(&mut State, &mut Ctx, &mut egui::Ui)`.
- Time: each screen advances from `ctx.input(|i| i.stable_dt)` accumulated against its speed,
  and requests repaints while running; tests drive it with `harness.step()` or call the state's
  `step()` directly.
- `control.rs`: answers lab-mcp bridge requests each frame (status, navigate, screenshot,
  describe + Automata methods).

## MCP

`automata-mcp` (crate `ap-mcp`) follows `../chem-playground/crates/chem-mcp`: engine tools run
in-process; app tools forward to the running app over `lab_mcp::bridge` (socket path from
`AUTOMATA_MCP_SOCKET`, default under the user's runtime dir).
