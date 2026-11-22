//! The Automata tools on the running app, one group per screen, forwarded over the live
//! bridge to the screens' control APIs (`crates/automata-playground/src/control.rs`):
//!
//! | tool | bridge method | what |
//! |---|---|---|
//! | `tray_get` | `tray.get` | rule, generation, population, born, died, mood, run state, history, RLE |
//! | `tray_set_rule` | `tray.set_rule` | set the rule (cells stay) |
//! | `tray_load_pattern` | `tray.load_pattern` | clear and place a pattern (RLE / plaintext / stamp) centred |
//! | `tray_control` | `tray.control` | run, pause, step n, clear, restart, randomize (seed) |
//! | `loom_get` | `loom.get` | rule, binary, blurb, first row, run state, row counter, newest rows |
//! | `loom_set` | `loom.set` | rule, first row (seed), restart, speed, run / pause, step n |
//! | `terrarium_get` | `terrarium.get` | the six jars (rule, name, population, liveliness, kept), brood, lineage |
//! | `terrarium_keep` | `terrarium.keep` | keep / un-keep jars |
//! | `terrarium_breed` | `terrarium.breed` | breed the next brood (mutation), then tick |
//! | `lab_get` | `lab.get` | script, errors, spec, generation, alive, report, series, RLE |
//! | `lab_set_script` | `lab.set_script` | replace the rule script (errors leave it unchanged) |
//! | `lab_scrub` | `lab.scrub` | scrub to a generation |
//!
//! The arguments object is sent as the method's params unchanged (the schemas here and the
//! param structs in `ap_shared::proto` use the same names). Every result is the screen's state
//! after the call, as `structuredContent`, after a one-line summary.

use crate::app::{AppLink, NAVIGATE_TIMEOUT, WORK_TIMEOUT};
use ap_shared::proto::{self, LabState, LoomState, MAX_STEPS, TerrariumState, TrayState};
use lab_mcp::mcp::{Content, RegistryError, Tool, ToolOutput, ToolRegistry};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use std::sync::Arc;
use std::time::Duration;

/// Registers a tool that forwards its arguments to `method` and shows the reply.
fn forward<T: DeserializeOwned + Serialize + 'static>(
    tools: &mut ToolRegistry,
    link: &Arc<AppLink>,
    tool: Tool,
    method: &'static str,
    timeout: Duration,
    summary: fn(&T) -> String,
) -> Result<(), RegistryError> {
    let link = Arc::clone(link);
    tools.register(tool, move |call| {
        let state: T = link.call(method, Value::Object(call.arguments.clone()), timeout)?;
        Ok(ToolOutput::json(&state).with_first(Content::text(summary(&state))))
    })
}

fn object(properties: Value, required: &[&str]) -> Value {
    json!({
        "type": "object",
        "properties": properties,
        "required": required,
        "additionalProperties": false
    })
}

const PATTERN_DESC: &str = "RLE (`x = 3, y = 3` then `bo$2bo$3o!`), plaintext rows of `.` and \
`O`, or a stamp id: glider, lwss, pulsar, penta, rpent, acorn, gun, rep.";
const SAMPLES_DESC: &str = "Points in the population series (2–2001, default 41).";

/// Registers the Tray, Loom, Terrarium and Rule-lab tools.
pub fn register(tools: &mut ToolRegistry, link: AppLink) -> Result<(), RegistryError> {
    let link = Arc::new(link);
    let steps = MAX_STEPS;

    // ---- Tray
    forward(
        tools,
        &link,
        Tool::new(
            "tray_get",
            "The running app's Tray (the 40 × 26 torus): rule code and name (cartridge or \
             \"Custom rule\"), generation, population, born and died in the last step, the \
             mood callout, run state and speed, the population history (≤ 120) and the whole \
             tray as RLE with `rule =`. Doesn't switch screens.",
        )
        .title("Read the Tray")
        .read_only(),
        proto::TRAY_GET,
        NAVIGATE_TIMEOUT,
        tray_summary,
    )?;
    forward(
        tools,
        &link,
        Tool::new(
            "tray_set_rule",
            "Set the Tray's rule (Moore neighbourhood); the cells stay. A rulestring \
             (`B36/S23`) or a name (Life, HighLife, Seeds, Day & Night, Maze, Coral, 2×2, \
             Move). Returns the Tray afterwards.",
        )
        .title("Set the Tray's rule")
        .idempotent()
        .input_schema(object(
            json!({
                "rule": {"type": "string", "description": "The rule, e.g. B3/S23 or HighLife.",
                         "maxLength": 1000},
                "show": crate::show_schema("Tray")
            }),
            &["rule"],
        )),
        proto::TRAY_SET_RULE,
        NAVIGATE_TIMEOUT,
        tray_summary,
    )?;
    forward(
        tools,
        &link,
        Tool::new(
            "tray_load_pattern",
            "Clear the Tray and place a pattern centred on it (wrapping; patterns larger than \
             40 × 26 are cropped around their centre). The counters start over at gen 0 and \
             the run state stays. A `rule =` in the RLE header is applied too, unless you give \
             `rule`. Returns the Tray afterwards (`note` says what was loaded / cropped).",
        )
        .title("Load a pattern into the Tray")
        .idempotent()
        .input_schema(object(
            json!({
                "pattern": {"type": "string", "description": PATTERN_DESC, "maxLength": 1_000_000},
                "rule": {"type": "string", "description": "Also set this rule (default: the pattern's `rule =`, else keep the Tray's).",
                         "maxLength": 1000},
                "show": crate::show_schema("Tray")
            }),
            &["pattern"],
        )),
        proto::TRAY_LOAD_PATTERN,
        NAVIGATE_TIMEOUT,
        tray_summary,
    )?;
    forward(
        tools,
        &link,
        Tool::new(
            "tray_control",
            "Drive the Tray like its buttons: `run`, `pause`, `step` (pauses, then advances \
             `n` generations, default 1), `clear` (rule kept), `restart` (the starting garden \
             under Life), `randomize` (a 30 % soup from `seed`, rule kept). `speed` (1–30 \
             gen/s) can be set with any action. Returns the Tray afterwards.",
        )
        .title("Run / pause / step the Tray")
        .annotations(changes())
        .input_schema(object(
            json!({
                "action": {"type": "string",
                           "enum": ["run", "pause", "step", "clear", "restart", "randomize"],
                           "description": "What to do."},
                "n": {"type": "integer", "minimum": 1, "maximum": steps,
                      "description": "Generations for `step` (default 1)."},
                "seed": {"type": "integer", "minimum": 0,
                         "description": "Seed for `randomize` (default: chosen by the app; the reply's `note` says which)."},
                "speed": {"type": "number", "minimum": 1, "maximum": 30,
                          "description": "Generations per second while running."},
                "show": crate::show_schema("Tray")
            }),
            &["action"],
        )),
        proto::TRAY_CONTROL,
        WORK_TIMEOUT,
        tray_summary,
    )?;

    // ---- Loom
    forward(
        tools,
        &link,
        Tool::new(
            "loom_get",
            "The running app's Loom (elementary CA, 121 cells, wrapping): rule, its binary \
             and blurb, the first row (single / random + seed), run state and speed, the \
             `row` counter and the newest `rows` rows (default 24, up to 72; # alive, . dead, \
             oldest first). Doesn't switch screens.",
        )
        .title("Read the Loom")
        .read_only()
        .input_schema(object(
            json!({"rows": {"type": "integer", "minimum": 0, "maximum": 72,
                            "description": "Newest rows to include (default 24)."}}),
            &[],
        )),
        proto::LOOM_GET,
        NAVIGATE_TIMEOUT,
        loom_summary,
    )?;
    forward(
        tools,
        &link,
        Tool::new(
            "loom_set",
            "Change the Loom; every field is optional and they apply in this order: `rule` \
             (0–255; restarts the thread), `start` single / random and `seed` (restarts), \
             `restart`, `speed` (2–40 rows/s), `running` (weave / pause), `step` (weave n rows \
             now). Returns the Loom afterwards with the newest `rows` rows.",
        )
        .title("Change the Loom")
        .annotations(changes())
        .input_schema(object(
            json!({
                "rule": {"type": "integer", "minimum": 0, "maximum": 255, "description": "The Wolfram rule."},
                "start": {"type": "string", "enum": ["single", "random"], "description": "The first row."},
                "seed": {"type": "integer", "minimum": 0, "description": "The random first row's seed (implies start: random)."},
                "restart": {"type": "boolean", "description": "Start the thread over (New thread)."},
                "speed": {"type": "number", "minimum": 2, "maximum": 40, "description": "Rows per second."},
                "running": {"type": "boolean", "description": "Weave (true) or pause (false)."},
                "step": {"type": "integer", "minimum": 1, "maximum": steps, "description": "Weave this many rows now."},
                "rows": {"type": "integer", "minimum": 0, "maximum": 72, "description": "Newest rows to include in the reply (default 24)."},
                "show": crate::show_schema("Loom")
            }),
            &[],
        )),
        proto::LOOM_SET,
        WORK_TIMEOUT,
        loom_summary,
    )?;

    // ---- Terrarium
    forward(
        tools,
        &link,
        Tool::new(
            "terrarium_get",
            "The running app's Terrarium: the six jars (number 1–6, rule code, name or \
             \"Mutant\", population of 32 × 32, liveliness 0–1 with its level quiet / lively / \
             frantic, kept), the brood number, mutation, run state, the keep note, the rule \
             \"Plant in the Tray\" would carry and the lineage. Doesn't switch screens.",
        )
        .title("Read the Terrarium")
        .read_only(),
        proto::TERRARIUM_GET,
        NAVIGATE_TIMEOUT,
        terrarium_summary,
    )?;
    forward(
        tools,
        &link,
        Tool::new(
            "terrarium_keep",
            "Keep (or with `kept: false`, un-keep) Terrarium jars by number (1–6, as on \
             screen). Kept jars are the parents of the next brood and carry over unchanged. \
             Returns the Terrarium afterwards.",
        )
        .title("Keep jars")
        .idempotent()
        .input_schema(object(
            json!({
                "jars": {"type": "array", "items": {"type": "integer", "minimum": 1, "maximum": 6},
                         "minItems": 1, "description": "Jar numbers, 1–6."},
                "kept": {"type": "boolean", "description": "Keep (true, default) or un-keep."},
                "show": crate::show_schema("Terrarium")
            }),
            &["jars"],
        )),
        proto::TERRARIUM_KEEP,
        NAVIGATE_TIMEOUT,
        terrarium_summary,
    )?;
    forward(
        tools,
        &link,
        Tool::new(
            "terrarium_breed",
            "Breed the next Terrarium brood (\"Breed brood N+1\"): kept jars carry over \
             unchanged in fresh soup, the other jars become children whose birth / survival \
             bits each come from a random parent, then `mutation` bits (1–4) flip. Parents are \
             the kept jars, or all six when none are kept; keeps are cleared. With `ticks`, \
             every jar then runs that many generations so liveliness means something. Returns \
             the Terrarium afterwards.",
        )
        .title("Breed a brood")
        .annotations(changes())
        .input_schema(object(
            json!({
                "mutation": {"type": "integer", "minimum": 1, "maximum": 4,
                             "description": "Bit flips per child (default: the screen's setting, initially 2)."},
                "ticks": {"type": "integer", "minimum": 0, "maximum": steps,
                          "description": "Generations to run every jar afterwards (default 0)."},
                "show": crate::show_schema("Terrarium")
            }),
            &[],
        )),
        proto::TERRARIUM_BREED,
        WORK_TIMEOUT,
        terrarium_summary,
    )?;

    // ---- Rule lab
    forward(
        tools,
        &link,
        Tool::new(
            "lab_get",
            "The running app's Rule lab: the script as shown (and its line-numbered errors, \
             if any), the spec in effect (rule, neighbourhood, boundary, grid, seed, gens), \
             the scrubbed generation and its live cells, Play state, the report (peak, final, \
             first repeat, period, verdict), a population series and the scrubbed \
             generation's RLE. Doesn't switch screens.",
        )
        .title("Read the Rule lab")
        .read_only()
        .input_schema(object(
            json!({"samples": {"type": "integer", "minimum": 2, "maximum": 2001, "description": SAMPLES_DESC}}),
            &[],
        )),
        proto::LAB_GET,
        WORK_TIMEOUT,
        lab_summary,
    )?;
    forward(
        tools,
        &link,
        Tool::new(
            "lab_set_script",
            "Replace the Rule lab's rule script (the format `rule_script` checks and \
             `analyze_rule` returns). When it parses, it becomes the editor's text, its spec \
             takes effect and the run and report update; when it doesn't, nothing changes and \
             the error lists every problem with its line number. Returns the Rule lab \
             afterwards.",
        )
        .title("Set the rule script")
        .idempotent()
        .input_schema(object(
            json!({
                "script": {"type": "string", "description": "The whole script.", "maxLength": 100_000},
                "samples": {"type": "integer", "minimum": 2, "maximum": 2001, "description": SAMPLES_DESC},
                "show": crate::show_schema("Rule lab")
            }),
            &["script"],
        )),
        proto::LAB_SET_SCRIPT,
        WORK_TIMEOUT,
        lab_summary,
    )?;
    forward(
        tools,
        &link,
        Tool::new(
            "lab_scrub",
            "Scrub the Rule lab to a generation (clamped to the run; pauses Play, like the \
             slider). The top-down view, `alive` and the RLE follow. Returns the Rule lab \
             afterwards.",
        )
        .title("Scrub the Rule lab")
        .idempotent()
        .input_schema(object(
            json!({
                "generation": {"type": "integer", "minimum": 0, "maximum": 2000, "description": "The generation."},
                "samples": {"type": "integer", "minimum": 2, "maximum": 2001, "description": SAMPLES_DESC},
                "show": crate::show_schema("Rule lab")
            }),
            &["generation"],
        )),
        proto::LAB_SCRUB,
        NAVIGATE_TIMEOUT,
        lab_summary,
    )?;
    Ok(())
}

/// Changes state, not destructively, and not idempotently (stepping twice steps twice).
fn changes() -> lab_mcp::mcp::ToolAnnotations {
    lab_mcp::mcp::ToolAnnotations {
        read_only: Some(false),
        destructive: Some(false),
        idempotent: Some(false),
        open_world: None,
    }
}

fn note(n: &Option<String>) -> String {
    n.as_deref().map(|n| format!(" {n}")).unwrap_or_default()
}

fn tray_summary(t: &TrayState) -> String {
    format!(
        "Tray: {} ({}), gen {}, {} alive (born {}, died {}) — {}; {}.{}",
        t.rule,
        t.rule_name,
        t.generation,
        t.population,
        t.born,
        t.died,
        t.mood.title,
        if t.running {
            format!("running at {} gen/s", t.speed)
        } else {
            "paused".to_owned()
        },
        note(&t.note)
    )
}

fn loom_summary(l: &LoomState) -> String {
    format!(
        "Loom: rule {} ({}, {}), {} first row, row {}; {}.{}",
        l.rule,
        l.binary,
        l.blurb,
        match l.start {
            proto::FirstRow::Single => "single-cell".to_owned(),
            proto::FirstRow::Random => format!(
                "random (seed {})",
                l.seed.map_or("?".into(), |s| s.to_string())
            ),
        },
        l.row,
        if l.running {
            format!("weaving at {} rows/s", l.speed)
        } else {
            "paused".to_owned()
        },
        note(&l.note)
    )
}

fn terrarium_summary(t: &TerrariumState) -> String {
    let jars: Vec<String> = t
        .jars
        .iter()
        .map(|j| {
            format!(
                "{} {} {}{} ({} alive, {})",
                j.number,
                j.rule,
                j.name,
                if j.kept { " [kept]" } else { "" },
                j.population,
                j.level
            )
        })
        .collect();
    format!(
        "Terrarium brood {}: {}. {}.{}",
        t.brood,
        jars.join("; "),
        t.keep_note,
        note(&t.note)
    )
}

fn lab_summary(l: &LabState) -> String {
    let errors = if l.errors.is_empty() {
        String::new()
    } else {
        format!(
            " The editor's text has {} error(s); the last valid script is in effect.",
            l.errors.len()
        )
    };
    format!(
        "Rule lab: {} · {} · {} {}×{} · {} · {} gens; gen {} · {} alive. {}{}{}",
        l.spec.rule.code,
        l.spec.neighbourhood.name,
        l.spec.boundary,
        l.spec.width,
        l.spec.height,
        l.spec.seed.label,
        l.spec.gens,
        l.generation,
        l.alive,
        l.report.verdict,
        errors,
        note(&l.note)
    )
}
