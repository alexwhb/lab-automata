# automata-mcp

The Automata Playground MCP server. AI agents (Claude Code, Claude Desktop, any MCP client) use
it to run cellular automata headless and to look at and drive the running app over the
[Model Context Protocol](https://modelcontextprotocol.io).

`automata-mcp` (crate `ap-mcp`) is a separate binary that speaks MCP on stdin/stdout. The
**engine tools** run `ap-engine` in-process, so they work without the app. The **app tools**
act on the running app: `automata-mcp` forwards them over the live bridge, a user-only Unix
socket the app listens on (`crates/automata-playground/src/control.rs`).

```
MCP client ──stdio──▶ automata-mcp ──Unix socket (live bridge)──▶ Automata Playground (control.rs)
                          │                                          └─ screens' control APIs
                          └─ engine tools: ap-engine in-process (no app needed)
```

## Build

```sh
export PATH="$HOME/.cargo/bin:$PATH"
cargo build --release -p ap-mcp            # → target/release/automata-mcp
target/release/automata-mcp --help
```

`scripts/bundle-macos.sh` also puts it in the app bundle:
`dist/Automata Playground.app/Contents/MacOS/automata-mcp`.

## Add it to Claude Code

```sh
claude mcp add automata-playground -- /path/to/automata-mcp
# for every project:
claude mcp add --scope user automata-playground -- /path/to/automata-mcp
```

Then open Automata Playground and ask Claude to, say, "put a Gosper gun on the tray under
HighLife and step it 60 generations", or, with no app at all, "what does B36/S125 do to an
R-pentomino?".

## Add it to Claude Desktop

Edit `~/Library/Application Support/Claude/claude_desktop_config.json` and restart Claude
Desktop:

```json
{
  "mcpServers": {
    "automata-playground": {
      "command": "/Applications/Automata Playground.app/Contents/MacOS/automata-mcp",
      "args": []
    }
  }
}
```

## Options

| | |
|---|---|
| `--socket <path>` | The app's bridge socket. Default: `$AUTOMATA_MCP_SOCKET`, else `~/Library/Application Support/dev.labseries.AutomataPlayground/mcp.sock` (a shorter path under the temp dir if that one is too long for a Unix socket). |
| `-V`, `--version` | Print the version. |
| `-h`, `--help` | Print help, including the default socket path. |

The app listens on the same default socket unless `AUTOMATA_MCP_SOCKET` is empty or `off`, or
it is started with `--no-mcp`; set the variable to a path to run an instance on another socket
(both sides read it). Logs go to stderr (`RUST_LOG=ap_mcp=debug,lab_mcp=debug` for more).

## Tools

Results are a one-line summary (first text block), then the same data as JSON
(`structuredContent` and a text copy). Mistakes (a rule that doesn't parse, a pattern that
doesn't fit, a jar that doesn't exist, a script with errors) are tool errors (`isError: true`)
whose message says what to send instead; nothing changes in the app when a call fails.

Rules are written `B3/S23` (also `b3s23`, `23/3`, `B3,10/S2,3`) or by name (Life, HighLife,
Seeds, Day & Night, Maze, Coral, 2×2, Move). Patterns are RLE, plaintext rows of `.` and `O`,
or a stamp id (`glider`, `lwss`, `pulsar`, `penta`, `rpent`, `acorn`, `gun`, `rep`).

### Engine tools (no app needed)

| tool | input | what it does |
|---|---|---|
| `run_life` | `pattern`, `rule`?, `neighbourhood`?, `width`?/`height`? (64 × 40, grown to fit; ≤ 256), `boundary`? (`torus`/`dead`), `gens`? (100; ≤ 10 000), `samples`?, `render`? | Runs the rule on the centred pattern: population (initial, final, peak @ gen, min), a sampled series, born / died totals and last step, extinction, the first exact repeat, spaceship motion (dx, dy per period), a verdict, the final RLE and an optional ASCII render. width × height × gens ≤ 2·10⁸ (about a second). |
| `analyze_rule` | `rule`?, `neighbourhood`? (preset or `[[dx,dy],…]`), `boundary`?, `seed`? (`r_pentomino`/`acorn`/`soup` + `soup_seed`, `density`), `width`?, `height`?, `gens`? (400; ≤ 2000), `samples`?, `generation`? | The Rule lab's analysis: peak @ gen, final, first repeat, period, verdict ("Settles into a period-2 cycle at gen 388." for the defaults), a series, the RLE at `generation` and the equivalent rule script. |
| `elementary` | `rule` (0–255), `width`? (121), `rows`? (32), `start`? (`single`/`random`), `seed`? | An elementary CA as the Loom weaves it: rows as `#`/`.` text, live cells per row, binary, blurb. width × rows ≤ 65 536. |
| `parse_rule` | `rule`, `neighbourhood`? | Canonical code, birth / survival counts, name, matching Tray cartridge. |
| `parse_pattern` | `pattern` | Kind (stamp / RLE / plaintext), name, author, comments, header rule, size, population, canonical RLE, rows (≤ 64 × 64). |
| `list_presets` | — | Cartridges, stamps (with RLE), famous threads, named rules, Terrarium start, neighbourhoods, seeds, boundaries. |
| `rule_script` | `script`, `analyze`? (true), `samples`? | Validates a Rule-lab script: `valid: false` with every error and its line, or the canonical text, whether it changed, the spec and the report. |

### App tools (the app must be running)

| tool | input | what it does |
|---|---|---|
| `app_status` | — | Running? Screen (id, label, mode), window size, version, pid. Not running is a normal result (`running: false`). |
| `list_screens` | — | The five screens (id, label, mode, shortcut, what's on it, mockup, board) and two modes; `current` when the app is up. Works offline. |
| `navigate` | `screen` | Switches screen: `tray`, `loom`, `terrarium`, `rule-lab`, `series`. |
| `screenshot` | `screen`? | A PNG of the window (physical pixels), after navigating if asked. |
| `describe_screen` | `screen`? | The screen's one-line summary and its AccessKit outline (`Role "name" = "value" [states]`, indented). Cheaper than a screenshot. |
| `tray_get` | — | Rule and name, generation, population, born, died, mood, run state, speed, history, RLE. |
| `tray_set_rule` | `rule`, `show`? | Sets the rule; the cells stay. |
| `tray_load_pattern` | `pattern`, `rule`?, `show`? | Clears the tray and places the pattern centred (cropped to 40 × 26 if bigger); a `rule =` header applies unless `rule` is given. |
| `tray_control` | `action` (`run`/`pause`/`step`/`clear`/`restart`/`randomize`), `n`?, `seed`?, `speed`?, `show`? | The Tray's buttons; `step` pauses then advances `n` (≤ 10 000). |
| `loom_get` | `rows`? (24) | Rule, binary, blurb, first row, run state, row counter, the newest rows. |
| `loom_set` | `rule`?, `start`?, `seed`?, `restart`?, `speed`?, `running`?, `step`?, `rows`?, `show`? | Applies them in that order. |
| `terrarium_get` | — | Six jars (rule, name, population, liveliness + level, kept), brood, mutation, keep note, chosen rule, lineage. |
| `terrarium_keep` | `jars` (1–6), `kept`?, `show`? | Keeps / un-keeps jars. |
| `terrarium_breed` | `mutation`? (1–4), `ticks`?, `show`? | Breeds the next brood, then optionally runs the jars. |
| `lab_get` | `samples`? | Script (+ errors), spec, scrubbed generation and alive, Play, report, series, RLE. |
| `lab_set_script` | `script`, `samples`?, `show`? | Replaces the rule script; a script with errors changes nothing and the error lists them by line. |
| `lab_scrub` | `generation`, `samples`?, `show`? | Scrubs (clamped to the run; pauses Play). |

Tools that change a screen switch the app to it (`show: false` to change it in the
background); the `*_get` tools don't. When the app isn't running every app tool except
`app_status` / `list_screens` returns an error that starts "Automata Playground isn't running —
open Automata Playground (dist/Automata Playground.app, or `cargo run --release -p
automata-playground` in the automata-playground checkout) and try again." If the app doesn't
answer in time (3 s status, 5 s navigate and reads, 10 s describe, 20 s screenshot, 60 s for
steps / script runs), the error says that instead. A minimized or covered window still answers:
it asks eframe to paint a frame for each request (`RequestPaintWhileHidden`).

## Protocol

Built on `lab-mcp` (from [labkit](https://github.com/alexwhb/labkit)): stdio, one JSON-RPC 2.0 message per line;
protocol **2026-07-28** (stateless, per-request `_meta`, `server/discover`) and the legacy
`initialize` handshake for 2025-11-25, 2025-06-18, 2025-03-26 and 2024-11-05; tools only. Tool
calls run concurrently, so `ping` and `notifications/cancelled` work while a screenshot or a
long `run_life` is in progress (`run_life` stops early when cancelled).

## Adding a tool

Tools are registered in three modules, in this order (also the `tools/list` order):

- `src/engine.rs`: **engine tools**. Use `ap-engine` (and `ap-shared` for input parsing and
  payloads), never the app or egui. The module docs walk through it.
- `src/app.rs`: the lab-series app tools (status, screens, navigate, screenshot, describe).
- `src/benches.rs`: the Automata screen tools, each a `forward(…)` of its arguments to a bridge
  method.

```rust
tools.register(
    Tool::new("population_at", "The population of a pattern after `gens` generations of a rule.")
        .title("Population at")
        .read_only()
        .input_schema(json!({
            "type": "object",
            "properties": {
                "pattern": {"type": "string"},
                "gens": {"type": "integer", "minimum": 0, "maximum": 1000}
            },
            "required": ["pattern", "gens"],
            "additionalProperties": false
        })),
    |call| {
        #[derive(serde::Deserialize)]
        struct Args { pattern: String, gens: usize }
        let Args { pattern, gens } = call.args()?;         // already checked against the schema
        let input = ap_shared::input::pattern_input(&pattern).map_err(ToolError::new)?;
        let mut grid = ap_engine::Grid::new(64, 40);
        grid.place(&input.file.pattern, 32, 20, ap_engine::Anchor::Center, ap_engine::Boundary::Torus);
        for _ in 0..gens { grid.step(&ap_engine::LifeRule::LIFE, ap_engine::Boundary::Torus); }
        Ok(ToolOutput::json(&json!({"population": grid.population()})))
    },
)?;
```

- Write the description for the model: what it does, defaults, limits, what to call next.
- Return `ToolOutput::json(…)` (add `.with_first(Content::text(summary))`), or
  `Err(ToolError)` for anything the model can fix. Keep every call to a second or two.
- For an **app tool**: add the method name and its params / result types to
  `crates/ap-shared/src/proto.rs`, answer it in `crates/automata-playground/src/control.rs`
  (`screen_method`: parse params, check them, `prepare(…)` the screen, call its control API,
  reply with its state), and register a `forward(…)` in `src/benches.rs` whose schema uses the
  same field names.
- Test it end to end in `tests/e2e.rs`: it spawns the real binary
  (`env!("CARGO_BIN_EXE_automata-mcp")`) and, for app tools, runs the real app in an
  egui_kittest harness on a temp socket (`tests/common/mod.rs`); check the change on the
  screen's state and through the GUI's AccessKit labels.

## Tests

```sh
CARGO_TARGET_DIR=target/mcp cargo test -p ap-mcp                                 # unit + e2e
CARGO_TARGET_DIR=target/mcp cargo test -p automata-playground --test mcp_bridge  # app side
CARGO_TARGET_DIR=target/mcp cargo test -p ap-shared
```

`AP_MCP_E2E_DUMP=<dir>` makes the e2e test save its screenshot and outline in `<dir>`.

The e2e suite covers `initialize` → `tools/list` (every tool, with title, description, closed
object schema and annotations), every engine tool with its error cases (including
`analyze_rule` on the mockup's spec: peak 221 @ 148, final 46, period 2, "Settles into a
period-2 cycle at gen 388."), `navigate`, `app_status`, `list_screens`, `screenshot` (a decoded
1440 × 900 PNG, not blank), `describe_screen`, the Tray / Loom / Terrarium / Rule-lab tools
(checked on the screens' state and the GUI's labels), schema errors, a stateless 2026-07-28
call, every app tool with no app, the app restarting under a running server, `--version`,
`--help` and a bad flag. `tests/mcp_bridge.rs` in the app covers the bridge methods directly,
bad params changing nothing, a client leaving mid-screenshot, a hidden window asking for a
frame, a second instance, a change to a never-visited screen surviving its first visit, and
that `ap-shared`'s screen table matches the router. What the tests can't cover is eframe's own
window read-back on macOS and its painting of a really hidden window.
