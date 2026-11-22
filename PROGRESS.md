# Progress

## Status

| Wave | Work | Status |
|---|---|---|
| 0 | repo, spec, architecture, mockup + references, `ap-theme` | done (lead) |
| 1 | `ap-engine` · app shell + widgets + Series · `ap-bench` | in progress |
| 2 | Tray · Loom + Terrarium · Rule lab | not started |
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

## Ownership (wave 1)

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
  fixing.
- **bench → ap-theme owner** (optional): `crates/ap-bench/src/tokens.rs` has a "local tokens"
  block of bench-only values with their CSS sources (floor band, tray / jar shadows, jar slab
  and glint, loom board ring / edge / fade / head drop / head shadow / track, key edge / drop /
  pattern stroke / empty / output glow, well border, focus). Move them into `ap-theme` if
  screens need them too.

## Candidates for labkit

(none yet)

## Open issues

(none)
