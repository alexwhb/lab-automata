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
engine.

- `Tilt` / `Projection`: CSS `perspective(P) rotateX(θ) scale(s)` about a transform origin, for
  a plane of a given size placed at a screen rect: `project(local) -> screen`,
  `unproject(screen) -> Option<local>` (ray-plane), and helpers to draw projected quads,
  rounded rects, lines and text (glyph meshes transformed vertex by vertex).
- `floor(painter, rect, …)` — the `.floor` grid plane; `stage(painter, rect)` — `.graph`.
- `TrayScene { cols, rows, cell(col,row) -> CellVisual { Empty, Trail(1..=3), Live(age) },
  tilt (animated 0 ↔ 1 between tilted and flat), hover, cursor, labels }` → paints and returns
  the hovered cell (through the projection) — the app turns that into edits.
- `LoomScene` (fabric rows, newest row, progress, rule label, spool angle, keys' states) and
  the key caps' drawing; `JarScene` (32 × 32 cells, ink, bob phase); `TopDownGrid` (Rule lab:
  born / alive / ghost cells on the `.well`).
- Snapshot tests render each scene to PNG with `egui_kittest`.

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
