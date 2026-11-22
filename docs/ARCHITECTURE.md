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
    crates/ap-shared           UI-free vocabulary shared by the app and ap-mcp: identity, screens,
                               bridge socket, Automata bridge methods + payloads, pattern / rule input
    crates/ap-mcp              the MCP server binary `automata-mcp`
    tests/e2e  (optional)      headless scenarios across crates
```

Dependency direction: `ap-theme` ← `ap-bench` ← app → `ap-engine`. `ap-engine` and `ap-bench`
never depend on each other: screens adapt engine state into bench scene inputs. MCP: app →
`ap-shared` → `ap-engine` + `lab-mcp`; `ap-mcp` → `ap-shared`, `ap-engine`, `lab-mcp` (never the
app or egui; its tests use the app as a dev-dependency).

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
  `layout_text` (letter-spacing), `galley_top_for_css_top`, `css_line_metrics`,
  `css_blur_sigma(r)` (the σ Chrome / Skia actually draws a CSS blur radius with: three box
  blurs, 16 px → 7.48, 10 px → 4.47; the tray's cell glows use it).
  `scripts/chrome-tray-gen30.mjs` renders the Tray bundle at gen 30 with transitions finished,
  for pixel comparisons in a settled state.
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

The shell (wave 1) owns everything here except the four bench screens' files, which wave 2
fills in. The rustdoc is the reference (`cargo doc -p automata-playground --open`).

| module | what |
|---|---|
| `lib.rs` | `APP_NAME`, `APP_ID` (`dev.labseries.AutomataPlayground`), `native_options()` (1440 × 900, min 390 × 600, wgpu presenting at vsync, window persisted), `run()` |
| `platform.rs` | OS glue: patterns the system opens with the app (TRAY-9). The macOS Apple Event `odoc` (Finder "Open With", the Dock, `open -a … glider.rle`) and pattern paths on the command line feed one queue (`queue_open`); the app registered with `set_receiver` (the real window, or a test harness) drains it at the start of every frame (`deliver` → `open_pattern`: `tray.open_file(path)` arms the custom stamp, goes to the Tray, logs `opened pattern <path>`; errors toast) |
| `router.rs` | `Mode { Playground, Advanced }`; `Route { Tray, Loom, Terrarium, RuleLab, Series }` with `mode() -> Option<Mode>` (Series: none), `is_bench()`, `tab_label()`, `title()`, `document()`, `mockup()`, `board_size()`, `panel_height()` (844 / 904: the stacked column height), `shortcut_digit()`, `id()` / `from_id()` |
| `app.rs` | `AutomataApp` (route, last bench, all screen states, toasts, outbox) and `AppOptions { start, rng_seed, autoplay, out_dir, save_dialog, launcher, load_persisted, mcp_socket }`; `AppOptions::test()` = seed 7, paused, no persistence, no dialogs, files into a fresh temp dir, a recording launcher. `frame(ui)` draws one frame (tests call it through kittest). Persists `{ route, last_bench }` in eframe storage. |
| `ctx.rs` | `Ctx` (the per-frame screen context, incl. frame pacing: `animate`, `wake_in`, `dt`), `Carry` (NAV-4 payload), `NavRequest`, `Command` (Space / `S`), `Services`, `Ticker` (`steps`, `due_in`) |
| `launcher.rs` | `AppId { Breadbox, Chem, Bio, Automata }` (identity, switcher label, bundle id, availability, tag) and the injectable `Launcher` (`system()`: Spotlight by bundle id → `/Applications` → `../<repo>/dist/*.app`, then `open`; `recording(&[installed])` for tests; `from_fn`). Bundle ids: Breadbox `app.breadbox.Breadbox`, Chem `dev.labseries.ChemPlayground`; Bio is planned. |
| `layout.rs` | `Column` specs (`aside`, `readout`, `main` + `flex` / `max` / `pad` / `gap` / borders / `scroll`), `resolve()` (lab-ui flex maths + `max-width`), `columns()` (lays out a page row: side by side they fill the window under the header, stacked each is `route.panel_height()` and the page scrolls), `stage()` (`.graph`), `bench()` (tab row + stage for a Playground screen) |
| `widgets/` | the mockup's components (catalogue in `widgets/mod.rs`): `header` (NAV-1/3), `tabs::bench_tabs` (NAV-2), `Button` (`ghost` / `prim` / `.big`), `SegGroup` (`.seg` header / tool, `.mini` wells), `MiniButton`, `Toggle` (`.tog`), `Keep` (`.keep`), `Bit` / `BitRow` (Tray / Rule lab), `NbGrid`, `Cartridge`, `StampTile`, `Preset`, `lbl` / `lbl_colored`, `Card` (`series()` / `panel()`), `StatTile`, `Callout`, `verdict`, `Chip`, `CodeBlock`, `Slider`, `Plot` (`sparkline` / `series`, stable band, playhead), `CellGlyph`, `Icon` (every mockup icon from its SVG path data), `Toasts`, `counter`, `hit` (44 px targets, tiled within dense groups), `flow` (wrapping bar layout), `text` (CSS line boxes, wrapped text), `gallery` |
| `control.rs` | the app side of the MCP bridge: answers lab-mcp bridge requests at the start of each frame (status, navigate, screenshot, describe + the Automata `tray.*` / `loom.*` / `terrarium.*` / `lab.*` methods on the screens' control APIs); `AppOptions::mcp_socket` (default: the shared socket; `None` in tests and smoke), `App::logic` for hidden windows, `Screen::describe` the summary. See § MCP |
| `screens/` | `Screen` trait, `Screens` (all states, kept alive across navigation); `series.rs` (complete); `tray.rs`, `loom.rs`, `terrarium.rs`, `rule_lab.rs` (placeholders: page frame, header / tab-row actions, keyboard hooks — wave 2 replaces them) |

### How a screen plugs in

A screen is a `pub struct State` (with `Default`) in `src/screens/<name>.rs` that implements
`screens::Screen`. The app owns it (`Screens::<name>`), so it survives navigation; only the
active screen is drawn and ticked.

```rust
use super::Screen;
use crate::ctx::{Carry, Command, Ctx, Ticker};
use crate::layout::{self, Column};
use crate::widgets::tabs::RowSlot;
use crate::widgets::{Icon, SegGroup, SegItem, SegStyle, counter, ghost, prim};

pub const COLUMNS: [Column; 3] = [
    Column::aside("Cartridges and stamps", 248.0).pad(14.0, 12.0).border_right(),
    Column::main("Bench"),
    Column::readout("Rule and readouts").gap(18.0),
];

#[derive(Default)]
pub struct State { running: bool, speed: f32, ticker: Ticker, started: bool /* + engine state */ }

impl Screen for State {
    // first frame on this route (and every return to it)
    fn enter(&mut self, ctx: &mut Ctx) {
        if !self.started { self.started = true; self.running = ctx.autoplay(); /* seed from ctx.rng_seed() */ }
    }
    // the header's right slot: left to right, 8 px gaps, wraps at 390; make items 32 px tall
    fn header_actions(&mut self, ctx: &mut Ctx, ui: &mut egui::Ui) {
        let run = if self.running { prim("Pause").icon(Icon::Pause) } else { prim("Run").icon(Icon::Play) };
        if run.show(ui).clicked() { self.running = !self.running; }
        counter(ui, &format!("gen {:04}", 0));
        if ghost("Share").show(ui).clicked() { ctx.copy("x = …"); ctx.toast("Copied the tray as RLE"); }
    }
    // the bench tab row's right side: one `slot.add` per flex item (or `slot.hint("…")`)
    fn tab_actions(&mut self, _ctx: &mut Ctx, slot: &mut RowSlot) {
        slot.add(|ui| SegGroup::new("Tool", &TOOLS, Some(0)).style(SegStyle::Tool).show(ui));
        slot.add(|ui| ghost("Random soup").icon(Icon::Dice).height(30.0).show(ui));
    }
    // Space (nothing focused) → RunPause; S (no text field focused) → Step
    fn command(&mut self, cmd: Command, _ctx: &mut Ctx) -> bool {
        match cmd { Command::RunPause => self.running = !self.running, Command::Step => { /* step */ } }
        true
    }
    // NAV-4: a rule (+ RLE) handed over by another screen, after `enter`
    fn receive(&mut self, carry: Carry, ctx: &mut Ctx) { /* parse carry.rule / carry.rle */ ctx.toast("…"); }
    // time passes (before anything is drawn, so the header's counter matches the body)
    fn tick(&mut self, ctx: &mut Ctx) {
        if self.running {
            for _ in 0..self.ticker.steps(ctx.dt(), self.speed) { /* engine step */ }
            ctx.wake_in(self.ticker.due_in(self.speed)); // the next frame when a step is due
        }
    }
    // the body under the header
    fn ui(&mut self, ctx: &mut Ctx, ui: &mut egui::Ui) {
        layout::columns(ui, ctx, &COLUMNS, |ui, ctx, i| match i {
            0 => { /* rules aside: ui is top-down, padded, `gap` spacing, scrolls */ }
            1 => layout::bench(ui, ctx, self, |s, ctx, stage| { /* draw on the .graph stage */ }),
            _ => { /* readouts */ }
        });
    }
    fn describe(&self) -> String { "Cell tray, 40 by 26, generation 30, 90 alive".into() } // A11Y-4, MCP
}
```

- **Navigation**: `ctx.go(route)`, `ctx.go_with(route, Carry::rule("B3/S23", Route::RuleLab).with_rle(rle))`
  (the target's `enter` then `receive` run at the start of the next frame), `ctx.open_series(app)`.
  The header (brand, modes, app switcher) and the tab row navigate by themselves; "Playground"
  opens `ctx.last_bench()`.
- **Toasts / clipboard / files**: `ctx.toast(text)`; `ctx.copy(text)` (also kept as
  `AutomataApp::last_copied()` for tests); `ctx.save_text("glider-garden.rle", &rle)` → native
  save dialog in the app, `out_dir` in tests (`AutomataApp::saved_files()`), `Ok(None)` when
  cancelled.
- **Time and randomness**: step the engine in `Screen::tick` (called after `enter` /
  `receive` / `command` and before the header, so every counter shows the same generation)
  with `ctx.dt()` + a `Ticker` per screen; `ctx.rng_seed()` (7 in tests); `ctx.autoplay()`.
- **Frame pacing** (never spin): nothing repaints unless asked. While running, a screen whose
  picture only changes on a step asks for the next frame when the step is due,
  `ctx.wake_in(ticker.due_in(rate))` (waits ≤ 0.1 s); motion that changes every frame (a
  transition, spools, bobbing jars) calls `ctx.animate()` each frame (display rate: vsync on
  screen, never sooner than 1/240 s, every 0.1 s while the window is hidden). `ctx.dt()` is the
  real time since the frame that asked for this one (≤ 0.1 s, or the wait + 20 ms), and
  egui's `stable_dt` after an idle stretch, so kittest frames stay 0.1 s. A paused screen
  asks for nothing (the Tray's callout pulse runs only while it runs).
- **Keyboard**: the app handles ⌘1–⌘4 and Tab order; screens get `Command::RunPause` /
  `Command::Step`; check `ctx.text_focus()` before treating other letters as shortcuts.
- **Layout**: `layout::columns` with the mockup's column specs (CSS `flex` + `max-width`,
  border-box) — at 1440 they fill the window height under the header, at 390 they stack at
  `route.panel_height()` (a `scroll` column grows to its content instead of scrolling, so
  scroll requests reach the page) and the page scrolls (heights match the 390 references: Tray 2759,
  Loom 1915, Terrarium 1875, Rule lab 2899). Benches put `layout::bench` in their main
  column; the Rule lab uses `Column::main(..).pad(16.0, 16.0).gap(14.0).scroll(true)`.
- **Accessibility**: use the widgets (they set roles, names, pressed / disabled, focus ring,
  44 px targets); give canvases a `describe()` and an AccessKit label; group regions with
  `UiBuilder::accessibility_role` and call `widgets::set_bounds(&ui, rect)` on them.
- **Tests**: `tests/common/mod.rs` builds the real app (`app().at(Route::Tray).phone().build()`),
  clicks by label / role (scrolling targets into view), reads `pressed` / `selected` /
  `disabled`, presses keys (`press`, `press_cmd`), steps (`step_until`), reads `toasts`, and
  renders (`render`, `render_full`, `screenshot`). `scripts/compare-mockup.sh` writes
  `[app | mockup]` PNGs to `target/compare/`.


### Screen control API

What the MCP bridge (wave 3) and tests call on the screens' states (`app.screens_mut().<name>`)
to drive them without the pointer. Each method acts on the real engine state; the next frame
draws the result.

#### Tray (`screens::tray::State`)

| method | |
|---|---|
| `rule() -> &LifeRule`, `set_rule(LifeRule)` | the rule (cells stay); cartridges / bits / name follow |
| `load_pattern(&Pattern) -> bool` | clears the tray and places the pattern centred (wrapping); patterns larger than 40 × 26 are trimmed, then cropped around their centre (returns `true` when cropped); counters reset |
| `set_running(bool)`, `running() -> bool` | Run / Pause (also the public `running` field) |
| `step_n(u32)` | advances n generations now, running or not |
| `clear()`, `restart()`, `randomize(seed: u64)` | Clear (rule kept), Restart (starting garden + Life), a 30 % soup from `seed` (rule kept); counters reset |
| `rle() -> String` | the whole 40 × 26 tray as RLE with `rule =` (what Share copies; positions survive a round trip through `load_pattern`) |
| `stats() -> TrayStats` | `{ generation, population, born, died, mood_title, mood_text }` |
| `grid() -> &Grid`, `history() -> &[usize]` | ages / trails; the sparkline's last ≤ 120 populations |
| `set_tool(Tool)`, `current_tool()`, `pick_stamp(i)`, `arm(name, &Pattern)` | Draw / Erase / Stamp, a built-in stamp, a pattern armed as the custom stamp (TRAY-9; the Finder `odoc` hook should call `arm`) |
| `set_speed(f32)`, `speed()`, `trails()`, `flat()`, `tilt()`, `cursor()` | look and feel (speed clamped to 1–30) |
| `description()` / `Screen::describe` | "Cell tray, 40 by 26, generation N, M alive" (A11Y-4) |
| `cell_center(x, y)`, `cell_rect(x, y)`, `cell_at(pos)` | screen geometry of the last frame drawn (through the perspective), for pointer tests |

`receive(Carry)` (NAV-4) parses the rule (Moore) and, with an RLE, loads it like
`load_pattern`; ⌘V (`Event::Paste`) and dropped `.rle` / `.cells` files arm the pattern as
the custom stamp.

#### Rule lab (`screens::rule_lab::State`)

| method | |
|---|---|
| `script_text() -> String` | the editor's text as shown (may have errors) |
| `set_script(&str) -> Result<(), Vec<ScriptError>>` | replaces the script: when it parses, the text becomes it and its spec takes effect (run synchronously); otherwise nothing changes and the line-numbered errors come back |
| `errors() -> &[ScriptError]` | the editor text's errors (empty when it parses) |
| `spec() -> RunSpec`, `script() -> &RuleScript` | the spec in effect: the last valid script's (with its name and probes) |
| `set_rule(LifeRule)`, `set_spec(RunSpec)` | what the spec controls do: re-run (synchronously) and rewrite the script's changed directive lines |
| `scrub(gen)`, `generation() -> usize`, `alive() -> usize` | the scrubbed generation (clamped to the run; pauses Play) and its population |
| `report() -> Report` | peak, final, first repeat, period, verdict (LAB-3) |
| `rle() -> String` | the scrubbed generation as RLE with `rule =`, untruncated (the panel shows the mockup's 420-character preview) |
| `populations() -> &[usize]`, `lab() -> &Lab` | the run's populations; the GUI-free model (`screens::rule_lab::model::Lab`) |
| `playing` (field), `Screen::describe` | Play / Pause; "Rule lab: B3/S23, Moore r1, wraps at edges; Top-down view of the grid, 64 by 40, generation 120, 148 alive; Settles into a period-2 cycle at gen 388." |

Script ⇄ spec sync (LAB-4): typing applies the text whenever it parses and leaves it exactly
as typed; while it doesn't parse, the errors are shown per line and the last valid spec stays
in effect. The spec controls (and `set_rule` / `set_spec`) rewrite only the directive lines
whose canonical form changed — for canonical text that is `RuleScript::to_text()`; comments,
spacing and other lines stay, and the changed directive's own line is replaced even if it held
an error. While the editor has keyboard focus and the text has errors, the text isn't touched;
the patch is applied when focus leaves. Specs above 4 M cell-generations typed into the
script run on a worker thread (the old run stays on screen; API calls wait for the result).

`receive(Carry)` (NAV-4) parses the rule (Moore) and re-runs. "Open in Playground" sends
`Carry { rule: <code>, rle: Some(rle()) }` to the Tray. Export: Copy RLE, Copy script, Save
`<stem>.rle`, Save `<stem>.rule` (`<stem>` from the script's name, `glider-garden`; the script
exported is the text when it parses, else the last valid script).

#### Loom (`screens::loom::State`)

| method | |
|---|---|
| `rule() -> u8`, `set_rule(u8)` | the rule number; setting it restarts the thread (LOOM-4) |
| `toggle_key(p)`, `previous_rule()`, `next_rule()`, `surprise() -> u8` | a key press (flips bit `p` = `left·4 + centre·2 + right`), −1 / +1 (wrapping 0 ↔ 255), Surprise me (another rule from the screen's generator, seeded by `ctx.rng_seed()`) |
| `start() -> WeaveStart`, `set_start(WeaveStart)`, `pick_first_row(FirstRow)` | how the thread starts (exactly that seed with `set_start`); the First row segment (Random draws a new seed per press) — all restart |
| `restart()` | New thread (a Random start draws a new row, like the mockup's `fresh(start)`) |
| `set_running(bool)`, `running() -> bool` | Weave / Pause (the spools turn only while weaving) |
| `set_speed(f32)`, `speed()` | rows per second, rounded and clamped to 2–40 |
| `step_n(u32)` | weaves n rows now (≤ 100 000 per call), running or not |
| `row_count() -> u64` | rows since the start, the first included (the `row 0000` counter) |
| `rows_text() -> String` | the visible rows (oldest first, ≤ 72), one line each, `█` alive / `·` empty |
| `share_text() -> String` | what Share copies: `Rule N` then `rows_text()` |
| `weave() -> &Weave`, `elementary() -> Elementary` | the engine thread and rule (binary, blurb, famous) |
| `description()` / `Screen::describe` | "Woven pattern, rule 30, 121 cells wide, 47 rows woven" (A11Y-4); `describe` adds "weaving / paused at N rows/s" |

The keys are AccessKit toggle buttons named like the mockup ("Pattern 110 makes a live cell",
`key_name(rule, p)`) in the group `KEYS_LABEL`. On stages narrower than 0.7 × 860 px (phones)
the board and head stay full size, cropped like the 390 reference, and the keys move into a
4 × 2 grid so every key stays on screen. Space toggles Weave / Pause; `S` pauses and weaves
one row.

#### Terrarium (`screens::terrarium::State`)

| method | |
|---|---|
| `jars() -> Vec<JarInfo>` | `JarInfo { code, name, population, liveliness, kept }` for the six jars (name = `rule_name` or "Mutant"; liveliness 0–1) |
| `keep(i, bool)`, `is_kept(i)` | Keep / Kept for jar `i` (0-based; out of range ignored); keep order decides the carried-over slots |
| `breed()` | "Breed brood N+1" (TERR-2): kept jars first (unchanged rules, fresh soup), the rest children of the parents; lineage entry; keeps cleared |
| `fresh_soup()` | re-seeds every jar, same rules |
| `set_mutation(u8)`, `mutation() -> u8` | bit flips per child, clamped to 1–4 |
| `set_running(bool)`, `running() -> bool` | Run / Pause (jars tick every 140 ms and bob only while running) |
| `tick_n(u32)` | ticks every jar n times now (≤ 10 000 per call) |
| `brood() -> u32`, `lineage() -> Vec<LineageEntry>` | the `brood N` counter; the last 8 broods' parents, newest first |
| `chosen_rule() -> String`, `keep_note() -> String` | the NAV-4 rule (first kept jar's, else jar 1's); "None kept: all six become parents" / "N kept as parents" |
| `share_text() -> String`, `terrarium() -> &Terrarium` | what Share copies (`Jar 1 · B3/S23 · Life` per line); the engine bench |
| `Screen::describe` | "Terrarium, brood 1, paused: B3/S23 Life (122 alive), …" |

Plant in the Tray / Analyse in Rule lab send `Carry { rule: chosen_rule(), rle: None, from:
Terrarium }` to the Tray / Rule lab. Each card is a group "Jar N" with the jar image ("Jar
running rule B3/S23, 122 cells alive"), the code and name labels, the "Keep jar N" toggle and a
`Meter` "Liveliness N%"; the jar reserves `JarScene::FOOTPRINT`, so the code is never under
it. The first `enter` seeds the jars from `ctx.rng_seed()` unless the API changed them first.

## MCP

`automata-mcp` (crate `ap-mcp`, `crates/ap-mcp/README.md`) follows
`../chem-playground/crates/chem-mcp`: a `lab_mcp::mcp::ToolRegistry` served on stdio
(`--socket`, `--help`, `--version`; logs on stderr). Engine tools run `ap-engine` in-process;
app tools forward to the running app over `lab_mcp::bridge`, one connection per call.

- **Socket**: `ap_shared::bridge_socket_path()` = `$AUTOMATA_MCP_SOCKET` (empty / `off`: no
  bridge), else `lab_mcp::bridge::default_socket_path(APP_ID)`
  (`~/Library/Application Support/dev.labseries.AutomataPlayground/mcp.sock`). Both the app
  (`AppOptions::default().mcp_socket`) and the server use it. `automata-playground --no-mcp`
  turns the app's bridge off; `AppOptions::test()` and the smoke run have none; tests pass a
  temp socket. A second instance (socket already owned) or a bad path logs "MCP bridge off" and
  runs without a bridge.
- **`ap-shared`** (no egui): `APP_NAME`, `APP_ID`, `SOCKET_ENV`, `HOW_TO_START`; `SCREENS` /
  `MODES` (ids `tray`, `loom`, `terrarium`, `rule-lab`, `series`; modes `playground`,
  `advanced`; `tests/mcp_bridge.rs` checks they equal `Route::ALL`); `proto` (the Automata
  bridge methods, their param structs — `deny_unknown_fields`, `show` defaulting to true — and
  result structs `TrayState`, `LoomState`, `TerrariumState`, `LabState`, plus `SpecInfo`,
  `ReportInfo`, `RuleInfo`, `NeighbourhoodInfo` built `From` engine types, `sample_series`);
  `input` (`pattern_input`: stamp id / name, RLE or plaintext; `rule_input`: rulestring or
  name; `stamp`, `named_rule`) so the app and the server accept the same text.
- **Engine tools**: `run_life`, `analyze_rule`, `elementary`, `parse_rule`, `parse_pattern`,
  `list_presets`, `rule_script` (limits: `run_life` ≤ 256 × 256, ≤ 10 000 gens, ≤ 2·10⁸
  cell-generations; `analyze_rule` the engine's 256 / 2000; `elementary` ≤ 65 536 cells).
- **App tools**: `app_status`, `list_screens` (offline, plus `current`), `navigate`,
  `screenshot`, `describe_screen`, and per screen `tray_get` / `tray_set_rule` /
  `tray_load_pattern` / `tray_control`, `loom_get` / `loom_set`, `terrarium_get` /
  `terrarium_keep` / `terrarium_breed`, `lab_get` / `lab_set_script` / `lab_scrub`, which
  forward their arguments unchanged as the params of `tray.get` … `lab.scrub`.
- **App side** (`control.rs`): `poll` runs first thing in `AutomataApp::frame` (a navigation or
  change shows in the same frame); `logic` (`App::logic`) queues requests while the window is
  hidden and sends `RequestPaintWhileHidden`; the bridge's waker repaints an idle app.
  Screenshots: `ViewportCommand::Screenshot`, PNG-encoded on a worker thread, after 2 settle
  frames when navigating. Describe: the AccessKit tree caught by an egui plugin at the end of a
  frame, flattened to `AccessNode`s, plus `summary` = the screen's `Screen::describe()`. Screen
  methods check their params first (bad ones change nothing: `INVALID_PARAMS` with a message
  for the model), then `prepare` the screen — `AutomataApp::enter_screen` runs its `enter` once
  so first-visit set-up (autoplay, the Terrarium's seeded soups) can't undo the change later —
  switch to it when `show`, call its control API (§ "Screen control API") and reply with its
  state.
- **Tests**: `crates/ap-mcp/tests/e2e.rs` spawns the real binary and drives the real app in
  egui_kittest on a temp socket; `crates/automata-playground/tests/mcp_bridge.rs` drives
  `control.rs` with a `BridgeClient`.

## Packaging and `test-all`

- `scripts/test-all.sh [--quick]` (`SKIP=fmt,lint,clippy,test,perf,bundle`): fmt, shellcheck,
  clippy `-D warnings`, `cargo test --workspace` (counted per crate; GUI = the app's integration
  tests, e2e = `tests/e2e/` or targets named `*scenario*` / `*e2e*`, MCP = `ap-mcp`'s tests),
  release perf gates, bundle. **Perf tests** live in a crate's `tests/perf.rs`, are
  `#[ignore]`d and print `<label>: <n> ms …` (ap-bench: `tray (…)` / `loom (…)` < 2 ms);
  ap-engine's `examples/lab_timing.rs` prints `worst: <n> ms per 400-generation run` (< 30 ms).
- `scripts/bundle-macos.sh [--dmg] [--check]` → `dist/Automata Playground.app`
  (`dev.labseries.AutomataPlayground`, Info.plist from `packaging/Info.plist.in`, icon
  `assets/icon/Automata Playground.icns` rendered from `automata-icon.svg` by
  `assets/icon/render-icon.mjs`). Built with **`AUTOMATA_BUNDLE=1`**: crates must not embed
  `env!("CARGO_MANIFEST_DIR")` when `option_env!("AUTOMATA_BUNDLE")` is set (the bundle may
  contain no path of the build machine). `automata-mcp` goes into `Contents/MacOS/` once a
  package has that `[[bin]]`.
- **Smoke contract** (`scripts/check-bundle.sh`, `test-all` start-up gate):
  `automata-playground --smoke <seconds>` runs the Tray for N seconds in a window (headless
  with `AUTOMATA_HEADLESS=1` or without a display), prints one line starting `SMOKE ` with at
  least `first_frame_ms=` and `frames=`, and exits 0; no persistence, no reads of the checkout.
  It draws only the frames the app asks for (window and headless), so `frames=` is the app's
  frame rate (Tray running: ~30 fps, the callout pulse). `check-bundle.sh` also opens
  `glider.rle` with `open -n -a <copy>` (odoc) and expects `opened pattern` in the app's log
  (SKIP without a window server).
