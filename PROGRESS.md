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

  **packaging (2026-11-22)**: verified — `scripts/bundle-macos.sh --check` passes (sandboxed
  headless + window smoke runs, `open` launch, no machine paths; MCP SKIP until `ap-mcp`), and
  every `test-all` perf gate passes (start-up 455 ms). Two things for the lead:
  (a) `cargo fmt --check` fails on `crates/ap-theme/src/lib.rs:448` (the `shade(CART_BODIES[0],
  0.45)` assert wants wrapping), the only red step in `scripts/test-all.sh` right now;
  (b) the windowed smoke run is not frame-capped: `mode=window … frames=21295
  frame_ms_avg=0.04` in 3 s (~7 000 fps; headless does 146). Fine for a smoke test, but if the
  real app repaints the same way while the Tray runs it would spin a core / the GPU — worth
  checking that it paces to vsync or to the generation speed.

- **tray → lead** (`tests/a11y.rs`, red because of the real Tray): `tab_key_walks_in_reading_order`
  expects the bench tabs right after Share, which held while the asides were placeholders.
  Now Tab follows the mockup's DOM order (NAV-5 "reading order"): header → the rules aside
  (Life, HighLife, Seeds, Day & Night, Maze, Coral, then the stamps Glider, Spaceship, …) →
  bench tabs → Draw / Erase / Stamp / Random soup / Clear → the tray grid → the readouts.
  Suggested fix: expect `…, "Share", "Life", "HighLife", "Seeds", "Day & Night", "Maze",
  "Coral", "Glider", "Spaceship"` for the first 19 presses (what the app does now), or press
  Tab 33 times and check the tabs and tool items follow the 14 aside buttons. Everything else
  in the app suite is green with the Tray (`cargo test -p automata-playground`).
- **tray → lead (FYI)**: `dae8c7b` added `ap-engine` and `ap-bench` to the app crate's
  `[dependencies]` (all three wave-2 screens need them; two lines + `Cargo.lock`).
- **tray → shell / packaging** (Finder `odoc`, request 3 above): arm the opened file with
  `app.screens_mut().tray.arm(name, &pattern)` (parse with `ap_engine::pattern::parse_file`)
  and navigate to the Tray; `Screen::receive(Carry { rle })` instead *loads* the pattern on a
  cleared tray (the NAV-4 semantics), which is not what TRAY-9 asks for a dropped / opened file.
- **tray → bench owner** (optional, look): side by side with `Tray-1440.png` the newborn /
  alive glows (`0 0 16px rgba(255,170,232,.75)`, `0 0 10px …`) read fainter than Chrome's —
  the mockup shows a soft pink haze around clusters. Compare
  `target/compare/Tray-1440-gen30.png` (`AP_COMPARE=1 cargo test -p automata-playground --test
  tray renders`).
- **rulelab → lead (widgets)**: the `advance_block`-based widgets (`BitRow`, `NbGrid`,
  `SegGroup`, `Callout`, `Card::label`) advance with zero spacing, so the parent's `gap` after
  them is lost and the next item touches them (Rule lab: "Born with" bits → "Survives with",
  Boundary well → "Seed", the cards in the spec aside). The Rule lab adds the gap back by hand
  (`rule_lab::gap()`, and a `scope_builder` group instead of `Card::label`). If you make those
  widgets keep the parent's spacing, drop those calls (or tell rulelab) to avoid double gaps.
- **rulelab → lead (widgets, look)**: `MiniButton` grows to its wrapped label's height (30 px
  for "von Neumann"); CSS keeps `.mini` at `height: 28px` and lets the text overflow, so the
  Hexagonal / Moore r2 presets sit 2 px lower than in `RuleLab-1440.png`.
- **rulelab → ap-theme owner** (optional): there is no error colour token; the Rule lab's
  script errors use the series' warning red from `lab_theme::LAB_DARK` (`red` `#FF5D4F`,
  `red_bg` `#2A1715`). Add `ap_theme::ERROR` / `ERROR_BG` (re-exporting those) if you want it
  in the Automata palette; `screens/rule_lab/editor.rs` would switch to them.
- **rulelab → tray (FYI, NAV-4)**: Rule lab "Open in Playground" sends `Carry { rule: <code>,
  rle: Some(<scrubbed generation, full RLE with rule =>), from: RuleLab }` (asserted in
  `tests/rule_lab.rs` through `tray.received`; please keep that field or tell me what to read).

- **loomterr → lead (layout, a11y)**: at 390 (stacked columns) a column with `scroll: true`
  swallows scroll requests: egui's `ScrollArea` always takes the pending `scroll_target`
  (`scroll_area.rs`: "we always take both scroll targets … to avoid them leaking"), so
  `scroll_to_rect` / keyboard focus / AccessKit `ScrollIntoView` on a widget in an aside never
  scrolls the *page* to it (e.g. Tab to "Surprise me" or "Breed brood 2" on a phone: the
  focus ring is off screen; `common::reveal` can't bring those buttons into view either).
  Suggested fix in `layout::columns`: when the columns stack, give scroll columns no own
  `ScrollArea` while their content fits the panel, or re-issue `ui.scroll_to_rect(rect)` on
  the page for the focused widget after the body is drawn. Until then `tests/{loom,terrarium}.rs`
  click at 390 in a window as tall as the page (`phone_full`).
- **loomterr → lead (tab row, look)**: at 390 the bench tab row is ≈ 13 px taller than in the
  references when the hint wraps (Loom-390: hint at y ≈ 303 vs 296, stage from ≈ 318 vs 305):
  the wrapped item gets a taller line than the 15 px hint. The Loom's key grid zooms to fit
  the stage, so nothing is clipped, just 13 px lower than the reference.
- **loomterr → lead (FYI, harness)**: `common::child_labels` reads `accesskit_node().label()`,
  but egui puts a `Role::Label`'s text in `value()` (label is `None`), so it returns `""` for
  labels (buttons are fine). `tests/terrarium.rs` reads `value()` directly; consider
  `label().or(value())` in the helper.
- **loomterr (FYI, widgets)**: like rulelab, the Loom / Terrarium asides add the column gap
  back by hand after `advance_block` widgets (`SegGroup` → Speed, lineage rows). If those
  widgets start keeping the parent's spacing, drop the `add_space` calls in
  `screens/loom/aside.rs` and `screens/terrarium/aside.rs` (or tell loomterr).
- **loomterr (decision, Terrarium)**: the jars bob only while running (the mockup's CSS bob
  never stops), so a paused Terrarium doesn't repaint every frame; the loom spools stop when
  paused as the mockup says. One line to change in `terrarium.rs::tick` if the lead prefers
  the always-on bob.

## Candidates for labkit

- **shell**: `automata_playground::widgets::flow` (CSS flex-wrap rows with `margin-left:
  auto` and remembered sizes + `request_discard`), `widgets::hit::tile` (44 px targets that tile
  a dense group without overlaps), `widgets::text::css_line_height` / `paint_line` (Chrome's
  `line-height: normal` box), `widgets::set_bounds` (AccessKit bounds for container `Ui`s,
  which egui 0.37 leaves unset), `widgets::icons::parse_path` (SVG `M L H V C S A Z` → egui
  shapes, with ear-clipped concave fills), `layout::resolve` (lab-ui flex lines + `max-width`).
- **rulelab**: `automata_playground::widgets::menu` (a popup menu under a button: AccessKit
  `Menu` / `MenuItem`, `has_popup` / `expanded` on the button, ↑ / ↓ / Escape / click-outside),
  and the script ⇄ spec line patching in `screens/rule_lab/model.rs` (`patch_text`) for other
  netlist editors (Breadbox's SPICE workbench).

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
