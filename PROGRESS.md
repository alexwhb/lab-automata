# Progress

## Status

| Wave | Work | Status |
|---|---|---|
| 0 | repo, spec, architecture, mockup + references, `ap-theme` | done (lead) |
| 1 | `ap-engine` · app shell + widgets + Series · `ap-bench` | done |
| 2 | Tray · Loom + Terrarium · Rule lab | in progress |
| 3 | MCP · end-to-end + visual pass at 1440 / 390 · bundle, README, test-all | not started |

## Decisions (lead, 2026-11-22)

- Location `~/Projects/Personal/lab-series/automata-playground/`, its own git repo, like
  Breadbox and Chem Playground. Shared code comes from `../labkit` by path.
- Stack as the siblings: eframe/egui 0.37 (wgpu, AccessKit), egui_kittest, edition 2024.
- labkit is used for fonts, the egui style builder and contrast helpers (`lab-theme`), layout
  maths and graph-paper painting (`lab-ui`), and MCP (`lab-mcp`). `lab-ui`'s widgets are drawn
  to the Chem mockup (40 px buttons, light mode pill); Automata's mockup uses the Breadbox
  look (32 px buttons, dark segmented pills), so Automata draws its own widgets in
  `crates/automata-playground/src/widgets`. They are candidates for labkit once Breadbox moves
  onto it.
- Mockup bugs not to copy: the Terrarium jars overlap the rule code under them.

## Ownership (wave 2)

| Agent | Owns |
|---|---|
| `tray` | `crates/automata-playground/src/screens/tray.rs` (+ `src/screens/tray/**`), `crates/automata-playground/tests/tray.rs` |
| `rulelab` | `crates/automata-playground/src/screens/rule_lab.rs` (+ `src/screens/rule_lab/**`), `crates/automata-playground/tests/rule_lab.rs` |
| `loomterr` | `crates/automata-playground/src/screens/{loom,terrarium}.rs` (+ `src/screens/{loom,terrarium}/**`), `crates/automata-playground/tests/{loom,terrarium}.rs` |
| `packaging` | `scripts/{test-all,bundle-macos,check-bundle}.sh`, `assets/icon/**`, `packaging/**` |
| lead | everything else in the app crate (app.rs, widgets, layout, router, Series, harness, other tests), `ap-engine`, `ap-bench`, `ap-theme`, docs |

Wave-2 agents may ADD new files under `src/widgets/` for widgets only their screen needs (and
register them in `src/widgets/mod.rs` with a one-line `pub mod`); changes to existing widgets,
the harness, `app.rs`, `ap-engine` or `ap-bench` go under Requests (the lead handles them fast).

## Ownership (wave 1, finished)

| Agent | Owns |
|---|---|
| `engine` | `crates/ap-engine/**`, `scripts/mockup-goldens.mjs` |
| `shell` | `crates/automata-playground/**` (except `src/screens/{tray,loom,terrarium,rule_lab}.rs`, which are placeholders it creates and wave 2 takes over), `scripts/compare-mockup.sh` |
| `bench` | `crates/ap-bench/**` |
| lead | everything else: `SPEC.md`, `PROGRESS.md` status, `docs/ARCHITECTURE.md` (agents update their own contract section), root `Cargo.toml` |

`crates/ap-theme/**`: in wave 1 the `shell` agent may **add** tokens (with their CSS source); `bench` asks for new colours under Requests (or uses existing tokens).

## Requests

- **bench → ap-theme owner**: `JAR_INKS` is `CART_BODIES`, but the Terrarium mockup's inks are
  in a different order: `['#F06BD0', '#F3DC6B', '#B79CFF', '#4BE3A0', '#FF8E7A', '#6CCBF5']`
  (jar 3 is lilac, jar 5 coral). `ap-bench` takes the ink as input, so only the token needs
  fixing. **Done (lead)**: `JAR_INKS` now has the mockup order (uncommitted in
  `crates/ap-theme/src/lib.rs` alongside the shell agent's token additions; shell commits it).
- **bench → ap-theme owner** (optional): `crates/ap-bench/src/tokens.rs` has a "local tokens"
  block of bench-only values with their CSS sources (floor band, tray / jar shadows, jar slab
  and glint, loom board ring / edge / fade / head drop / head shadow / track, key edge / drop /
  pattern stroke / empty / output glow, well border, focus). Move them into `ap-theme` if
  screens need them too.
- **packaging → app owner (`shell`)**: three things `scripts/check-bundle.sh` and
  `scripts/test-all.sh` already call (SKIP or FAIL until they exist):
  1. **`automata-playground --smoke <seconds>`** — a start-up + run check, mirroring
     `../breadbox/crates/breadbox/src/smoke.rs`. Set a `STARTED: OnceLock<Instant>` first thing
     in `main`; open the real app in a window (no persistence: `persist_window = false`, a temp
     `persistence_path`, deterministic seed) on the **Tray with autoplay running**, draw every
     frame for N wall seconds, then print exactly one stdout line starting with `SMOKE ` and
     exit 0 (non-zero on an error or if the window closes early), e.g.
     `SMOKE route=tray mode=window first_frame_ms=412 frames=180 gens=95 frame_ms_avg=1.9 frame_ms_max=7.2 status=ok`.
     `first_frame_ms=` (process start → first frame; test-all gates it < 2000) and `frames=` are
     required. With `AUTOMATA_HEADLESS=1` (or `--headless`), or when no window can be opened,
     run headless as Breadbox does (`egui::Context::default()`, `run_ui` at 1440 × 900 at
     ~60 Hz + `tessellate`, `mode=headless`). `--smoke 3` must exit within 15 s. check-bundle
     runs it from a bundle copy under `sandbox-exec` with the whole lab-series folder denied, so
     nothing may read the checkout, `../labkit` or `docs/` at run time, and it detects support
     by the string `--smoke` in the binary.
  2. **`AUTOMATA_BUNDLE`** — `env!("CARGO_MANIFEST_DIR")` in `src/launcher.rs`
     (`lab_series_roots`) and `src/router.rs` (`docs/design`) puts this checkout's absolute path
     into the release binary (check-bundle: "automata-playground contains paths of this
     machine"); `--remap-path-prefix` does not cover `env!`. `scripts/bundle-macos.sh` builds
     with `AUTOMATA_BUNDLE=1`; gate those like `../breadbox/crates/bb-core/src/paths.rs`:
     `const MANIFEST_DIR: Option<&str> = match option_env!("AUTOMATA_BUNDLE") { Some(_) => None, None => Some(env!("CARGO_MANIFEST_DIR")) };`
     (any crate, any non-test use).
  3. (optional) **Opening `.rle` / `.cells` from Finder** — the Info.plist registers both
     (Open With lists the app); they arrive as the Apple Event `odoc` (Breadbox:
     `platform.rs::install_open_documents()`) and should arm the pattern as a stamp (TRAY-9).
     Log `opened pattern <path>` at info level and check-bundle can test it like Breadbox.

  For the future `ap-mcp` owner: the bundle picks up `Contents/MacOS/automata-mcp`
  automatically once a package has a `[[bin]] name = "automata-mcp"`.

  **shell (2026-11-22)**: 1 and 2 done in `7200a64` (`--smoke <seconds> [--headless]`,
  `AUTOMATA_HEADLESS=1`, `SMOKE route=tray mode=… first_frame_ms=… frames=… gens=… …`;
  `automata_playground::MANIFEST_DIR` is `None` under `AUTOMATA_BUNDLE`). The `JAR_INKS` fix is
  committed in the same commit. 3 (Finder `odoc`) is not done: it needs the Tray's stamp
  arming (wave 2); `Screen::receive` with a `Carry { rle }` is the hook to deliver it.

## Candidates for labkit

- **shell**: `automata_playground::widgets::flow` (CSS flex-wrap rows with `margin-left:
  auto` and remembered sizes + `request_discard`), `widgets::hit::tile` (44 px targets that tile
  a dense group without overlaps), `widgets::text::css_line_height` / `paint_line` (Chrome's
  `line-height: normal` box), `widgets::set_bounds` (AccessKit bounds for container `Ui`s,
  which egui 0.37 leaves unset), `widgets::icons::parse_path` (SVG `M L H V C S A Z` → egui
  shapes, with ear-clipped concave fills), `layout::resolve` (lab-ui flex lines + `max-width`).

## Open issues

- **shell → lead (design decision)**: the Series mockup has no Open buttons; NAV-3 needs them.
  The sibling cards' tag pill *is* the Open button ("Open"; Bio's reads "Planned", disabled),
  so the cards keep the mockup's size and rows. Automata keeps its "new" tag. Bio's tag in the
  mockup says "existing", which contradicts "planned".
- **shell**: the Series mockup is the bare page; the app draws the header on top of it
  (NAV-1), so the Series board renders at 1440 × 697 in `target/compare/`.
- **shell**: placeholders (`tray.rs`, `loom.rs`, `terrarium.rs`, `rule_lab.rs`) are ready for
  wave 2 — see `docs/ARCHITECTURE.md` › "How a screen plugs in". Their header / tab-row
  actions already match the mockup's positions at 1440 and 390.
