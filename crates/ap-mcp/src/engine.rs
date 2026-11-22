//! Engine tools: they run `ap-engine` in-process and headless, so they work without the app.
//!
//! | tool | what |
//! |---|---|
//! | `run_life` | a Life-like rule on a pattern for N generations: population summary and series, born / died, exact repeat, spaceship motion, the final RLE, an optional ASCII render |
//! | `analyze_rule` | a Rule-lab spec (rule, neighbourhood, boundary, seed, size, gens) → the Rule lab's report |
//! | `elementary` | an elementary CA (rule 0–255): rows as text, per-row counts |
//! | `parse_rule` | a rulestring or name → canonical code, counts, name |
//! | `parse_pattern` | RLE / plaintext / stamp id → size, population, canonical RLE |
//! | `list_presets` | cartridges, stamps, famous threads, named rules, neighbourhoods, seeds |
//! | `rule_script` | validate / normalise a Rule-lab script → line-numbered errors, or the canonical text and its report |
//!
//! # Adding an engine tool
//!
//! 1. Use `ap-engine` (and `ap-shared` for input parsing and payloads), never the app or egui.
//! 2. Register the tool in [`register`]: a `snake_case` name, a title, a description written
//!    for the model (what it does, defaults, limits, what to call next) and a JSON Schema of
//!    its arguments. Arguments are validated against the schema before the handler runs;
//!    deserialize them with `call.args::<YourArgs>()`.
//! 3. Return [`ToolOutput::json`] (structured content plus the same JSON as text) with a
//!    one-line summary first, or a [`ToolError`] whose message says what to send instead.
//!    Clamp sizes so no call can take more than a couple of seconds.
//! 4. Test it through the real binary in `tests/e2e.rs`.

use ap_engine::analysis::{MAX_RUN_DIM, MAX_RUN_GENS};
use ap_engine::pattern::{library, rle};
use ap_engine::presets::{self, CARTRIDGES, NAMED};
use ap_engine::{
    BBox, Boundary, Elementary, Frame, Grid, LifeRule, Neighbourhood, NeighbourhoodPreset, Pattern,
    RuleScript, RunSpec, Seed, Weave, WeaveStart, elementary,
};
use ap_shared::input::{self, PatternInput, PatternSource};
use ap_shared::proto::{
    DEFAULT_SAMPLES, NeighbourhoodInfo, RepeatInfo, ReportInfo, RuleInfo, ScriptErrorInfo,
    SpecInfo, sample_series,
};
use lab_mcp::mcp::{Content, RegistryError, Tool, ToolError, ToolOutput, ToolRegistry};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::collections::hash_map::Entry;

/// `run_life`'s largest grid side.
pub const RUN_MAX_DIM: usize = 256;
/// `run_life`'s most generations.
pub const RUN_MAX_GENS: usize = 10_000;
/// `run_life`'s largest job, in cell-generations (width × height × gens).
pub const RUN_MAX_WORK: u64 = 200_000_000;
/// `run_life`'s default grid (the Rule lab's 64 × 40).
pub const RUN_DEFAULT_SIZE: (usize, usize) = (64, 40);
/// `run_life`'s default generations.
pub const RUN_DEFAULT_GENS: usize = 100;
/// The largest side of an ASCII render.
pub const RENDER_MAX: usize = 128;
/// `elementary`'s largest width and row count.
pub const ELEMENTARY_MAX_DIM: usize = 1024;
/// `elementary`'s most cells in one answer (width × rows).
pub const ELEMENTARY_MAX_CELLS: usize = 65_536;
/// Memory for exact-repeat detection in `run_life`, in 64-bit words of stored frames.
const REPEAT_BUDGET_WORDS: usize = 4 << 20;
/// Memory for spaceship detection in `run_life`, in stored live cells.
const SHAPE_BUDGET_CELLS: usize = 2 << 20;

const PATTERN_DESC: &str = "The pattern: RLE (`x = 3, y = 3, rule = B3/S23` then `bo$2bo$3o!`; \
the header is optional), plaintext rows of `.` and `O` (`!` comment lines allowed), or a stamp \
id: glider, lwss, pulsar, penta, rpent, acorn, gun (Gosper glider gun), rep (HighLife \
replicator).";
const RULE_DESC: &str = "A Life-like rule: `B3/S23` (also `b3s23`, `23/3`, `B3,10/S2,3` for \
counts ≥ 10) or a name: Life, HighLife, Seeds, Day & Night, Maze, Coral, 2×2, Move.";
const NEIGHBOURHOOD_DESC: &str = "The neighbourhood: a preset (`moore` = 8 cells, `vn` = von \
Neumann 4, `hex` = hexagonal 6, `moore2` = 24) or a list of [dx, dy] offsets in -2…2 (not \
[0, 0]). Default moore.";
const SAMPLES_DESC: &str =
    "Points in the population series (2–2001, default 41; both ends always included).";

/// Registers the engine tools.
pub fn register(tools: &mut ToolRegistry) -> Result<(), RegistryError> {
    tools.register(
        Tool::new(
            "run_life",
            "Run a Life-like rule on a pattern, headless (no app needed). The pattern is \
             centred on a width × height grid (default 64 × 40, grown to fit a bigger pattern \
             when you give no size; at most 256 × 256) with torus or dead edges, then run for \
             `gens` generations (default 100, at most 10 000; width × height × gens ≤ 2·10⁸). \
             Returns the population (initial, final, peak @ gen, min), a sampled series, total \
             born / died, the last step's born / died, when it died out, the first exact \
             repeat (start, period), spaceship motion (dx, dy per period, detected before it \
             wraps), a verdict, the final pattern as RLE and, with `render`, the final grid as \
             ASCII (O alive, . dead).",
        )
        .title("Run a Life-like rule")
        .read_only()
        .input_schema(json!({
            "type": "object",
            "properties": {
                "pattern": {"type": "string", "description": PATTERN_DESC, "maxLength": 1_000_000},
                "rule": {"type": "string", "description": format!(
                    "{RULE_DESC} Default: the pattern's `rule =` header, else B3/S23.")},
                "neighbourhood": neighbourhood_schema(),
                "width": {"type": "integer", "minimum": 1, "maximum": RUN_MAX_DIM,
                          "description": "Grid width (default 64, or the pattern + 8 cells each side)."},
                "height": {"type": "integer", "minimum": 1, "maximum": RUN_MAX_DIM,
                           "description": "Grid height (default 40, or the pattern + 8 cells each side)."},
                "boundary": boundary_schema(),
                "gens": {"type": "integer", "minimum": 0, "maximum": RUN_MAX_GENS,
                         "description": "Generations to run (default 100)."},
                "samples": {"type": "integer", "minimum": 2, "maximum": RUN_MAX_GENS + 1,
                            "description": SAMPLES_DESC},
                "render": {"type": "boolean", "description":
                    "Include the final grid as ASCII rows (the whole grid up to 128 × 128, else \
                     the live cells' bounding box if it fits)."}
            },
            "required": ["pattern"],
            "additionalProperties": false
        })),
        |call| {
            let args: RunLifeArgs = call.args()?;
            let run = run_life(&args, &|| call.is_cancelled())?;
            Ok(ToolOutput::json(&run).with_first(Content::text(run.summary())))
        },
    )?;

    tools.register(
        Tool::new(
            "analyze_rule",
            "Analyse a rule the way the Rule lab does (no app needed): run the spec for `gens` \
             generations keeping every frame, then report the peak population @ gen, the final \
             population, the first exact repeat and period, and the verdict (\"Dies out by gen \
             N.\" / \"Freezes into still lifes at gen N.\" / \"Settles into a period-P cycle at \
             gen N.\" / \"No repeat within G gens.\"). Defaults are the Rule lab's: B3/S23, \
             Moore, torus, 64 × 40, R-pentomino seed, 400 gens. Also returns a population \
             series, the RLE at `generation` (default the last) and the equivalent rule script \
             (paste it into the app with `lab_set_script`).",
        )
        .title("Analyse a rule")
        .read_only()
        .input_schema(json!({
            "type": "object",
            "properties": {
                "rule": {"type": "string", "description": format!("{RULE_DESC} Default B3/S23.")},
                "neighbourhood": neighbourhood_schema(),
                "boundary": boundary_schema(),
                "seed": {"type": "string", "enum": ["r_pentomino", "rpent", "acorn", "soup"],
                         "description": "The starting cells: the R-pentomino (default) or the \
                                         acorn centred on the grid, or a random soup in the \
                                         grid's central box."},
                "soup_seed": {"type": "integer", "minimum": 0, "maximum": u32::MAX,
                              "description": "The soup's seed (default 7, the mockup's)."},
                "density": {"type": "number", "minimum": 0, "maximum": 1,
                            "description": "The soup's density (default 0.33)."},
                "width": {"type": "integer", "minimum": 1, "maximum": MAX_RUN_DIM,
                          "description": "Grid width (default 64)."},
                "height": {"type": "integer", "minimum": 1, "maximum": MAX_RUN_DIM,
                           "description": "Grid height (default 40)."},
                "gens": {"type": "integer", "minimum": 1, "maximum": MAX_RUN_GENS,
                         "description": "Generations (default 400)."},
                "samples": {"type": "integer", "minimum": 2, "maximum": MAX_RUN_GENS + 1,
                            "description": SAMPLES_DESC},
                "generation": {"type": "integer", "minimum": 0, "maximum": MAX_RUN_GENS,
                               "description": "Which generation's RLE to return (default the last)."}
            },
            "additionalProperties": false
        })),
        |call| {
            let args: AnalyzeArgs = call.args()?;
            let a = analyze(&args)?;
            Ok(ToolOutput::json(&a).with_first(Content::text(a.summary())))
        },
    )?;

    tools.register(
        Tool::new(
            "elementary",
            "Weave an elementary (Wolfram) cellular automaton, as the Loom does (no app \
             needed): rule 0–255 on a wrapping row of `width` cells (default 121), `rows` rows \
             (default 32) from a single centre cell or a random first row (`seed`). Returns the \
             rows as text (# alive, . dead, oldest first), live cells per row, the rule's \
             binary (outputs for 111 → 000) and its blurb (\"Chaos · Random-looking\" for \
             famous rules). width × rows ≤ 65 536.",
        )
        .title("Elementary CA")
        .read_only()
        .input_schema(json!({
            "type": "object",
            "properties": {
                "rule": {"type": "integer", "minimum": 0, "maximum": 255,
                         "description": "The Wolfram rule number."},
                "width": {"type": "integer", "minimum": 1, "maximum": ELEMENTARY_MAX_DIM,
                          "description": "Cells per row (default 121)."},
                "rows": {"type": "integer", "minimum": 1, "maximum": ELEMENTARY_MAX_DIM,
                         "description": "Rows, the first included (default 32)."},
                "start": {"type": "string", "enum": ["single", "random"],
                          "description": "First row: one live centre cell (default) or random (½ each)."},
                "seed": {"type": "integer", "minimum": 0,
                         "description": "Seed of the random first row (default 1; implies start: random)."}
            },
            "required": ["rule"],
            "additionalProperties": false
        })),
        |call| {
            let args: ElementaryArgs = call.args()?;
            let e = weave(&args)?;
            Ok(ToolOutput::json(&e).with_first(Content::text(e.summary())))
        },
    )?;

    tools.register(
        Tool::new(
            "parse_rule",
            "Read a Life-like rule (no app needed): a rulestring in any common form (`B3/S23`, \
             `b3s23`, `B3/S23/`, `S23/B3`, `23/3`, comma counts `B3,10/S2,3`) or a name. Returns \
             the canonical code, the birth and survival counts, the name if it has one and the \
             Tray cartridge it matches. Errors say what is wrong.",
        )
        .title("Parse a rule")
        .read_only()
        .input_schema(json!({
            "type": "object",
            "properties": {
                "rule": {"type": "string", "description": RULE_DESC, "maxLength": 1000},
                "neighbourhood": neighbourhood_schema()
            },
            "required": ["rule"],
            "additionalProperties": false
        })),
        |call| {
            #[derive(Deserialize)]
            struct Args {
                rule: String,
                #[serde(default)]
                neighbourhood: Option<Value>,
            }
            let args: Args = call.args()?;
            let nb = neighbourhood(args.neighbourhood.as_ref())?;
            let rule = input::rule_input(&args.rule, nb).map_err(ToolError::new)?;
            let info = RuleInfo::from(&rule);
            let cartridge = presets::matching_cartridge(&rule)
                .map(|c| json!({"id": c.id, "name": c.name, "note": c.note}));
            let text = format!(
                "{} = {} ({}; {}).",
                args.rule.trim(),
                info.code,
                info.name.as_deref().unwrap_or("no name"),
                rule.neighbourhood().name()
            );
            let v = json!({
                "input": args.rule,
                "code": info.code,
                "birth": info.birth,
                "survive": info.survive,
                "name": info.name,
                "neighbourhood": NeighbourhoodInfo::from(rule.neighbourhood()),
                "cartridge": cartridge,
            });
            Ok(ToolOutput::json(&v).with_first(Content::text(text)))
        },
    )?;

    tools.register(
        Tool::new(
            "parse_pattern",
            "Read a pattern (no app needed): RLE, plaintext or a stamp id. Returns its kind, \
             name / author / comments, the RLE header's rule, its size and population, the \
             canonical RLE (70-column lines, `rule =` kept) and, when it is at most 64 × 64, \
             its rows (O alive, . dead).",
        )
        .title("Parse a pattern")
        .read_only()
        .input_schema(json!({
            "type": "object",
            "properties": {
                "pattern": {"type": "string", "description": PATTERN_DESC, "maxLength": 1_000_000}
            },
            "required": ["pattern"],
            "additionalProperties": false
        })),
        |call| {
            #[derive(Deserialize)]
            struct Args {
                pattern: String,
            }
            let args: Args = call.args()?;
            let p = parse_pattern(&args.pattern)?;
            Ok(ToolOutput::json(&p).with_first(Content::text(p.summary())))
        },
    )?;

    tools.register(
        Tool::new(
            "list_presets",
            "Everything built in (no app needed): the Tray's six rule cartridges, the eight \
             stamps (id, name, size, RLE), the Loom's six famous threads, the named rules \
             (the Terrarium's names map) and its six starting jars, the Rule lab's \
             neighbourhood presets, seeds and boundaries.",
        )
        .title("List presets")
        .read_only(),
        |_| {
            let p = presets_json();
            let text = format!(
                "{} cartridges, {} stamps, {} famous threads, {} named rules, {} neighbourhoods.",
                CARTRIDGES.len(),
                library::STAMPS.len(),
                elementary::FAMOUS.len(),
                NAMED.len(),
                NeighbourhoodPreset::ALL.len()
            );
            Ok(ToolOutput::json(&p).with_first(Content::text(text)))
        },
    )?;

    tools.register(
        Tool::new(
            "rule_script",
            "Validate and normalise a Rule-lab rule script (no app needed). The format is the \
             Rule lab's netlist: `* comment`, `.rule life_like B3/S23`, `.neigh moore n=8` \
             (or vn / hex / moore2, or `custom n=3 (0,-1) (-2,1) (2,2)`), `.grid 64x40 \
             boundary=torus|dead`, `.seed r_pentomino|acorn @ (32,20)` or `.seed soup seed=7 \
             density=0.33`, `.run gens=400`, optional `.probe population -> plot1` and `.end`. \
             Returns `valid: false` with every error and its line number, or the canonical \
             text, whether it differs from yours, the spec and (unless `analyze: false`) the \
             run's report. Load a script into the app with `lab_set_script`.",
        )
        .title("Check a rule script")
        .read_only()
        .input_schema(json!({
            "type": "object",
            "properties": {
                "script": {"type": "string", "description": "The whole script.", "maxLength": 100_000},
                "analyze": {"type": "boolean", "description": "Run the spec and report (default true)."},
                "samples": {"type": "integer", "minimum": 2, "maximum": MAX_RUN_GENS + 1,
                            "description": SAMPLES_DESC}
            },
            "required": ["script"],
            "additionalProperties": false
        })),
        |call| {
            #[derive(Deserialize)]
            struct Args {
                script: String,
                #[serde(default)]
                analyze: Option<bool>,
                #[serde(default)]
                samples: Option<usize>,
            }
            let args: Args = call.args()?;
            let r = check_script(
                &args.script,
                args.analyze.unwrap_or(true),
                args.samples.unwrap_or(DEFAULT_SAMPLES),
            );
            Ok(ToolOutput::json(&r).with_first(Content::text(r.summary())))
        },
    )?;
    Ok(())
}

fn neighbourhood_schema() -> Value {
    json!({
        "type": ["string", "array"],
        "description": NEIGHBOURHOOD_DESC,
        "items": {"type": "array", "items": {"type": "integer", "minimum": -2, "maximum": 2}}
    })
}

fn boundary_schema() -> Value {
    json!({
        "type": "string",
        "enum": ["torus", "dead"],
        "description": "torus: edges wrap (default); dead: cells beyond the edge are always dead."
    })
}

// ---- argument helpers -----------------------------------------------------------------------

/// The `neighbourhood` argument: a preset key or a list of `[dx, dy]` offsets.
pub fn neighbourhood(arg: Option<&Value>) -> Result<Neighbourhood, ToolError> {
    let bad = || {
        ToolError::new(
            "`neighbourhood` must be a preset (moore, vn, hex, moore2) or a list of [dx, dy] \
             pairs with dx, dy in -2…2, e.g. [[0,-1],[-1,0],[1,0],[0,1]]",
        )
    };
    match arg {
        None | Some(Value::Null) => Ok(Neighbourhood::MOORE),
        Some(Value::String(key)) => NeighbourhoodPreset::from_key(key)
            .map(NeighbourhoodPreset::neighbourhood)
            .ok_or_else(|| {
                ToolError::new(format!(
                    "unknown neighbourhood `{key}`; use moore, vn (von Neumann), hex or moore2, \
                     or a list of [dx, dy] offsets"
                ))
            }),
        Some(Value::Array(items)) => {
            let mut offsets = Vec::with_capacity(items.len());
            for item in items {
                let pair = item.as_array().filter(|p| p.len() == 2).ok_or_else(bad)?;
                let coord = |v: &Value| {
                    v.as_i64()
                        .and_then(|n| i32::try_from(n).ok())
                        .ok_or_else(bad)
                };
                offsets.push((coord(&pair[0])?, coord(&pair[1])?));
            }
            let nb = Neighbourhood::from_offsets(offsets)
                .map_err(|e| ToolError::new(format!("bad neighbourhood: {e}")))?;
            if nb.is_empty() {
                return Err(ToolError::new(
                    "the neighbourhood needs at least one cell (an offset other than [0, 0])",
                ));
            }
            Ok(nb)
        }
        Some(_) => Err(bad()),
    }
}

fn boundary(arg: Option<&str>) -> Result<Boundary, ToolError> {
    match arg {
        None => Ok(Boundary::Torus),
        Some(k) => Boundary::from_key(k)
            .ok_or_else(|| ToolError::new(format!("unknown boundary `{k}`; use torus or dead"))),
    }
}

fn pattern(text: &str) -> Result<PatternInput, ToolError> {
    input::pattern_input(text).map_err(ToolError::new)
}

// ---- run_life -------------------------------------------------------------------------------

/// `run_life`'s arguments.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct RunLifeArgs {
    /// RLE, plaintext or a stamp id.
    pub pattern: String,
    /// The rule (default: the pattern's `rule =`, else Life).
    #[serde(default)]
    pub rule: Option<String>,
    /// A preset key or offsets (default Moore).
    #[serde(default)]
    pub neighbourhood: Option<Value>,
    /// Grid width.
    #[serde(default)]
    pub width: Option<usize>,
    /// Grid height.
    #[serde(default)]
    pub height: Option<usize>,
    /// `torus` (default) or `dead`.
    #[serde(default)]
    pub boundary: Option<String>,
    /// Generations (default 100).
    #[serde(default)]
    pub gens: Option<usize>,
    /// Series points.
    #[serde(default)]
    pub samples: Option<usize>,
    /// Include an ASCII render of the final grid.
    #[serde(default)]
    pub render: Option<bool>,
}

/// The pattern a run started from.
#[derive(Debug, Clone, Serialize)]
pub struct PatternSummary {
    /// `stamp`, `rle` or `plaintext`.
    pub source: &'static str,
    /// The stamp id, for stamps.
    pub stamp: Option<&'static str>,
    /// `#N` / `!Name:` / the stamp's name.
    pub name: Option<String>,
    /// Bounding box width.
    pub width: usize,
    /// Bounding box height.
    pub height: usize,
    /// Live cells.
    pub population: usize,
    /// Where its top-left corner was placed on the grid.
    pub origin: [i64; 2],
}

/// Population over a run.
#[derive(Debug, Clone, Serialize)]
pub struct PopulationSummary {
    /// At generation 0.
    pub initial: usize,
    /// At the last generation.
    pub final_population: usize,
    /// The highest.
    pub peak: usize,
    /// The first generation reaching it.
    pub peak_gen: usize,
    /// The lowest.
    pub min: usize,
    /// The first generation reaching it.
    pub min_gen: usize,
}

/// Born / died in one step.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct StepCounts {
    /// Cells born.
    pub born: usize,
    /// Cells that died.
    pub died: usize,
}

/// A pattern that repeats its shape somewhere else: a spaceship.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct Motion {
    /// The generation the shape was first seen.
    pub start: usize,
    /// Generations per repeat.
    pub period: usize,
    /// Columns moved per period (positive = right).
    pub dx: i64,
    /// Rows moved per period (positive = down).
    pub dy: i64,
}

/// A live-cell bounding box.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct Box2 {
    /// Left column.
    pub x: usize,
    /// Top row.
    pub y: usize,
    /// Width.
    pub width: usize,
    /// Height.
    pub height: usize,
}

impl From<BBox> for Box2 {
    fn from(b: BBox) -> Self {
        Box2 {
            x: b.x,
            y: b.y,
            width: b.w,
            height: b.h,
        }
    }
}

/// An ASCII render of (part of) a grid.
#[derive(Debug, Clone, Serialize)]
pub struct Render {
    /// The rendered region (the whole grid, or the live cells' box).
    pub region: Box2,
    /// One string per row, `O` alive, `.` dead.
    pub rows: Vec<String>,
}

/// What `run_life` returns.
#[derive(Debug, Clone, Serialize)]
pub struct LifeRun {
    /// The rule run.
    pub rule: RuleInfo,
    /// Where the rule came from: `argument`, `pattern` (its `rule =`) or `default`.
    pub rule_source: &'static str,
    /// The neighbourhood.
    pub neighbourhood: NeighbourhoodInfo,
    /// `torus` or `dead`.
    pub boundary: &'static str,
    /// Grid width.
    pub width: usize,
    /// Grid height.
    pub height: usize,
    /// Generations run.
    pub gens: usize,
    /// The starting pattern.
    pub pattern: PatternSummary,
    /// The population over the run.
    pub population: PopulationSummary,
    /// `[generation, population]` samples.
    pub series: Vec<[usize; 2]>,
    /// Cells born over the whole run.
    pub born_total: u64,
    /// Cells that died over the whole run.
    pub died_total: u64,
    /// The last step's counts (`None` for 0 gens).
    pub last_step: Option<StepCounts>,
    /// The first generation with no live cells.
    pub extinct_gen: Option<usize>,
    /// The first exact repeat of the whole grid.
    pub first_repeat: Option<RepeatInfo>,
    /// How far the repeat search went: `complete`, or the generation it stopped at (memory).
    pub repeat_searched_to: Value,
    /// Spaceship motion, if the live cells repeated their shape somewhere else.
    pub motion: Option<Motion>,
    /// What happened, in one sentence.
    pub verdict: String,
    /// The live cells' box at the end.
    pub final_bbox: Option<Box2>,
    /// The final live cells as RLE (trimmed to their box) with `rule =`.
    pub final_rle: String,
    /// The final grid as ASCII, when asked for.
    pub render: Option<Render>,
}

impl LifeRun {
    /// One line for the model.
    pub fn summary(&self) -> String {
        let p = &self.population;
        format!(
            "{} on a {}×{} {} for {} gens: {} → {} cells (peak {} @ {}). {}",
            self.rule.code,
            self.width,
            self.height,
            self.boundary,
            self.gens,
            p.initial,
            p.final_population,
            p.peak,
            p.peak_gen,
            self.verdict
        )
    }
}

/// Runs `run_life`. `cancelled` is polled every 64 generations.
pub fn run_life(args: &RunLifeArgs, cancelled: &dyn Fn() -> bool) -> Result<LifeRun, ToolError> {
    let input = pattern(&args.pattern)?;
    let nb = neighbourhood(args.neighbourhood.as_ref())?;
    let (rule, rule_source) = match (&args.rule, &input.file.rule) {
        (Some(r), _) => (
            input::rule_input(r, nb).map_err(ToolError::new)?,
            "argument",
        ),
        (None, Some(header)) => match input::rule_input(header, nb) {
            Ok(rule) => (rule, "pattern"),
            Err(e) => {
                return Err(ToolError::new(format!(
                    "the pattern's `rule = {header}` header can't be used ({e}); pass `rule` \
                     to override it"
                )));
            }
        },
        (None, None) => (LifeRule::LIFE.with_neighbourhood(nb), "default"),
    };
    let boundary = boundary(args.boundary.as_deref())?;
    // like the Tray: a declared box too big for the grid counts by its live cells' bounding
    // box (`x = 300, y = 1` around one cell is a 1×1 pattern); one that fits keeps its margins
    let declared = &input.file.pattern;
    let fits = |n: usize, given: Option<usize>| n <= given.unwrap_or(RUN_MAX_DIM).min(RUN_MAX_DIM);
    let trimmed;
    let pat = if fits(declared.width(), args.width) && fits(declared.height(), args.height) {
        declared
    } else {
        trimmed = declared.trimmed();
        &trimmed
    };
    let (pw, ph) = (pat.width(), pat.height());
    if pw > RUN_MAX_DIM || ph > RUN_MAX_DIM {
        return Err(ToolError::new(format!(
            "the pattern is {pw}×{ph}; run_life grids are at most {RUN_MAX_DIM}×{RUN_MAX_DIM}"
        )));
    }
    let fit = |given: Option<usize>, default: usize, need: usize, axis: &str| match given {
        Some(n) if n < need => Err(ToolError::new(format!(
            "the pattern is {pw}×{ph}, which doesn't fit a grid {axis} of {n}; give a bigger \
             {axis} (or leave it out to fit the pattern)"
        ))),
        Some(n) => Ok(n.clamp(1, RUN_MAX_DIM)),
        None => Ok(default.max(need + 16).min(RUN_MAX_DIM)),
    };
    let w = fit(args.width, RUN_DEFAULT_SIZE.0, pw, "width")?;
    let h = fit(args.height, RUN_DEFAULT_SIZE.1, ph, "height")?;
    let gens = args.gens.unwrap_or(RUN_DEFAULT_GENS).min(RUN_MAX_GENS);
    let work = (w * h) as u64 * gens as u64;
    if work > RUN_MAX_WORK {
        return Err(ToolError::new(format!(
            "{w}×{h} cells × {gens} gens is {work} cell-generations; the limit is \
             {RUN_MAX_WORK}. Use a smaller grid or fewer gens"
        )));
    }

    let mut grid = Grid::new(w, h);
    let (cx, cy) = ((w / 2) as i64, (h / 2) as i64);
    grid.place(pat, cx, cy, ap_engine::Anchor::Center, boundary);
    let origin = [cx - (pw / 2) as i64, cy - (ph / 2) as i64];

    let mut pops = Vec::with_capacity(gens + 1);
    pops.push(grid.population());
    let mut tracker = RepeatTracker::new(w, h);
    tracker.record(&grid, 0);
    let (mut born_total, mut died_total, mut last_step) = (0u64, 0u64, None);
    for g in 1..=gens {
        if g % 64 == 0 && cancelled() {
            return Err(ToolError::new("cancelled"));
        }
        let st = grid.step(&rule, boundary);
        born_total += st.born as u64;
        died_total += st.died as u64;
        last_step = Some(StepCounts {
            born: st.born,
            died: st.died,
        });
        pops.push(st.population);
        tracker.record(&grid, g);
    }

    let (peak_gen, peak) = first_extreme(&pops, |a, b| a > b);
    let (min_gen, min) = first_extreme(&pops, |a, b| a < b);
    let final_population = pops.last().copied().unwrap_or(0);
    let extinct_gen = pops.iter().position(|&p| p == 0);
    let first_repeat = tracker.repeat.map(|(start, period)| RepeatInfo {
        start,
        period,
        seen_at: start + period,
    });
    let verdict = verdict(
        final_population,
        extinct_gen,
        first_repeat,
        tracker.motion,
        gens,
    );
    let final_pattern = Pattern::from_grid(&grid);
    Ok(LifeRun {
        rule: RuleInfo::from(&rule),
        rule_source,
        neighbourhood: rule.neighbourhood().into(),
        boundary: boundary.key(),
        width: w,
        height: h,
        gens,
        pattern: PatternSummary {
            source: input.source.key(),
            stamp: match input.source {
                PatternSource::Stamp(id) => Some(id),
                _ => None,
            },
            name: input.file.name.clone(),
            width: pw,
            height: ph,
            population: pat.population(),
            origin,
        },
        population: PopulationSummary {
            initial: pops[0],
            final_population,
            peak,
            peak_gen,
            min,
            min_gen,
        },
        series: sample_series(&pops, args.samples.unwrap_or(DEFAULT_SAMPLES)),
        born_total,
        died_total,
        last_step,
        extinct_gen,
        first_repeat,
        repeat_searched_to: match (tracker.repeat, tracker.stopped_at) {
            (None, Some(g)) => json!(g),
            _ => json!("complete"),
        },
        motion: tracker.motion,
        verdict,
        final_bbox: grid.bbox().map(Box2::from),
        final_rle: rle::write(&final_pattern, Some(&rule), None),
        render: args
            .render
            .unwrap_or(false)
            .then(|| render(&grid))
            .flatten(),
    })
}

/// The first index holding the "best" value by `better`.
fn first_extreme(values: &[usize], better: impl Fn(usize, usize) -> bool) -> (usize, usize) {
    values
        .iter()
        .copied()
        .enumerate()
        .fold(None, |best: Option<(usize, usize)>, (g, p)| match best {
            Some((_, b)) if !better(p, b) => best,
            _ => Some((g, p)),
        })
        .unwrap_or((0, 0))
}

/// Exact repeats of the whole grid and repeats of the live cells' shape elsewhere.
struct RepeatTracker {
    frames: HashMap<Frame, usize>,
    max_frames: usize,
    repeat: Option<(usize, usize)>,
    stopped_at: Option<usize>,
    shapes: HashMap<Pattern, (usize, usize, usize)>,
    shape_cells: usize,
    shapes_done: bool,
    motion: Option<Motion>,
}

impl RepeatTracker {
    fn new(w: usize, h: usize) -> Self {
        let words = (w * h).div_ceil(64).max(1);
        Self {
            frames: HashMap::new(),
            max_frames: (REPEAT_BUDGET_WORDS / words).max(1),
            repeat: None,
            stopped_at: None,
            shapes: HashMap::new(),
            shape_cells: 0,
            shapes_done: false,
            motion: None,
        }
    }

    fn record(&mut self, grid: &Grid, g: usize) {
        if self.repeat.is_none() && self.stopped_at.is_none() {
            if self.frames.len() >= self.max_frames {
                self.stopped_at = Some(g);
                self.frames = HashMap::new();
            } else {
                match self.frames.entry(Frame::from_grid(grid)) {
                    Entry::Occupied(e) => {
                        self.repeat = Some((*e.get(), g - e.get()));
                        self.frames = HashMap::new();
                    }
                    Entry::Vacant(e) => {
                        e.insert(g);
                    }
                }
            }
        }
        if self.shapes_done {
            return;
        }
        let Some(bb) = grid.bbox() else {
            self.shapes_done = true; // nothing alive: nothing moves
            return;
        };
        let shape = Pattern::from_grid(grid);
        self.shape_cells += shape.population();
        if self.shape_cells > SHAPE_BUDGET_CELLS {
            self.shapes_done = true;
            self.shapes = HashMap::new();
            return;
        }
        match self.shapes.entry(shape) {
            Entry::Occupied(e) => {
                let (g0, x0, y0) = *e.get();
                let (dx, dy) = (bb.x as i64 - x0 as i64, bb.y as i64 - y0 as i64);
                if (dx, dy) != (0, 0) {
                    self.motion = Some(Motion {
                        start: g0,
                        period: g - g0,
                        dx,
                        dy,
                    });
                }
                self.shapes_done = true;
                self.shapes = HashMap::new();
            }
            Entry::Vacant(e) => {
                e.insert((g, bb.x, bb.y));
            }
        }
    }
}

fn verdict(
    final_pop: usize,
    extinct: Option<usize>,
    repeat: Option<RepeatInfo>,
    motion: Option<Motion>,
    gens: usize,
) -> String {
    let base = match (final_pop, repeat) {
        (0, _) => format!("Dies out by gen {}.", extinct.unwrap_or(gens)),
        (_, Some(r)) if r.period == 1 => format!("Freezes into still lifes at gen {}.", r.start),
        (_, Some(r)) => format!(
            "Settles into a period-{} cycle at gen {}.",
            r.period, r.start
        ),
        (_, None) => format!("No repeat within {gens} gens."),
    };
    match motion {
        Some(m) if final_pop > 0 => format!(
            "A spaceship: moves ({}, {}) every {} gens from gen {}. {base}",
            m.dx, m.dy, m.period, m.start
        ),
        _ => base,
    }
}

fn render(grid: &Grid) -> Option<Render> {
    let (w, h) = (grid.width(), grid.height());
    let region = if w <= RENDER_MAX && h <= RENDER_MAX {
        Box2 {
            x: 0,
            y: 0,
            width: w,
            height: h,
        }
    } else {
        let bb = grid.bbox()?;
        if bb.w > RENDER_MAX || bb.h > RENDER_MAX {
            return None;
        }
        bb.into()
    };
    let rows = (region.y..region.y + region.height)
        .map(|y| {
            (region.x..region.x + region.width)
                .map(|x| if grid.is_alive(x, y) { 'O' } else { '.' })
                .collect()
        })
        .collect();
    Some(Render { region, rows })
}

// ---- analyze_rule ---------------------------------------------------------------------------

/// `analyze_rule`'s arguments.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct AnalyzeArgs {
    /// The rule (default B3/S23).
    #[serde(default)]
    pub rule: Option<String>,
    /// A preset key or offsets (default Moore).
    #[serde(default)]
    pub neighbourhood: Option<Value>,
    /// `torus` (default) or `dead`.
    #[serde(default)]
    pub boundary: Option<String>,
    /// `r_pentomino` / `rpent` (default), `acorn`, `soup`.
    #[serde(default)]
    pub seed: Option<String>,
    /// The soup's seed (default 7).
    #[serde(default)]
    pub soup_seed: Option<u32>,
    /// The soup's density (default 0.33).
    #[serde(default)]
    pub density: Option<f64>,
    /// Grid width (default 64).
    #[serde(default)]
    pub width: Option<usize>,
    /// Grid height (default 40).
    #[serde(default)]
    pub height: Option<usize>,
    /// Generations (default 400).
    #[serde(default)]
    pub gens: Option<usize>,
    /// Series points.
    #[serde(default)]
    pub samples: Option<usize>,
    /// Which generation's RLE to return (default the last).
    #[serde(default)]
    pub generation: Option<usize>,
}

/// A generation's cells.
#[derive(Debug, Clone, Serialize)]
pub struct GenerationRle {
    /// The generation.
    pub generation: usize,
    /// Its live cells.
    pub population: usize,
    /// The whole grid as RLE with `rule =`.
    pub rle: String,
}

/// What `analyze_rule` returns.
#[derive(Debug, Clone, Serialize)]
pub struct Analysis {
    /// The spec run.
    pub spec: SpecInfo,
    /// The Rule lab's analysis card.
    pub report: ReportInfo,
    /// `[generation, population]` samples.
    pub series: Vec<[usize; 2]>,
    /// The RLE at the asked generation.
    pub at: GenerationRle,
    /// The same spec as a rule script (`lab_set_script` takes it).
    pub script: String,
}

impl Analysis {
    /// One line for the model.
    pub fn summary(&self) -> String {
        let (s, r) = (&self.spec, &self.report);
        format!(
            "{} · {} · {} {}×{} · {} · {} gens: peak {}, final {}, first repeat {}, period {}. {}",
            s.rule.code,
            s.neighbourhood.name,
            s.boundary,
            s.width,
            s.height,
            s.seed.label,
            s.gens,
            r.peak_text,
            r.final_population,
            r.first_repeat_text,
            r.period_text,
            r.verdict
        )
    }
}

/// The run spec for `analyze_rule`'s arguments.
pub fn spec_from(args: &AnalyzeArgs) -> Result<RunSpec, ToolError> {
    let nb = neighbourhood(args.neighbourhood.as_ref())?;
    let rule = match &args.rule {
        Some(r) => input::rule_input(r, nb).map_err(ToolError::new)?,
        None => LifeRule::LIFE.with_neighbourhood(nb),
    };
    let soup_args = args.soup_seed.is_some() || args.density.is_some();
    let seed = match args.seed.as_deref() {
        None if soup_args => soup(args),
        None | Some("r_pentomino" | "rpent") => Seed::RPentomino,
        Some("acorn") => Seed::Acorn,
        Some("soup") => soup(args),
        Some(other) => {
            return Err(ToolError::new(format!(
                "unknown seed `{other}`; use r_pentomino, acorn or soup"
            )));
        }
    };
    if soup_args && !matches!(seed, Seed::Soup { .. }) {
        return Err(ToolError::new(
            "`soup_seed` and `density` only apply to `seed: soup`",
        ));
    }
    let defaults = RunSpec::default();
    let spec = RunSpec {
        rule,
        boundary: boundary(args.boundary.as_deref())?,
        w: args.width.unwrap_or(defaults.w),
        h: args.height.unwrap_or(defaults.h),
        seed,
        gens: args.gens.unwrap_or(defaults.gens),
    };
    spec.validate().map_err(|e| ToolError::new(e.to_string()))?;
    Ok(spec)
}

fn soup(args: &AnalyzeArgs) -> Seed {
    let Seed::Soup {
        seed: default_seed,
        density: default_density,
    } = Seed::MOCKUP_SOUP
    else {
        unreachable!("MOCKUP_SOUP is a soup")
    };
    Seed::Soup {
        seed: args.soup_seed.unwrap_or(default_seed),
        density: args.density.unwrap_or(default_density),
    }
}

/// Runs `analyze_rule`.
pub fn analyze(args: &AnalyzeArgs) -> Result<Analysis, ToolError> {
    let spec = spec_from(args)?;
    let run = ap_engine::run(&spec);
    let last = run.gens();
    let g = args.generation.unwrap_or(last);
    if g > last {
        return Err(ToolError::new(format!(
            "`generation` {g} is past the run's last generation ({last})"
        )));
    }
    let at = GenerationRle {
        generation: g,
        population: run.population(g).unwrap_or(0),
        rle: run.rle(g).unwrap_or_default(),
    };
    Ok(Analysis {
        spec: SpecInfo::from(&spec),
        report: ReportInfo::from(&run.report()),
        series: sample_series(run.populations(), args.samples.unwrap_or(DEFAULT_SAMPLES)),
        at,
        script: RuleScript::from_spec("analysis.rule", &spec).to_text(),
    })
}

// ---- elementary -----------------------------------------------------------------------------

/// `elementary`'s arguments.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ElementaryArgs {
    /// 0–255.
    pub rule: u8,
    /// Cells per row (default 121).
    #[serde(default)]
    pub width: Option<usize>,
    /// Rows (default 32).
    #[serde(default)]
    pub rows: Option<usize>,
    /// `single` (default) or `random`.
    #[serde(default)]
    pub start: Option<String>,
    /// Random first row seed (default 1).
    #[serde(default)]
    pub seed: Option<u64>,
}

/// What `elementary` returns.
#[derive(Debug, Clone, Serialize)]
pub struct Woven {
    /// The rule.
    pub rule: u8,
    /// Its binary, outputs for 111 → 000.
    pub binary: String,
    /// "Chaos · Random-looking" / "Uncharted rule".
    pub blurb: String,
    /// The famous thread's name, if it is one.
    pub famous: Option<&'static str>,
    /// Cells per row.
    pub width: usize,
    /// `single` or `random`.
    pub start: &'static str,
    /// The random first row's seed.
    pub seed: Option<u64>,
    /// The rows, oldest first, `#` alive, `.` dead.
    pub rows: Vec<String>,
    /// Live cells per row.
    pub counts: Vec<usize>,
    /// Live cells in all rows.
    pub total: usize,
}

impl Woven {
    /// One line for the model.
    pub fn summary(&self) -> String {
        format!(
            "Rule {} ({}, {}): {} rows × {} cells from a {} first row, {} live cells.",
            self.rule,
            self.binary,
            self.blurb,
            self.rows.len(),
            self.width,
            self.start,
            self.total
        )
    }
}

/// Runs `elementary`.
pub fn weave(args: &ElementaryArgs) -> Result<Woven, ToolError> {
    let width = args.width.unwrap_or(ap_engine::elementary::LOOM_WIDTH);
    let rows = args.rows.unwrap_or(32);
    if !(1..=ELEMENTARY_MAX_DIM).contains(&width) || !(1..=ELEMENTARY_MAX_DIM).contains(&rows) {
        return Err(ToolError::new(format!(
            "width and rows must be 1 to {ELEMENTARY_MAX_DIM}"
        )));
    }
    if width * rows > ELEMENTARY_MAX_CELLS {
        return Err(ToolError::new(format!(
            "width × rows is {}; the limit is {ELEMENTARY_MAX_CELLS} cells (e.g. 121 × 541 or \
             256 × 256)",
            width * rows
        )));
    }
    let start = match (args.start.as_deref(), args.seed) {
        (None | Some("single"), None) => WeaveStart::Single,
        (Some("single"), Some(_)) => {
            return Err(ToolError::new("`seed` only applies to `start: random`"));
        }
        (None | Some("random"), seed) => WeaveStart::Random(seed.unwrap_or(1)),
        (Some(other), _) => {
            return Err(ToolError::new(format!(
                "unknown start `{other}`; use single or random"
            )));
        }
    };
    let mut w = Weave::with_size(width, rows, args.rule, start);
    for _ in 1..rows {
        w.weave();
    }
    let rows: Vec<String> = w
        .rows()
        .map(|r| r.iter().map(|&c| if c { '#' } else { '.' }).collect())
        .collect();
    let counts: Vec<usize> = w.rows().map(|r| r.iter().filter(|&&c| c).count()).collect();
    let rule = Elementary(args.rule);
    Ok(Woven {
        rule: args.rule,
        binary: rule.binary(),
        blurb: rule.blurb(),
        famous: rule.famous().map(|f| f.name),
        width,
        start: match start {
            WeaveStart::Single => "single",
            WeaveStart::Random(_) => "random",
        },
        seed: match start {
            WeaveStart::Single => None,
            WeaveStart::Random(s) => Some(s),
        },
        total: counts.iter().sum(),
        rows,
        counts,
    })
}

// ---- parse_pattern --------------------------------------------------------------------------

/// What `parse_pattern` returns.
#[derive(Debug, Clone, Serialize)]
pub struct ParsedPattern {
    /// `stamp`, `rle` or `plaintext`.
    pub source: &'static str,
    /// The stamp id, for stamps.
    pub stamp: Option<&'static str>,
    /// `#N` / `!Name:` / the stamp's name.
    pub name: Option<String>,
    /// `#O`.
    pub author: Option<String>,
    /// `#C` / `!` comment lines.
    pub comments: Vec<String>,
    /// The header's `rule =`, verbatim.
    pub rule: Option<String>,
    /// That rule, canonical, if it parses as a Moore Life-like rule.
    pub rule_code: Option<String>,
    /// Width.
    pub width: usize,
    /// Height.
    pub height: usize,
    /// Live cells.
    pub population: usize,
    /// The canonical RLE.
    pub rle: String,
    /// The rows (`O` / `.`), for patterns up to 64 × 64.
    pub cells: Option<Vec<String>>,
}

impl ParsedPattern {
    /// One line for the model.
    pub fn summary(&self) -> String {
        let name = self
            .name
            .as_deref()
            .map(|n| format!(" \"{n}\""))
            .unwrap_or_default();
        let rule = self
            .rule_code
            .as_deref()
            .map(|r| format!(", rule {r}"))
            .unwrap_or_default();
        format!(
            "{}{name}: {}×{}, {} live cells{rule}.",
            self.source, self.width, self.height, self.population
        )
    }
}

/// Runs `parse_pattern`.
pub fn parse_pattern(text: &str) -> Result<ParsedPattern, ToolError> {
    let input = pattern(text)?;
    let f = &input.file;
    let parsed_rule = f.life_rule().and_then(Result::ok);
    let p = &f.pattern;
    let small = p.width() <= 64 && p.height() <= 64;
    Ok(ParsedPattern {
        source: input.source.key(),
        stamp: match input.source {
            PatternSource::Stamp(id) => Some(id),
            _ => None,
        },
        name: f.name.clone(),
        author: f.author.clone(),
        comments: f.comments.clone(),
        rule: f.rule.clone(),
        rule_code: parsed_rule.map(|r| r.code()),
        width: p.width(),
        height: p.height(),
        population: p.population(),
        rle: rle::write(p, parsed_rule.as_ref(), f.name.as_deref()),
        cells: small.then(|| p.to_rows()),
    })
}

// ---- list_presets ---------------------------------------------------------------------------

fn presets_json() -> Value {
    json!({
        "cartridges": CARTRIDGES.iter().map(|c| json!({
            "id": c.id, "name": c.name, "note": c.note, "rule": c.rule.code()
        })).collect::<Vec<_>>(),
        "stamps": library::STAMPS.iter().map(|s| {
            let p = s.pattern();
            json!({
                "id": s.id, "name": s.name, "width": p.width(), "height": p.height(),
                "population": p.population(), "rle": rle::write(&p, None, Some(s.name)),
            })
        }).collect::<Vec<_>>(),
        "famous_threads": elementary::FAMOUS.iter().map(|f| json!({
            "rule": f.rule, "name": f.name, "note": f.note, "binary": Elementary(f.rule).binary()
        })).collect::<Vec<_>>(),
        "named_rules": NAMED.iter().map(|(name, rule)| json!({"name": name, "rule": rule.code()}))
            .collect::<Vec<_>>(),
        "terrarium_start": presets::TERRARIUM_START.iter().map(LifeRule::code).collect::<Vec<_>>(),
        "neighbourhoods": NeighbourhoodPreset::ALL.iter()
            .map(|p| NeighbourhoodInfo::from(p.neighbourhood())).collect::<Vec<_>>(),
        "seeds": [
            {"id": "r_pentomino", "label": Seed::RPentomino.label(), "note": "centred on the grid"},
            {"id": "acorn", "label": Seed::Acorn.label(), "note": "centred on the grid"},
            {"id": "soup", "label": "Soup", "note": "random cells in the central box; soup_seed (default 7), density (default 0.33)"}
        ],
        "boundaries": Boundary::ALL.iter().map(|b| json!({
            "id": b.key(), "label": b.label(), "description": b.description()
        })).collect::<Vec<_>>(),
        "tray": {"width": presets::TRAY_W, "height": presets::TRAY_H, "boundary": presets::TRAY_BOUNDARY.key()},
    })
}

// ---- rule_script ----------------------------------------------------------------------------

/// What `rule_script` returns.
#[derive(Debug, Clone, Serialize)]
pub struct ScriptCheck {
    /// Whether it parses.
    pub valid: bool,
    /// Every error, sorted by line.
    pub errors: Vec<ScriptErrorInfo>,
    /// The canonical text (valid scripts).
    pub canonical: Option<String>,
    /// Whether the canonical text differs from the input (ignoring trailing whitespace).
    pub changed: Option<bool>,
    /// The spec (valid scripts).
    pub spec: Option<SpecInfo>,
    /// The run's report (valid scripts, unless `analyze: false`).
    pub report: Option<ReportInfo>,
    /// `[generation, population]` samples of the run.
    pub series: Option<Vec<[usize; 2]>>,
}

impl ScriptCheck {
    /// One line for the model.
    pub fn summary(&self) -> String {
        if !self.valid {
            let lines: Vec<String> = self
                .errors
                .iter()
                .map(|e| format!("line {}: {}", e.line, e.message))
                .collect();
            return format!(
                "The script has {} error{}:\n{}",
                self.errors.len(),
                if self.errors.len() == 1 { "" } else { "s" },
                lines.join("\n")
            );
        }
        let changed = if self.changed == Some(true) {
            " (normalised: see `canonical`)"
        } else {
            ""
        };
        match &self.report {
            Some(r) => format!("The script is valid{changed}. {}", r.verdict),
            None => format!("The script is valid{changed}."),
        }
    }
}

/// Runs `rule_script`.
pub fn check_script(text: &str, analyze: bool, samples: usize) -> ScriptCheck {
    match RuleScript::parse(text) {
        Ok(script) => {
            let canonical = script.to_text();
            let spec = script.spec();
            let run = analyze.then(|| ap_engine::run(&spec));
            let normalised = text.replace("\r\n", "\n");
            ScriptCheck {
                valid: true,
                errors: Vec::new(),
                changed: Some(normalised.trim_end() != canonical.trim_end()),
                canonical: Some(canonical),
                spec: Some(SpecInfo::from(&spec)),
                report: run.as_ref().map(|r| ReportInfo::from(&r.report())),
                series: run.map(|r| sample_series(r.populations(), samples)),
            }
        }
        Err(errors) => ScriptCheck {
            valid: false,
            errors: errors.iter().map(ScriptErrorInfo::from).collect(),
            canonical: None,
            changed: None,
            spec: None,
            report: None,
            series: None,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn life(pattern: &str) -> RunLifeArgs {
        RunLifeArgs {
            pattern: pattern.into(),
            ..RunLifeArgs::default()
        }
    }

    #[test]
    fn a_glider_moves_one_cell_diagonally_every_four_gens() {
        let r = run_life(&life("glider"), &|| false).unwrap();
        let m = r.motion.unwrap();
        assert_eq!((m.period, m.dx, m.dy), (4, 1, 1));
        assert_eq!(r.population.final_population, 5);
        assert!(
            r.first_repeat.is_none(),
            "a 64×40 torus wraps after 1280 gens"
        );
        assert!(
            r.verdict
                .starts_with("A spaceship: moves (1, 1) every 4 gens")
        );
    }

    #[test]
    fn a_blinker_has_period_two() {
        let r = run_life(&life("OOO"), &|| false).unwrap();
        let rep = r.first_repeat.unwrap();
        assert_eq!((rep.start, rep.period), (0, 2));
        assert!(r.motion.is_none());
        assert_eq!(r.verdict, "Settles into a period-2 cycle at gen 0.");
    }

    #[test]
    fn rules_come_from_the_header_unless_given() {
        let r = run_life(&life("x = 3, y = 1, rule = B2/S\n3o!"), &|| false).unwrap();
        assert_eq!((r.rule.code.as_str(), r.rule_source), ("B2/S", "pattern"));
        let args = RunLifeArgs {
            rule: Some("Life".into()),
            ..life("x = 3, y = 1, rule = B2/S\n3o!")
        };
        assert_eq!(run_life(&args, &|| false).unwrap().rule_source, "argument");
    }

    #[test]
    fn grids_grow_to_fit_unless_sized() {
        let rows = ["O".repeat(90)];
        let r = run_life(
            &RunLifeArgs {
                gens: Some(1),
                ..life(&rows.join("\n"))
            },
            &|| false,
        )
        .unwrap();
        assert_eq!((r.width, r.height), (106, 40));
        let e = run_life(
            &RunLifeArgs {
                width: Some(50),
                ..life(&rows.join("\n"))
            },
            &|| false,
        )
        .unwrap_err();
        assert!(e.message.contains("doesn't fit a grid width of 50"), "{e}");
    }

    #[test]
    fn the_rule_lab_report() {
        let a = analyze(&AnalyzeArgs::default()).unwrap();
        assert_eq!(a.report.peak_text, "221 @ 148");
        assert_eq!(a.report.final_population, 46);
        assert_eq!(a.report.period, Some(2));
        assert_eq!(
            a.report.verdict,
            "Settles into a period-2 cycle at gen 388."
        );
        assert_eq!(
            RuleScript::parse(&a.script).unwrap().spec(),
            RunSpec::default()
        );
    }

    #[test]
    fn neighbourhood_arguments() {
        assert_eq!(
            neighbourhood(Some(&json!("vn"))).unwrap(),
            Neighbourhood::VON_NEUMANN
        );
        assert_eq!(
            neighbourhood(Some(&json!([[0, -1], [-1, 0], [1, 0], [0, 1]]))).unwrap(),
            Neighbourhood::VON_NEUMANN
        );
        assert!(neighbourhood(Some(&json!([[0, 0]]))).is_err());
        assert!(neighbourhood(Some(&json!([[3, 0]]))).is_err());
        assert!(neighbourhood(Some(&json!([[1]]))).is_err());
        assert!(neighbourhood(Some(&json!("square"))).is_err());
    }

    #[test]
    fn rule_30_and_90() {
        let w = weave(&ElementaryArgs {
            rule: 90,
            rows: Some(8),
            ..ElementaryArgs::default()
        })
        .unwrap();
        // Gould's sequence: 2^popcount(n)
        assert_eq!(w.counts, [1, 2, 2, 4, 2, 4, 4, 8]);
        let w = weave(&ElementaryArgs {
            rule: 30,
            ..ElementaryArgs::default()
        })
        .unwrap();
        assert_eq!(w.blurb, "Chaos · Random-looking");
        assert_eq!(w.rows.len(), 32);
        assert_eq!(w.rows[0].len(), 121);
    }
}
