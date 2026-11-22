//! The Automata bridge methods (beyond `status` / `navigate` / `screenshot` / `describe`, which
//! every lab-series app answers; see `lab_mcp::bridge::proto`) and their payloads.
//!
//! | method | params | result |
//! |---|---|---|
//! | [`TRAY_GET`] | none | [`TrayState`] |
//! | [`TRAY_SET_RULE`] | [`TraySetRule`] | [`TrayState`] |
//! | [`TRAY_LOAD_PATTERN`] | [`TrayLoadPattern`] | [`TrayState`] |
//! | [`TRAY_CONTROL`] | [`TrayControl`] | [`TrayState`] |
//! | [`LOOM_GET`] | [`LoomGet`] | [`LoomState`] |
//! | [`LOOM_SET`] | [`LoomSet`] | [`LoomState`] |
//! | [`TERRARIUM_GET`] | none | [`TerrariumState`] |
//! | [`TERRARIUM_KEEP`] | [`TerrariumKeep`] | [`TerrariumState`] |
//! | [`TERRARIUM_BREED`] | [`TerrariumBreed`] | [`TerrariumState`] |
//! | [`LAB_GET`] | [`LabGet`] | [`LabState`] |
//! | [`LAB_SET_SCRIPT`] | [`LabSetScript`] | [`LabState`] |
//! | [`LAB_SCRUB`] | [`LabScrub`] | [`LabState`] |
//!
//! Methods that change a screen act on its state whether or not it is showing; with `show`
//! (the default) the app also switches to that screen so the person at the keyboard sees the
//! change. A bad argument (a rule that doesn't parse, a jar that doesn't exist, a script with
//! errors) is an `INVALID_PARAMS` error reply whose message says what to send instead, and
//! nothing changes.

use ap_engine::{Boundary, LifeRule, Neighbourhood, Report, RunSpec, Seed};
use serde::{Deserialize, Serialize};

/// The Tray's state.
pub const TRAY_GET: &str = "tray.get";
/// Sets the Tray's rule (the cells stay).
pub const TRAY_SET_RULE: &str = "tray.set_rule";
/// Clears the Tray and places a pattern centred on it.
pub const TRAY_LOAD_PATTERN: &str = "tray.load_pattern";
/// Run / pause / step / clear / restart / randomize the Tray.
pub const TRAY_CONTROL: &str = "tray.control";
/// The Loom's state.
pub const LOOM_GET: &str = "loom.get";
/// Changes the Loom: rule, first row, run / pause, steps, speed.
pub const LOOM_SET: &str = "loom.set";
/// The Terrarium's state.
pub const TERRARIUM_GET: &str = "terrarium.get";
/// Keeps (or un-keeps) jars.
pub const TERRARIUM_KEEP: &str = "terrarium.keep";
/// Breeds the next brood.
pub const TERRARIUM_BREED: &str = "terrarium.breed";
/// The Rule lab's state.
pub const LAB_GET: &str = "lab.get";
/// Replaces the Rule lab's script.
pub const LAB_SET_SCRIPT: &str = "lab.set_script";
/// Scrubs the Rule lab to a generation.
pub const LAB_SCRUB: &str = "lab.scrub";

/// Every Automata method, in table order.
pub const METHODS: [&str; 12] = [
    TRAY_GET,
    TRAY_SET_RULE,
    TRAY_LOAD_PATTERN,
    TRAY_CONTROL,
    LOOM_GET,
    LOOM_SET,
    TERRARIUM_GET,
    TERRARIUM_KEEP,
    TERRARIUM_BREED,
    LAB_GET,
    LAB_SET_SCRIPT,
    LAB_SCRUB,
];

/// The most generations / rows / ticks one request may advance.
pub const MAX_STEPS: u32 = 10_000;

fn yes() -> bool {
    true
}

// ---- params ---------------------------------------------------------------------------------

/// Params of [`TRAY_SET_RULE`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TraySetRule {
    /// A rulestring (`B36/S23`) or a name (`HighLife`); Moore neighbourhood.
    pub rule: String,
    /// Switch to the Tray (default `true`).
    #[serde(default = "yes")]
    pub show: bool,
}

/// Params of [`TRAY_LOAD_PATTERN`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrayLoadPattern {
    /// RLE, plaintext or a stamp id (see [`crate::input::pattern_input`]).
    pub pattern: String,
    /// A rule to set as well. Without it, a valid `rule =` in the RLE header is used.
    #[serde(default)]
    pub rule: Option<String>,
    /// Switch to the Tray (default `true`).
    #[serde(default = "yes")]
    pub show: bool,
}

/// What [`TRAY_CONTROL`] does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TrayAction {
    /// Run.
    Run,
    /// Pause.
    Pause,
    /// Pause, then advance `n` generations (default 1).
    Step,
    /// Kill every cell (the rule stays).
    Clear,
    /// The starting garden under Life.
    Restart,
    /// A 30 % random soup from `seed` (the rule stays).
    Randomize,
}

/// Params of [`TRAY_CONTROL`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrayControl {
    /// What to do.
    pub action: TrayAction,
    /// Generations for `step` (1–[`MAX_STEPS`], default 1).
    #[serde(default)]
    pub n: Option<u32>,
    /// Seed for `randomize` (default: drawn by the app).
    #[serde(default)]
    pub seed: Option<u64>,
    /// Also set the speed (1–30 gen/s).
    #[serde(default)]
    pub speed: Option<f32>,
    /// Switch to the Tray (default `true`).
    #[serde(default = "yes")]
    pub show: bool,
}

/// Params of [`LOOM_GET`].
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LoomGet {
    /// How many of the newest rows to include (0–72, default 24).
    #[serde(default)]
    pub rows: Option<usize>,
}

/// A thread's first row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FirstRow {
    /// One live cell in the middle.
    Single,
    /// Each cell alive with probability ½.
    Random,
}

/// Params of [`LOOM_SET`]; every field is optional and they apply in this order: `rule`,
/// `start` (+ `seed`), `restart`, `speed`, `running`, `step`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LoomSet {
    /// The rule, 0–255 (restarts the thread).
    #[serde(default)]
    pub rule: Option<u8>,
    /// The first row (restarts the thread).
    #[serde(default)]
    pub start: Option<FirstRow>,
    /// The random first row's seed (implies `start: random`).
    #[serde(default)]
    pub seed: Option<u64>,
    /// Start the thread over (a random start draws a new row, unless `seed` names it).
    #[serde(default)]
    pub restart: bool,
    /// Rows per second (2–40).
    #[serde(default)]
    pub speed: Option<f32>,
    /// Weave (`true`) or pause.
    #[serde(default)]
    pub running: Option<bool>,
    /// Weave this many rows now (1–[`MAX_STEPS`]).
    #[serde(default)]
    pub step: Option<u32>,
    /// Rows to include in the reply (see [`LoomGet::rows`]).
    #[serde(default)]
    pub rows: Option<usize>,
    /// Switch to the Loom (default `true`).
    #[serde(default = "yes")]
    pub show: bool,
}

/// Params of [`TERRARIUM_KEEP`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TerrariumKeep {
    /// Jar numbers as on screen, 1–6.
    pub jars: Vec<usize>,
    /// Keep (`true`, the default) or un-keep.
    #[serde(default = "yes")]
    pub kept: bool,
    /// Switch to the Terrarium (default `true`).
    #[serde(default = "yes")]
    pub show: bool,
}

/// Params of [`TERRARIUM_BREED`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TerrariumBreed {
    /// Bit flips per child, 1–4 (default: the screen's setting).
    #[serde(default)]
    pub mutation: Option<u8>,
    /// Then tick every jar this many times (0–[`MAX_STEPS`], default 0), so liveliness and
    /// populations mean something.
    #[serde(default)]
    pub ticks: Option<u32>,
    /// Switch to the Terrarium (default `true`).
    #[serde(default = "yes")]
    pub show: bool,
}

/// Params of [`LAB_GET`].
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LabGet {
    /// Points in the population series (2–2001, default 41).
    #[serde(default)]
    pub samples: Option<usize>,
}

/// Params of [`LAB_SET_SCRIPT`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LabSetScript {
    /// The whole script.
    pub script: String,
    /// Points in the population series of the reply.
    #[serde(default)]
    pub samples: Option<usize>,
    /// Switch to the Rule lab (default `true`).
    #[serde(default = "yes")]
    pub show: bool,
}

/// Params of [`LAB_SCRUB`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LabScrub {
    /// The generation (clamped to the run).
    pub generation: usize,
    /// Points in the population series of the reply.
    #[serde(default)]
    pub samples: Option<usize>,
    /// Switch to the Rule lab (default `true`).
    #[serde(default = "yes")]
    pub show: bool,
}

// ---- results --------------------------------------------------------------------------------

/// The Tray's callout.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MoodInfo {
    /// "Churning", "Settled", ….
    pub title: String,
    /// Its sentence.
    pub text: String,
}

/// What [`TRAY_GET`] and the Tray's other methods return.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrayState {
    /// The rule code (`B3/S23`).
    pub rule: String,
    /// The rule panel's name: the cartridge's ("Life") or "Custom rule".
    pub rule_name: String,
    /// The lit cartridge's id (`life`), if the rule is one.
    pub cartridge: Option<String>,
    /// Generations since the last restart / clear / soup / load (the `gen 0000` counter).
    pub generation: u64,
    /// Live cells.
    pub population: usize,
    /// Born in the last step.
    pub born: usize,
    /// Died in the last step.
    pub died: usize,
    /// The "what is happening" callout.
    pub mood: MoodInfo,
    /// Running (`true`) or paused.
    pub running: bool,
    /// Generations per second while running.
    pub speed: f32,
    /// Tray width (40).
    pub width: usize,
    /// Tray height (26).
    pub height: usize,
    /// The sparkline: the last ≤ 120 populations, oldest first.
    pub history: Vec<usize>,
    /// The whole tray as RLE with `rule =` (what Share copies).
    pub rle: String,
    /// The canvas description ("Cell tray, 40 by 26, generation 30, 90 alive").
    pub description: String,
    /// What the request did, when it's worth saying ("cropped to 40 × 26", "seed 12").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// A famous thread.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FamousInfo {
    /// "Chaos".
    pub name: String,
    /// "Random-looking".
    pub note: String,
}

/// What [`LOOM_GET`] and [`LOOM_SET`] return.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LoomState {
    /// The rule, 0–255.
    pub rule: u8,
    /// Its 8-bit binary, outputs for 111 → 000.
    pub binary: String,
    /// "Chaos · Random-looking" or "Uncharted rule".
    pub blurb: String,
    /// The famous thread, if it is one.
    pub famous: Option<FamousInfo>,
    /// `single` or `random`.
    pub start: FirstRow,
    /// The random first row's seed.
    pub seed: Option<u64>,
    /// Weaving (`true`) or paused.
    pub running: bool,
    /// Rows per second.
    pub speed: f32,
    /// Rows woven since the start, the first included (the `row 0000` counter).
    pub row: u64,
    /// Cells per row (121).
    pub width: usize,
    /// Rows on the board now (at most 72).
    pub visible_rows: usize,
    /// Rows filled / 72.
    pub progress: f64,
    /// The newest rows (oldest first), `#` alive and `.` dead.
    pub rows: Vec<String>,
    /// The fabric's description.
    pub description: String,
    /// What the request did, when it's worth saying.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// One jar.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JarState {
    /// 1–6, as on screen.
    pub number: usize,
    /// The rule code.
    pub rule: String,
    /// The rule's name or "Mutant".
    pub name: String,
    /// Live cells (of 32 × 32).
    pub population: usize,
    /// The liveliness meter, 0–1.
    pub liveliness: f64,
    /// `quiet`, `lively` or `frantic` (the meter's colour).
    pub level: String,
    /// Kept as a parent.
    pub kept: bool,
}

/// One lineage entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LineageInfo {
    /// The brood it produced.
    pub brood: u32,
    /// Its parents' rule codes.
    pub parents: Vec<String>,
}

/// What the Terrarium's methods return.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TerrariumState {
    /// The brood number (1 before any breeding).
    pub brood: u32,
    /// Bit flips per child.
    pub mutation: u8,
    /// Running (`true`) or paused.
    pub running: bool,
    /// The six jars.
    pub jars: Vec<JarState>,
    /// The note under the breed button.
    pub keep_note: String,
    /// The rule "Plant in the Tray" / "Analyse in Rule lab" would carry.
    pub chosen_rule: String,
    /// Newest first, last 8.
    pub lineage: Vec<LineageInfo>,
    /// What the request did, when it's worth saying.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// A neighbourhood.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NeighbourhoodInfo {
    /// `moore`, `vn`, `hex`, `moore2` or `custom`.
    pub key: String,
    /// "Moore r1", "von Neumann", … or "Custom (N cells)".
    pub name: String,
    /// Number of neighbours.
    pub cells: usize,
    /// The `(dx, dy)` offsets, row-major.
    pub offsets: Vec<[i32; 2]>,
}

impl From<Neighbourhood> for NeighbourhoodInfo {
    fn from(nb: Neighbourhood) -> Self {
        NeighbourhoodInfo {
            key: nb.key().to_owned(),
            name: nb.name(),
            cells: nb.len(),
            offsets: nb.offsets().map(|(x, y)| [x, y]).collect(),
        }
    }
}

/// A rule spelled out.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuleInfo {
    /// The canonical code (`B36/S23`).
    pub code: String,
    /// Birth counts.
    pub birth: Vec<usize>,
    /// Survival counts.
    pub survive: Vec<usize>,
    /// The rule's name ("HighLife"), for named Moore rules.
    pub name: Option<String>,
}

impl From<&LifeRule> for RuleInfo {
    fn from(rule: &LifeRule) -> Self {
        RuleInfo {
            code: rule.code(),
            birth: rule.birth().iter().collect(),
            survive: rule.survive().iter().collect(),
            name: ap_engine::presets::rule_name(rule).map(str::to_owned),
        }
    }
}

/// A Rule-lab seed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SeedInfo {
    /// `r_pentomino`, `acorn` or `soup`.
    pub kind: String,
    /// "R-pentomino", "Acorn", "Soup".
    pub label: String,
    /// The soup's LCG seed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub soup_seed: Option<u32>,
    /// The soup's density.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub density: Option<f64>,
}

impl From<&Seed> for SeedInfo {
    fn from(seed: &Seed) -> Self {
        let (soup_seed, density) = match *seed {
            Seed::Soup { seed, density } => (Some(seed), Some(density)),
            _ => (None, None),
        };
        SeedInfo {
            kind: seed.key().to_owned(),
            label: seed.label().to_owned(),
            soup_seed,
            density,
        }
    }
}

/// A Rule-lab run spec.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpecInfo {
    /// The rule.
    pub rule: RuleInfo,
    /// Its neighbourhood.
    pub neighbourhood: NeighbourhoodInfo,
    /// `torus` or `dead`.
    pub boundary: String,
    /// Grid width.
    pub width: usize,
    /// Grid height.
    pub height: usize,
    /// The starting cells.
    pub seed: SeedInfo,
    /// Generations run.
    pub gens: usize,
}

impl From<&RunSpec> for SpecInfo {
    fn from(spec: &RunSpec) -> Self {
        SpecInfo {
            rule: RuleInfo::from(&spec.rule),
            neighbourhood: spec.rule.neighbourhood().into(),
            boundary: boundary_key(spec.boundary).to_owned(),
            width: spec.w,
            height: spec.h,
            seed: SeedInfo::from(&spec.seed),
            gens: spec.gens,
        }
    }
}

fn boundary_key(b: Boundary) -> &'static str {
    b.key()
}

/// The first repeat of a run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepeatInfo {
    /// The generation the cycle starts at.
    pub start: usize,
    /// Its length (1 = still lifes).
    pub period: usize,
    /// Where the repeat is seen, `start + period` (the "First repeat" tile).
    pub seen_at: usize,
}

/// The Rule lab's analysis card (LAB-3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReportInfo {
    /// The highest population.
    pub peak: usize,
    /// The first generation reaching it.
    pub peak_gen: usize,
    /// `"221 @ 148"`.
    pub peak_text: String,
    /// The population at the last generation.
    pub final_population: usize,
    /// The first exact repeat, if any.
    pub first_repeat: Option<RepeatInfo>,
    /// `"gen 390"` or `"none"`.
    pub first_repeat_text: String,
    /// The cycle length.
    pub period: Option<usize>,
    /// `"2"` or `"—"`.
    pub period_text: String,
    /// The first generation with no live cells.
    pub extinct_gen: Option<usize>,
    /// "Settles into a period-2 cycle at gen 388." etc.
    pub verdict: String,
}

impl From<&Report> for ReportInfo {
    fn from(r: &Report) -> Self {
        ReportInfo {
            peak: r.peak,
            peak_gen: r.peak_gen,
            peak_text: r.peak_text(),
            final_population: r.final_pop,
            first_repeat: r.first_repeat.map(|f| RepeatInfo {
                start: f.start,
                period: f.period,
                seen_at: f.seen_at(),
            }),
            first_repeat_text: r.first_repeat_text(),
            period: r.period,
            period_text: r.period_text(),
            extinct_gen: r.extinct_gen,
            verdict: r.verdict.clone(),
        }
    }
}

/// A script error.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScriptErrorInfo {
    /// 1-based line.
    pub line: usize,
    /// What is wrong.
    pub message: String,
}

impl From<&ap_engine::ScriptError> for ScriptErrorInfo {
    fn from(e: &ap_engine::ScriptError) -> Self {
        ScriptErrorInfo {
            line: e.line,
            message: e.message.clone(),
        }
    }
}

/// What the Rule lab's methods return.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LabState {
    /// The script as shown in the editor (it may have errors).
    pub script: String,
    /// The editor text's errors (empty when it parses).
    pub errors: Vec<ScriptErrorInfo>,
    /// The spec in effect (the last valid script's).
    pub spec: SpecInfo,
    /// The scrubbed generation.
    pub generation: usize,
    /// Live cells at the scrubbed generation.
    pub alive: usize,
    /// Play is on.
    pub playing: bool,
    /// The analysis.
    pub report: ReportInfo,
    /// `[generation, population]` samples of the run.
    pub series: Vec<[usize; 2]>,
    /// The scrubbed generation as RLE with `rule =`.
    pub rle: String,
    /// The screen's one-line description.
    pub description: String,
    /// What the request did, when it's worth saying.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// The default number of points in a population series.
pub const DEFAULT_SAMPLES: usize = 41;

/// `samples` evenly spaced `[index, value]` points of `values`, always including the first and
/// the last (all of them when there are no more than `samples`).
pub fn sample_series(values: &[usize], samples: usize) -> Vec<[usize; 2]> {
    let n = values.len();
    let samples = samples.max(2);
    if n <= samples {
        return values
            .iter()
            .copied()
            .enumerate()
            .map(|(i, v)| [i, v])
            .collect();
    }
    let mut out: Vec<[usize; 2]> = (0..samples)
        .map(|k| {
            let i = k * (n - 1) / (samples - 1);
            [i, values[i]]
        })
        .collect();
    out.dedup_by_key(|p| p[0]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn params_default_to_showing() {
        let p: TraySetRule = serde_json::from_str(r#"{"rule":"B3/S23"}"#).unwrap();
        assert!(p.show);
        let c: TrayControl = serde_json::from_str(r#"{"action":"step","n":3}"#).unwrap();
        assert_eq!((c.action, c.n, c.show), (TrayAction::Step, Some(3), true));
    }

    #[test]
    fn series_samples_include_both_ends() {
        let v: Vec<usize> = (0..=400).collect();
        let s = sample_series(&v, 41);
        assert_eq!(s.len(), 41);
        assert_eq!(s[0], [0, 0]);
        assert_eq!(s[1], [10, 10]);
        assert_eq!(s[40], [400, 400]);
        assert_eq!(sample_series(&[5, 6], 41), vec![[0, 5], [1, 6]]);
        assert!(sample_series(&[], 10).is_empty());
    }

    #[test]
    fn spec_and_report_info() {
        let spec = RunSpec::default();
        let info = SpecInfo::from(&spec);
        assert_eq!(info.rule.code, "B3/S23");
        assert_eq!(info.rule.name.as_deref(), Some("Life"));
        assert_eq!(info.neighbourhood.cells, 8);
        assert_eq!(info.seed.kind, "r_pentomino");
        let r = ReportInfo::from(&ap_engine::run(&spec).report());
        assert_eq!(r.peak_text, "221 @ 148");
        assert_eq!(r.first_repeat.unwrap().seen_at, 390);
    }
}
