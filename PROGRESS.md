# Progress

## Status

| Wave | Work | Status |
|---|---|---|
| 0 | repo, spec, architecture, mockup + references, `ap-theme` | done (lead) |
| 1 | `ap-engine` · app shell + widgets + Series · `ap-bench` | done |
| 2 | Tray · Loom + Terrarium · Rule lab | done |
| 3 | MCP · platform polish · layout fixes + visual pass + e2e journeys + README · packaging · independent review + fixes | done |

## Decisions (lead, 2026-11-22)

- Location `~/Projects/Personal/lab-series/automata-playground/`, its own git repo, like
  Breadbox and Chem Playground. Shared code comes from labkit (by path until
  2026-11-22; since then a git dependency on github.com/alexwhb/labkit pinned to a commit).
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
     **Done (platform, `c9d2f3a`)**: `src/platform.rs` (odoc + command-line paths → one queue,
     drained each frame by the receiving app → `tray.open_file` → Tray, `opened pattern <path>`,
     parse errors toast); `check-bundle.sh` section 4 opens `glider.rle` with `open -n -a`
     (SKIP without a window server).

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
  **Done (platform, `2e07ef4`, `11cb793`, `a20175b`)**: it did spin (the real app on the Tray /
  Loom / Terrarium: 107–114 % CPU; the window smoke ~1 000 fps): eframe paints without vsync
  back-pressure when macOS doesn't composite the window, and doesn't paint (nor wait) when it
  knows it's hidden. Now `Ctx::wake_in` paces the Tray / Rule lab Play to their generations,
  `Ctx::animate` never asks sooner than 1/240 s (every 0.1 s while hidden), the Callout pulse
  runs only while the Tray runs, and `--smoke` draws only the frames the app asks for
  (`frames=94` in 3 s, ~31 fps: the pulse). Real app, not composited: Tray 12 %, Loom 26 %,
  Terrarium 31 %, paused / Series / Rule lab 0 %, any screen hidden (⌘H) 3 %.

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
  **Done (platform)**: as suggested, through `State::open_file(path)` (the dropped-file code).
- **tray → bench owner** (optional, look): side by side with `Tray-1440.png` the newborn /
  alive glows (`0 0 16px rgba(255,170,232,.75)`, `0 0 10px …`) read fainter than Chrome's —
  the mockup shows a soft pink haze around clusters. Compare
  `target/compare/Tray-1440-gen30.png` (`AP_COMPARE=1 cargo test -p automata-playground --test
  tray renders`).
  **Done (platform, `06cc612`)**: at the same state they match Chrome. The animated reference
  catches box-shadow / background transitions (.2 s) mid-flight, which is the extra haze.
  `crates/ap-bench/scripts/chrome-tray-gen30.mjs` renders the bundle at gen 30 with transitions
  finished; against it the halo around live cells is within 0.2 luminance of Chrome's. Measured
  Chrome's blur: σ 7.6 / 4.5 for `16px` / `10px` (Skia's three box blurs), now
  `ap_bench::draw::css_blur_sigma`. Matching the live run's haze would mean animating each
  cell's transitions (display-rate frames while running); not done.
- **rulelab → lead (widgets)**: the `advance_block`-based widgets (`BitRow`, `NbGrid`,
  `SegGroup`, `Callout`, `Card::label`) advance with zero spacing, so the parent's `gap` after
  them is lost and the next item touches them (Rule lab: "Born with" bits → "Survives with",
  Boundary well → "Seed", the cards in the spec aside). The Rule lab adds the gap back by hand
  (`rule_lab::gap()`, and a `scope_builder` group instead of `Card::label`). If you make those
  widgets keep the parent's spacing, drop those calls (or tell rulelab) to avoid double gaps.
  **Done (qa, 307ff9d)**: the block widgets end with `advance_cursor_after_rect`, so a
  column's gap works like CSS `gap` after them (test `block_widgets_keep_the_column_gap`);
  `rule_lab::gap()` is gone and the spec cards use `Card::label`. Tray / Loom / Terrarium /
  Series lost their compensations; renders were pixel-identical. The Tray aside's remaining
  2 px drift came from the sparkline (the svg's 1 px border: `Plot` heights are now the
  content box + 2) and stat tiles (CSS line boxes).
- **rulelab → lead (widgets, look)**: `MiniButton` grows to its wrapped label's height (30 px
  for "von Neumann"); CSS keeps `.mini` at `height: 28px` and lets the text overflow, so the
  Hexagonal / Moore r2 presets sit 2 px lower than in `RuleLab-1440.png`.
  **Done (platform, `11cb793`)**: always 28 px, the wrapped label overflows centred.
- **rulelab → ap-theme owner** (optional): there is no error colour token; the Rule lab's
  script errors use the series' warning red from `lab_theme::LAB_DARK` (`red` `#FF5D4F`,
  `red_bg` `#2A1715`). Add `ap_theme::ERROR` / `ERROR_BG` (re-exporting those) if you want it
  in the Automata palette; `screens/rule_lab/editor.rs` would switch to them.
  **Done (platform, `61e2190`)**: `ap_theme::ERROR` / `ERROR_BG`; the editor uses them.
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
  **Done (qa, 36eda9e, 9b29912)**: stacked, a scroll column has no `ScrollArea` of its own
  and its line grows to the content instead; `widgets::reveal_on_focus` scrolls a control that
  gains keyboard focus into view; `layout::bubble_scroll` passes the Terrarium jar stage's own
  scrolling on to the page. `phone_full` is gone (390 × 1000 windows); covered by
  `journeys::phone_focus_scrolls_the_page`.
- **loomterr → lead (tab row, look)**: at 390 the bench tab row is ≈ 13 px taller than in the
  references when the hint wraps (Loom-390: hint at y ≈ 303 vs 296, stage from ≈ 318 vs 305):
  the wrapped item gets a taller line than the 15 px hint. The Loom's key grid zooms to fit
  the stage, so nothing is clipped, just 13 px lower than the reference.
  **Done (qa, 36eda9e)**: slot items measured in a vertically centred frame, which egui
  stretches to the row (28 px); they now measure top-aligned and `flow` centres them.
- **loomterr → lead (FYI, harness)**: `common::child_labels` reads `accesskit_node().label()`,
  but egui puts a `Role::Label`'s text in `value()` (label is `None`), so it returns `""` for
  labels (buttons are fine). `tests/terrarium.rs` reads `value()` directly; consider
  `label().or(value())` in the helper.
  **Done (qa, 36eda9e)**: `common::text_of` (label or value); `child_labels` uses it and
  `tests/terrarium.rs` too.
- **loomterr (FYI, widgets)**: like rulelab, the Loom / Terrarium asides add the column gap
  back by hand after `advance_block` widgets (`SegGroup` → Speed, lineage rows). If those
  widgets start keeping the parent's spacing, drop the `add_space` calls in
  `screens/loom/aside.rs` and `screens/terrarium/aside.rs` (or tell loomterr).
  **Done (qa, 307ff9d)**: both `add_space` calls are gone (see the rulelab entry).
- **loomterr (decision, Terrarium)**: the jars bob only while running (the mockup's CSS bob
  never stops), so a paused Terrarium doesn't repaint every frame; the loom spools stop when
  paused as the mockup says. One line to change in `terrarium.rs::tick` if the lead prefers
  the always-on bob.
- **platform (decision, Tray)**: likewise the "what is happening" dot pulses only while the
  Tray runs (`Callout::pulse(self.running)` in `tray/panels.rs`); otherwise its 30 fps repaint
  kept a paused Tray drawing. Pacing API for screens: `ctx.wake_in(ticker.due_in(rate))` when
  only a step changes the picture, `ctx.animate()` while something moves every frame (Loom
  spools and Terrarium jars keep `animate`, as they move every frame).
- **platform → packaging / lead (FYI)**: `--smoke` now paces like the app, so `frames=` is a
  frame-rate check (Tray: ~90 in 3 s); `test-all` could gate it (e.g. `frames < 400`).

- **mcp → lead (FYI, hooks)**: the MCP wave touched the shell minimally: `app.rs` has a
  `control` field, `control::poll(self, ui)` as the first line of `frame()`, `App::logic` →
  `control::logic`, and `pub(crate) fn enter_screen(route, ctx)` (the bridge enters a screen
  once before driving it in the background, so its first-visit set-up can't undo the change);
  `AppOptions::default()` now listens on `ap_shared::bridge_socket_path()` (tests / smoke: none);
  `lib.rs` has `run_with(opts)`; `main.rs` takes `--no-mcp`. New crates: `ap-shared`, `ap-mcp`.

## Review fixes (`fixes`, 2026-11-22)

The independent review's verified findings, each with a regression test (ported from the
reviewer's repro, seen failing before the fix): `crates/automata-playground/tests/review_regressions.rs`,
`crates/ap-engine/tests/review_regressions.rs`, `crates/ap-bench/tests/review_regressions.rs`,
`crates/ap-mcp/tests/review_regressions.rs`.

- **1** Rule lab reads stale while a background run computes: `State::finish()` (public) before
  scrub / Open in Playground / Export / every `lab.*` bridge call, `Lab::set_script` waits; while
  a run computes the run header, tiles, verdict, RLE and `describe` say "running…".
- **2** Tray tap (press + release in one frame) drew nothing; press + move started at the
  pointer's end: the press runs on a same-frame click too, at the frame's press position.
- **3** Open in Playground with a non-Moore-r1 neighbourhood: refused with "The Tray runs Moore
  r1 rules; this rule uses …" (the carry is a rulestring the Tray reads as Moore r1).
- **4** Tray `rle()` → `load_pattern()` recentred: a pattern that fits keeps its box; bigger
  ones are trimmed, then cropped around their centre (Rule-lab frames as before).
- **5 / 6** `describe` mixed screens / answered for another screen: captures carry their route,
  a screen change under a waiting request settles again, a named `screen` is gone back to
  (≤ `MAX_RETURNS`).
- **7** Abandoned bridge requests were applied: `handle` drops them (`PendingRequest::is_abandoned`).
- **8** Rule-script tokenizer quadratic in whitespace runs: linear (100 k spaces: 2.1 s → < 1 ms).
- **9** Analyse in Rule lab into a big spec froze the frame: `receive` runs it in the background.
- **10** Superseded Rule-lab workers ran to the end: `ap_engine::analysis::run_cancellable`;
  dropping a `Pending` cancels it within a generation.
- **11** A leading BOM made a control change wipe the script: stripped / skipped.
- **12** `loom.set {seed, restart}` discarded the seed: no restart after `set_start`.
- **13** `LoomGeometry::fabric_cell_at` returned row == capacity on the bottom edge (one-line
  fix in `ap-bench`, platform's crate).
- Also: Tray `set_speed(NaN)` ignored; ⌘V pastes over 4 MB and stamps over 10 000 live cells
  refused with a toast; a NAV-4 carry survives a bridge `navigate` in the next frame (it was
  dropped by `apply`); `run_life` sizes a too-big box by its live cells; `tray_load_pattern`'s
  `maxLength` (now 170 000) fits the bridge's 1 MiB line after JSON escaping, and `AppLink::call`
  refuses bigger arguments clearly.
- `tests/mcp_bridge.rs::a_client_that_leaves_mid_screenshot_does_no_harm` now waits until the
  app holds the request before hanging up (hanging up first is finding 7's case).

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

- **mcp**: `automata_playground::control`'s `AccessCapture` egui plugin and `outline_nodes`
  (AccessKit tree → `lab_mcp::bridge::proto::AccessNode`s) are a verbatim copy of Chem
  Playground's; with the generic half of both `control.rs` files (status / navigate /
  screenshot with settle frames / describe, the hidden-window `logic`) they could move into
  `lab-mcp` behind an `egui` feature, parameterised by the app's route type.
  **fixes (review)**: if it moves, keep what `control.rs` learnt: the capture records the route
  it was armed on, waiting requests re-settle when the screen changes under them and honour a
  named `screen` (findings 5 / 6), and abandoned requests are dropped before they're applied.
- **fixes (review, `lab-mcp` bridge)**: `PendingRequest::is_abandoned` is set when the reader
  sees EOF, which a client that half-closes its write side after sending (e.g. `nc -N`) also
  causes; such a client now gets no reply from Automata. An exact "peer gone" check (the write
  side failing / `POLLHUP`) would tell the two apart. The 1 MiB `MAX_REQUEST_BYTES` is per JSON
  line after escaping; servers must size their schemas' `maxLength` for it (Automata:
  `TRAY_PATTERN_MAX_LEN`).

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

## After the build (2026-11-22)

- The header's app switcher is gone while the apps don't switch to each other (NAV-3). The
  Series screen stays (MCP `navigate`, `App::open_series`); mockup comparisons cut the switcher
  out of the references (`tests/common`: `desktop_reference`, `phone_reference`).
- Published as github.com/alexwhb/lab-automata (MIT). The lab-series repos share a `lab-` prefix:
  `lab-breadbox` (renamed from `breadbox`), `lab-automata`, later `lab-chem` / `lab-bio`, and
  `labkit` (now public too, pinned here by git commit). Local folder names are unchanged.
