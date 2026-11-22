# Automata Playground

Cellular-automata playground + rule lab for macOS, written in Rust. The fourth bench in the lab
series (Breadbox → Chem Playground → Bio Playground → Automata Playground). Playground mode
(Tray · 2D Life-like rules, Loom · 1D elementary rules, Terrarium · breed rules) and Advanced
mode (Rule lab: spec, scrub, analyse, netlist-style script, export).

**Read before working:**

| | |
|---|---|
| [`SPEC.md`](SPEC.md) | the definition of done — every requirement has an ID |
| [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) | crate map and contracts between crates |
| [`docs/design/`](docs/design) | the mockup: `mockup/*.dc.html` (spec), `screenshots/*-1440.png` / `*-390.png` (rendered references), `bundles/*.html` (runnable in Chrome) |
| [`PROGRESS.md`](PROGRESS.md) | who owns what, status, open issues, requests |
| [`../labkit`](../labkit) | shared lab-series crates (`lab-theme`, `lab-ui`, `lab-mcp`) — use them, never edit them |
| [`../breadbox`](../breadbox), [`../chem-playground`](../chem-playground) | sister apps — read for patterns (egui_kittest harness, canvases, MCP); never edit them |

## Rules

- **Toolchain**: `export PATH="$HOME/.cargo/bin:$PATH"` (Homebrew's older rustc must not be used).
- **Parallel agents share this checkout.** Use your own target dir
  (`export CARGO_TARGET_DIR=target/<your-agent-name>`), build/test with `-p <crate>` for what
  you own, and never edit files owned by another active agent (see `PROGRESS.md`). Need a
  change elsewhere? Write it under "Requests" in `PROGRESS.md`.
- **labkit and the sister apps are other people's live work**: another team is editing labkit
  right now. Never edit anything outside this repo. If labkit fails to compile, it's probably
  mid-edit: wait a minute and retry. Anything that should move into labkit goes under
  "Candidates for labkit" in `PROGRESS.md`.
- **Commits**: commit only your own paths (`git add <paths> && git commit -m "..." -- <paths>`);
  retry if `.git/index.lock` is busy. Small, frequent commits with clear messages. End commit
  messages with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- Root `Cargo.toml`: adding your crate to `members` or a dependency is allowed for anyone as a
  one-line change; re-read the file just before editing.
- Keep `docs/ARCHITECTURE.md` in sync when you change a cross-crate contract.
- No panics on user input; libraries use `thiserror`, the app `anyhow`.
- Colours, fonts, radii and sizes come from `ap_theme` (which builds on `lab_theme`) — never
  hard-code a hex in a screen. Missing token? Add it to `ap-theme` with its mockup CSS source
  (ask the lead via Requests if you don't own it).
- Every feature lands with tests. Real behaviour beats mocks: drive the GUI with `egui_kittest`
  through AccessKit labels, run the real engine, spawn the real MCP server.
- **Every number shown comes from the engine** (SPEC DONE-2). No mockup numbers in screens.
- **No sparkle / magic-wand / four-point-star icons, ever** (no ✨ ✦ ✧). Pick an icon that says
  what the feature does.
- `cargo fmt --check` (not `--all`: that also checks `../labkit`), `cargo clippy --workspace --all-targets -- -D warnings` and
  `cargo test --workspace` must stay green (`scripts/test-all.sh` once it exists). Tests must
  not take more than a few seconds each in the default (opt-level 2) test profile.
