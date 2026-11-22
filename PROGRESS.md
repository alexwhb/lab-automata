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

(none)

## Candidates for labkit

(none yet)

## Open issues

(none)
