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

The engine agent owns the exact names; this is the minimum surface the screens rely on. The
rustdoc is the reference once it exists (`cargo doc -p ap-engine --open`). Keep this section in
sync when you change a public API.

- **Rules**: `Neighbourhood` (offsets within 5 × 5, never (0,0); presets `Moore` "Moore r1"
  key `moore`, `VonNeumann` "von Neumann" `vn`, `Hexagonal` "Hexagonal" `hex`, `Moore2`
  "Moore r2" `moore2`; `preset()` recognises a custom set equal to a preset), `LifeRule`
  (birth / survive count sets, neighbourhood; `parse`, `code()` canonical string, toggle
  helpers, `with_neighbourhood` drops counts above N), `Boundary { Torus, Dead }`, the named
  presets (the six Tray cartridges with name + note; the Terrarium names map incl. "2×2" and
  "Move").
- **Grid**: `w × h` cells with `age: u16` (0 = dead, 1 = newborn, saturating) and `trail: u8`
  (0–3); `step(&rule, boundary) -> StepStats { born, died, population }` implementing exactly
  the Tray mockup's `advance()` (new age = old + 1 on survival; trail = 3 on death, else
  max(0, t − 1); trail 0 while alive); set / toggle / clear; `place(&Pattern, x, y, anchor)`
  wrapping on torus, clipping on dead; population; bounding box; cheap clone.
- **Patterns**: `Pattern` (w, h, live cells); `library` with the eight stamps (`id`, display
  name, cells — the mockup's `P` table); `rle::parse` / `rle::write`; `cells::parse`
  (plaintext). `Pattern::from_grid_region`.
- **Elementary**: `Elementary` rule u8; `next_row(&[bool]) -> Vec<bool>` (wraps); `Weave`
  state (width 121, keep last 72 rows, total row count, start Single / Random(seed)); the six
  famous threads (number, name, note).
- **Analysis**: `RunSpec { rule, boundary, w, h, seed: Seed, gens }`, `Seed { RPentomino,
  Acorn, Soup { seed, density } }` placed as the Rule lab mockup does (R-pentomino at (31,19),
  acorn at (29,19), soup = the mockup LCG in f64 inside x∈(16,48), y∈(8,32));
  `run(&spec) -> Run` with every frame (bitsets), populations, `first_repeat: Option<Repeat {
  start, period }>`; `Report { peak, peak_gen, final_pop, first_repeat_gen, period, verdict }`.
- **Mood**: `Mood::classify(&history) -> Mood` with `title()` / `text()` (Tray mockup logic).
- **Breeding**: `Rng` (seeded, deterministic), `Jar` (32 × 32 torus, rule, soup, activity
  window of 12, `tick()`, `liveliness()`, `population()`), `breed(parents, mutation, kept, rng)`
  per TERR-2, `Lineage`.
- **Script**: `RuleScript { name, rule, neighbourhood key, w, h, boundary, seed, gens, probes }`
  with `parse(&str) -> Result<RuleScript, Vec<ScriptError { line, message }>>` and
  `to_text()` (exact mockup format).

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
