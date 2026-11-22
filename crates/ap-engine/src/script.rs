//! The Rule lab's script, the netlist of Advanced mode (SPEC LAB-4, ENG-7).
//!
//! ```text
//! * automata netlist · glider-garden.rule
//! .rule   life_like  B3/S23
//! .neigh  moore  n=8
//! .grid   64x40  boundary=torus
//! .seed   r_pentomino @ (32,20)
//! .run    gens=400
//! .probe  population  -> plot1
//! .probe  period      -> report
//! .end
//! ```
//!
//! [`RuleScript::to_text`] prints exactly this format. [`RuleScript::parse`] is lenient about
//! spacing and case (`.RULE Life_Like b3s23`, `gens = 400`, `@(32, 20)` all work), takes `*`
//! comment lines anywhere, and reports every problem with its line number. Every directive but
//! `.probe` and `.end` is required, and each at most once.
//!
//! Custom neighbourhoods list their offsets after the count, row-major:
//! `.neigh  custom  n=3  (0,-1) (-1,0) (2,2)` — offsets are `(dx,dy)` in −2…2, never `(0,0)`.
//! A custom list equal to a preset prints as that preset. Stamp seeds (`r_pentomino`, `acorn`)
//! always sit at the grid's centre, so `@ (x,y)` is optional and, when given, must be the centre.
//! A soup's `seed=` and `density=` default to the mockup's 7 and 0.33.

use std::fmt;

use thiserror::Error;

use crate::analysis::{MAX_RUN_DIM, MAX_RUN_GENS, RunSpec, Seed};
use crate::rule::{Boundary, LifeRule, Neighbourhood, NeighbourhoodPreset};

/// The document name the mockup uses.
pub const DEFAULT_NAME: &str = "glider-garden.rule";
/// The name a script without a `* automata netlist · <name>` header gets.
pub const UNTITLED_NAME: &str = "untitled.rule";

/// What a probe reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProbeQuantity {
    /// The population per generation (`population`).
    Population,
    /// The cycle length (`period`).
    Period,
    /// The peak population (`peak`).
    Peak,
    /// The final population (`final`).
    Final,
    /// The first repeat's generation (`first_repeat`).
    FirstRepeat,
    /// The verdict sentence (`verdict`).
    Verdict,
    /// The scrubbed generation as RLE (`rle`).
    Rle,
}

impl ProbeQuantity {
    /// Every quantity.
    pub const ALL: [ProbeQuantity; 7] = [
        ProbeQuantity::Population,
        ProbeQuantity::Period,
        ProbeQuantity::Peak,
        ProbeQuantity::Final,
        ProbeQuantity::FirstRepeat,
        ProbeQuantity::Verdict,
        ProbeQuantity::Rle,
    ];

    /// The script keyword.
    pub const fn key(self) -> &'static str {
        match self {
            ProbeQuantity::Population => "population",
            ProbeQuantity::Period => "period",
            ProbeQuantity::Peak => "peak",
            ProbeQuantity::Final => "final",
            ProbeQuantity::FirstRepeat => "first_repeat",
            ProbeQuantity::Verdict => "verdict",
            ProbeQuantity::Rle => "rle",
        }
    }

    /// Looks a keyword up, case-insensitively.
    pub fn from_key(key: &str) -> Option<ProbeQuantity> {
        let key = key.to_ascii_lowercase();
        ProbeQuantity::ALL.into_iter().find(|q| q.key() == key)
    }
}

/// `.probe <quantity> -> <target>`: where a measured quantity is shown.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Probe {
    /// What is measured.
    pub quantity: ProbeQuantity,
    /// Where it goes (`plot1`, `report`, …): letters, digits, `_`, `-`, `.`.
    pub target: String,
}

impl Probe {
    /// A probe.
    pub fn new(quantity: ProbeQuantity, target: impl Into<String>) -> Probe {
        Probe {
            quantity,
            target: target.into(),
        }
    }

    /// The mockup's two probes: population → plot1, period → report.
    pub fn defaults() -> Vec<Probe> {
        vec![
            Probe::new(ProbeQuantity::Population, "plot1"),
            Probe::new(ProbeQuantity::Period, "report"),
        ]
    }
}

/// A problem in a script, on a 1-based line.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("line {line}: {message}")]
pub struct ScriptError {
    /// 1-based line number (for a missing directive: the `.end` line, else the last line).
    pub line: usize,
    /// What is wrong and how to fix it.
    pub message: String,
}

impl ScriptError {
    fn new(line: usize, message: impl Into<String>) -> ScriptError {
        ScriptError {
            line,
            message: message.into(),
        }
    }
}

/// A parsed rule script: the whole Rule-lab spec plus the document name and the probes.
#[derive(Debug, Clone, PartialEq)]
pub struct RuleScript {
    /// The document name from the header comment (`glider-garden.rule`). One line.
    pub name: String,
    /// The rule and its neighbourhood (`.rule` + `.neigh`).
    pub rule: LifeRule,
    /// Grid width (`.grid WxH`).
    pub w: usize,
    /// Grid height.
    pub h: usize,
    /// `boundary=`.
    pub boundary: Boundary,
    /// `.seed`.
    pub seed: Seed,
    /// `.run gens=`.
    pub gens: usize,
    /// The `.probe` lines, in order.
    pub probes: Vec<Probe>,
}

impl Default for RuleScript {
    /// The mockup's script (`glider-garden.rule`, B3/S23, Moore, 64 × 40 torus, R-pentomino,
    /// 400 gens, the two default probes).
    fn default() -> Self {
        RuleScript::from_spec(DEFAULT_NAME, &RunSpec::default())
    }
}

impl RuleScript {
    /// A script for `spec`, with the default probes.
    pub fn from_spec(name: impl Into<String>, spec: &RunSpec) -> RuleScript {
        RuleScript {
            name: name.into(),
            rule: spec.rule,
            w: spec.w,
            h: spec.h,
            boundary: spec.boundary,
            seed: spec.seed,
            gens: spec.gens,
            probes: Probe::defaults(),
        }
    }

    /// The run this script describes.
    pub fn spec(&self) -> RunSpec {
        RunSpec {
            rule: self.rule,
            boundary: self.boundary,
            w: self.w,
            h: self.h,
            seed: self.seed,
            gens: self.gens,
        }
    }

    /// Replaces the spec part (editing the spec rewrites the text), keeping name and probes.
    pub fn set_spec(&mut self, spec: &RunSpec) {
        self.rule = spec.rule;
        self.w = spec.w;
        self.h = spec.h;
        self.boundary = spec.boundary;
        self.seed = spec.seed;
        self.gens = spec.gens;
    }

    /// Prints the script in the mockup's format (no trailing newline).
    pub fn to_text(&self) -> String {
        let nb = self.rule.neighbourhood();
        let mut neigh = format!("{}  n={}", nb.key(), nb.len());
        if nb.preset().is_none() && !nb.is_empty() {
            let offsets: Vec<String> = nb
                .offsets()
                .map(|(dx, dy)| format!("({dx},{dy})"))
                .collect();
            neigh.push_str("  ");
            neigh.push_str(&offsets.join(" "));
        }
        let (cx, cy) = Seed::centre(self.w, self.h);
        let seed = match self.seed {
            Seed::RPentomino => format!("r_pentomino @ ({cx},{cy})"),
            Seed::Acorn => format!("acorn @ ({cx},{cy})"),
            Seed::Soup { seed, density } => format!("soup seed={seed} density={density}"),
        };
        let mut lines = vec![
            format!(
                "* automata netlist · {}",
                self.name.replace(['\r', '\n'], " ").trim()
            ),
            format!(".rule   life_like  {}", self.rule.code()),
            format!(".neigh  {neigh}"),
            format!(
                ".grid   {}x{}  boundary={}",
                self.w,
                self.h,
                self.boundary.key()
            ),
            format!(".seed   {seed}"),
            format!(".run    gens={}", self.gens),
        ];
        for p in &self.probes {
            lines.push(format!(".probe  {:<10}  -> {}", p.quantity.key(), p.target));
        }
        lines.push(".end".to_string());
        lines.join("\n")
    }

    /// Parses a script; on failure returns every problem found, sorted by line.
    pub fn parse(text: &str) -> Result<RuleScript, Vec<ScriptError>> {
        Parser::default().run(text)
    }
}

impl fmt::Display for RuleScript {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_text())
    }
}

#[derive(Clone, Copy)]
enum SeedKind {
    RPentomino,
    Acorn,
}

/// A directive seen on a line, with its value (`None` when the line had errors).
type Slot<T> = Option<(usize, Option<T>)>;

#[derive(Default)]
struct Parser {
    errors: Vec<ScriptError>,
    name: Option<String>,
    rule: Option<(usize, String)>,
    neigh: Slot<Neighbourhood>,
    grid: Slot<(usize, usize, Boundary)>,
    seed: Slot<(Seed, Option<(i64, i64)>)>,
    gens: Slot<usize>,
    probes: Vec<Probe>,
    end: Option<usize>,
    last_line: usize,
}

const DIRECTIVES: &str = ".rule, .neigh, .grid, .seed, .run, .probe or .end";

impl Parser {
    fn err(&mut self, line: usize, message: impl Into<String>) {
        self.errors.push(ScriptError::new(line, message));
    }

    /// Reports a duplicate and returns `true` if `first` is already set.
    fn duplicate(&mut self, line: usize, directive: &str, first: Option<usize>) -> bool {
        if let Some(first) = first {
            self.err(
                line,
                format!("{directive} is given twice (first on line {first})"),
            );
            true
        } else {
            false
        }
    }

    fn run(mut self, text: &str) -> Result<RuleScript, Vec<ScriptError>> {
        let text = text.strip_prefix('\u{feff}').unwrap_or(text);
        for (index, raw) in text.lines().enumerate() {
            let line_no = index + 1;
            self.last_line = line_no;
            let line = raw.trim();
            if line.is_empty() {
                continue;
            }
            if let Some(comment) = line.strip_prefix('*') {
                self.comment(comment);
                continue;
            }
            if let Some(end) = self.end {
                self.err(
                    line_no,
                    format!("nothing may follow .end (line {end}); move this line above it"),
                );
                continue;
            }
            let tokens = tokens(line);
            let Some((directive, args)) = tokens.split_first() else {
                continue;
            };
            let args: Vec<&str> = args.iter().map(String::as_str).collect();
            match directive.to_ascii_lowercase().as_str() {
                ".rule" => self.rule(line_no, &args),
                ".neigh" => self.neigh(line_no, &args),
                ".grid" => self.grid(line_no, &args),
                ".seed" => self.seed(line_no, &args),
                ".run" => self.gens(line_no, &args),
                ".probe" => self.probe(line_no, &args),
                ".end" => {
                    if !args.is_empty() {
                        self.err(line_no, ".end takes nothing after it");
                    }
                    self.end = Some(line_no);
                }
                d if d.starts_with('.') => {
                    self.err(line_no, format!("unknown directive `{directive}` (expected {DIRECTIVES})"))
                }
                _ => self.err(
                    line_no,
                    format!("`{directive}` isn't a directive; lines start with {DIRECTIVES}, or `*` for a comment"),
                ),
            }
        }
        self.finish()
    }

    fn comment(&mut self, comment: &str) {
        if self.name.is_some() {
            return;
        }
        let c = comment.trim();
        let prefix = "automata netlist";
        if c.len() >= prefix.len()
            && c.is_char_boundary(prefix.len())
            && c[..prefix.len()].eq_ignore_ascii_case(prefix)
        {
            let rest = c[prefix.len()..].trim_start();
            let rest = rest.strip_prefix(['·', '-', ':', '|']).unwrap_or(rest);
            self.name = Some(rest.trim().to_string());
        }
    }

    fn rule(&mut self, line: usize, args: &[&str]) {
        if self.duplicate(line, ".rule", self.rule.as_ref().map(|r| r.0)) {
            return;
        }
        let Some((family, code)) = args.split_first() else {
            self.err(
                line,
                "expected a family and a rule, e.g. `.rule  life_like  B3/S23`",
            );
            self.rule = Some((line, String::new()));
            return;
        };
        match family.to_ascii_lowercase().replace('-', "_").as_str() {
            "life_like" | "lifelike" => {}
            "generations" => return self.unsupported(line, "Generations"),
            "larger_than_life" | "ltl" => return self.unsupported(line, "Larger than Life"),
            "lenia" => return self.unsupported(line, "Lenia"),
            _ if LifeRule::parse_with(family, Neighbourhood::MOORE2).is_ok() && code.is_empty() => {
                self.err(
                    line,
                    format!("missing the family: write `.rule  life_like  {family}`"),
                );
                self.rule = Some((line, String::new()));
                return;
            }
            _ => {
                self.err(
                    line,
                    format!("unknown rule family `{family}` (only life_like is available)"),
                );
                self.rule = Some((line, String::new()));
                return;
            }
        }
        if code.is_empty() {
            self.err(
                line,
                "missing the rule code, e.g. `.rule  life_like  B3/S23`",
            );
        }
        self.rule = Some((line, code.concat()));
    }

    fn unsupported(&mut self, line: usize, family: &str) {
        self.err(
            line,
            format!("{family} rules aren't available yet; only life_like is"),
        );
        self.rule = Some((line, String::new()));
    }

    fn neigh(&mut self, line: usize, args: &[&str]) {
        if self.duplicate(line, ".neigh", self.neigh.as_ref().map(|n| n.0)) {
            return;
        }
        let parsed = self.neigh_value(line, args);
        self.neigh = Some((line, parsed));
    }

    fn neigh_value(&mut self, line: usize, args: &[&str]) -> Option<Neighbourhood> {
        let Some((key, rest)) = args.split_first() else {
            self.err(
                line,
                "expected a neighbourhood: moore, vn, hex, moore2 or custom",
            );
            return None;
        };
        let preset = NeighbourhoodPreset::from_key(key);
        let custom = key.eq_ignore_ascii_case("custom");
        if preset.is_none() && !custom {
            self.err(
                line,
                format!("unknown neighbourhood `{key}` (use moore, vn, hex, moore2 or custom)"),
            );
            return None;
        }
        let mut n = None;
        let mut offsets = Vec::new();
        let mut ok = true;
        for arg in rest {
            if let Some(v) = key_value(arg, "n") {
                match v.parse::<usize>() {
                    Ok(v) if n.is_none() => n = Some(v),
                    Ok(_) => {
                        self.err(line, "n= is given twice");
                        ok = false;
                    }
                    Err(_) => {
                        self.err(line, format!("n={v} isn't a number of cells"));
                        ok = false;
                    }
                }
            } else if arg.starts_with('(') {
                match parse_pair(arg) {
                    Some(p) => offsets.push(p),
                    None => {
                        self.err(line, format!("`{arg}` isn't an offset like (1,-1)"));
                        ok = false;
                    }
                }
            } else {
                self.err(line, format!("unexpected `{arg}` in .neigh"));
                ok = false;
            }
        }
        let nb = match preset {
            Some(p) => {
                if !offsets.is_empty() {
                    self.err(
                        line,
                        format!(
                            "offsets are only for custom neighbourhoods; {} is fixed",
                            p.key()
                        ),
                    );
                    ok = false;
                }
                p.neighbourhood()
            }
            None => {
                let small = offsets
                    .iter()
                    .map(|&(x, y)| Some((i32::try_from(x).ok()?, i32::try_from(y).ok()?)))
                    .collect::<Option<Vec<_>>>();
                let built = match small {
                    Some(small) => Neighbourhood::from_offsets(small).map_err(|e| e.to_string()),
                    None => Err("offsets run from -2 to 2".to_string()),
                };
                match built {
                    Ok(nb) => nb,
                    Err(message) => {
                        self.err(line, message);
                        return None;
                    }
                }
            }
        };
        if let Some(n) = n
            && n != nb.len()
        {
            let what = preset.map_or_else(
                || "these offsets make".to_string(),
                |p| format!("{} has", p.key()),
            );
            self.err(
                line,
                format!("n={n} doesn't match: {what} {} cells", nb.len()),
            );
            ok = false;
        }
        ok.then_some(nb)
    }

    fn grid(&mut self, line: usize, args: &[&str]) {
        if self.duplicate(line, ".grid", self.grid.as_ref().map(|g| g.0)) {
            return;
        }
        let parsed = self.grid_value(line, args);
        self.grid = Some((line, parsed));
    }

    fn grid_value(&mut self, line: usize, args: &[&str]) -> Option<(usize, usize, Boundary)> {
        let Some((size, rest)) = args.split_first() else {
            self.err(line, "expected a size, e.g. `.grid  64x40  boundary=torus`");
            return None;
        };
        let dims = size
            .split_once(['x', 'X', '×'])
            .and_then(|(w, h)| Some((w.parse::<usize>().ok()?, h.parse::<usize>().ok()?)));
        let mut ok = true;
        let (w, h) = match dims {
            Some((w, h)) if (1..=MAX_RUN_DIM).contains(&w) && (1..=MAX_RUN_DIM).contains(&h) => {
                (w, h)
            }
            Some((w, h)) => {
                self.err(
                    line,
                    format!("{w}x{h} is out of range: each side must be 1 to {MAX_RUN_DIM}"),
                );
                ok = false;
                (1, 1)
            }
            None => {
                self.err(line, format!("`{size}` isn't a grid size like 64x40"));
                ok = false;
                (1, 1)
            }
        };
        let mut boundary = None;
        for arg in rest {
            match key_value(arg, "boundary") {
                Some(v) => match Boundary::from_key(v) {
                    Some(_) if boundary.is_some() => {
                        self.err(line, "boundary= is given twice");
                        ok = false;
                    }
                    Some(b) => boundary = Some(b),
                    None => {
                        self.err(line, format!("unknown boundary `{v}` (use torus or dead)"));
                        ok = false;
                    }
                },
                None => {
                    self.err(
                        line,
                        format!(
                            "unexpected `{arg}` in .grid (expected boundary=torus or boundary=dead)"
                        ),
                    );
                    ok = false;
                }
            }
        }
        ok.then(|| (w, h, boundary.unwrap_or_default()))
    }

    fn seed(&mut self, line: usize, args: &[&str]) {
        if self.duplicate(line, ".seed", self.seed.as_ref().map(|s| s.0)) {
            return;
        }
        let parsed = self.seed_value(line, args);
        self.seed = Some((line, parsed));
    }

    fn seed_value(&mut self, line: usize, args: &[&str]) -> Option<(Seed, Option<(i64, i64)>)> {
        let Some((kind, rest)) = args.split_first() else {
            self.err(line, "expected a seed: r_pentomino, acorn or soup");
            return None;
        };
        let stamp = match kind.to_ascii_lowercase().replace('-', "_").as_str() {
            "r_pentomino" | "rpentomino" | "rpent" => Some(SeedKind::RPentomino),
            "acorn" => Some(SeedKind::Acorn),
            "soup" => None,
            _ => {
                self.err(
                    line,
                    format!("unknown seed `{kind}` (use r_pentomino, acorn or soup)"),
                );
                return None;
            }
        };
        if let Some(kind) = stamp {
            let seed = match kind {
                SeedKind::RPentomino => Seed::RPentomino,
                SeedKind::Acorn => Seed::Acorn,
            };
            return match rest {
                [] => Some((seed, None)),
                ["@", at] => match parse_pair(at) {
                    Some(p) => Some((seed, Some(p))),
                    None => {
                        self.err(line, format!("`{at}` isn't a position like (32,20)"));
                        None
                    }
                },
                _ => {
                    self.err(
                        line,
                        format!("expected `{} @ (x,y)` or just `{}`", seed.key(), seed.key()),
                    );
                    None
                }
            };
        }
        let (mut seed, mut density) = (None, None);
        let mut ok = true;
        for arg in rest {
            if let Some(v) = key_value(arg, "seed") {
                match v.parse::<u32>() {
                    Ok(_) if seed.is_some() => {
                        self.err(line, "seed= is given twice");
                        ok = false;
                    }
                    Ok(s) => seed = Some(s),
                    Err(_) => {
                        self.err(
                            line,
                            format!("seed={v} isn't a whole number from 0 to {}", u32::MAX),
                        );
                        ok = false;
                    }
                }
            } else if let Some(v) = key_value(arg, "density") {
                match v.parse::<f64>() {
                    Ok(_) if density.is_some() => {
                        self.err(line, "density= is given twice");
                        ok = false;
                    }
                    Ok(d) if (0.0..=1.0).contains(&d) => density = Some(d),
                    _ => {
                        self.err(line, format!("density={v} must be a number from 0 to 1"));
                        ok = false;
                    }
                }
            } else {
                self.err(
                    line,
                    format!("unexpected `{arg}` in .seed soup (expected seed= and density=)"),
                );
                ok = false;
            }
        }
        ok.then(|| {
            (
                Seed::Soup {
                    seed: seed.unwrap_or(7),
                    density: density.unwrap_or(0.33),
                },
                None,
            )
        })
    }

    fn gens(&mut self, line: usize, args: &[&str]) {
        if self.duplicate(line, ".run", self.gens.as_ref().map(|g| g.0)) {
            return;
        }
        let parsed = match args {
            [arg] => match key_value(arg, "gens").map(str::parse::<usize>) {
                Some(Ok(g)) if (1..=MAX_RUN_GENS).contains(&g) => Some(g),
                Some(_) => {
                    self.err(
                        line,
                        format!("gens must be a whole number from 1 to {MAX_RUN_GENS}"),
                    );
                    None
                }
                None => {
                    self.err(
                        line,
                        format!("unexpected `{arg}` in .run (expected gens=400)"),
                    );
                    None
                }
            },
            _ => {
                self.err(line, "expected `.run  gens=400`");
                None
            }
        };
        self.gens = Some((line, parsed));
    }

    fn probe(&mut self, line: usize, args: &[&str]) {
        let [quantity, "->", target] = args else {
            self.err(line, "expected `.probe  population  -> plot1`");
            return;
        };
        let Some(q) = ProbeQuantity::from_key(quantity) else {
            let keys: Vec<&str> = ProbeQuantity::ALL.iter().map(|q| q.key()).collect();
            self.err(
                line,
                format!("unknown probe `{quantity}` (use {})", keys.join(", ")),
            );
            return;
        };
        if target.is_empty()
            || !target
                .chars()
                .all(|c| c.is_alphanumeric() || matches!(c, '_' | '-' | '.'))
        {
            self.err(
                line,
                format!("`{target}` isn't a probe target (letters, digits, _ - .)"),
            );
            return;
        }
        self.probes.push(Probe::new(q, *target));
    }

    fn finish(mut self) -> Result<RuleScript, Vec<ScriptError>> {
        let at = self.end.unwrap_or(self.last_line).max(1);
        for (missing, example) in [
            (self.rule.is_none(), ".rule  life_like  B3/S23"),
            (self.neigh.is_none(), ".neigh  moore  n=8"),
            (self.grid.is_none(), ".grid  64x40  boundary=torus"),
            (self.seed.is_none(), ".seed  r_pentomino @ (32,20)"),
            (self.gens.is_none(), ".run  gens=400"),
        ] {
            if missing {
                let directive = example.split_whitespace().next().unwrap_or_default();
                self.err(at, format!("missing {directive} (e.g. `{example}`)"));
            }
        }
        let neighbourhood = self.neigh.and_then(|n| n.1);
        let rule = match &self.rule {
            Some((line, code)) if !code.is_empty() => {
                let line = *line;
                // Without a valid neighbourhood, still check the rule's syntax (up to 24).
                match LifeRule::parse_with(code, neighbourhood.unwrap_or(Neighbourhood::MOORE2)) {
                    Ok(rule) => Some(rule),
                    Err(e) => {
                        self.err(line, format!("{e}"));
                        None
                    }
                }
            }
            _ => None,
        };
        let grid = self.grid.and_then(|g| g.1);
        let seed = match self.seed {
            Some((line, Some((seed, at)))) => {
                if let (Some(at), Some((w, h, _))) = (at, grid) {
                    let (cx, cy) = Seed::centre(w, h);
                    if at != (cx as i64, cy as i64) {
                        self.err(
                            line,
                            format!(
                                "{} always sits at the grid's centre: for {w}x{h} write @ ({cx},{cy}) or leave the position out",
                                seed.key()
                            ),
                        );
                    }
                }
                Some(seed)
            }
            _ => None,
        };
        let gens = self.gens.and_then(|g| g.1);
        if !self.errors.is_empty() {
            self.errors.sort_by_key(|e| e.line);
            return Err(self.errors);
        }
        match (rule, neighbourhood, grid, seed, gens) {
            (Some(rule), Some(_), Some((w, h, boundary)), Some(seed), Some(gens)) => {
                Ok(RuleScript {
                    name: self.name.unwrap_or_else(|| UNTITLED_NAME.to_string()),
                    rule,
                    w,
                    h,
                    boundary,
                    seed,
                    gens,
                    probes: self.probes,
                })
            }
            // Unreachable in practice: every `None` above reported an error.
            _ => Err(vec![ScriptError::new(at, "the script is incomplete")]),
        }
    }
}

/// `key=value` (key case-insensitive) → `value`.
fn key_value<'a>(arg: &'a str, key: &str) -> Option<&'a str> {
    let (k, v) = arg.split_once('=')?;
    k.eq_ignore_ascii_case(key).then_some(v)
}

/// `(x,y)` → `(x, y)`.
fn parse_pair(arg: &str) -> Option<(i64, i64)> {
    let inner = arg.strip_prefix('(')?.strip_suffix(')')?;
    let (x, y) = inner.split_once(',')?;
    Some((x.parse().ok()?, y.parse().ok()?))
}

/// Splits a line into tokens: whitespace separates words, except inside parentheses (removed
/// there, so `( 32 , 20 )` is one token `(32,20)`) and around `=` (`gens = 400` → `gens=400`);
/// `@` and `->` are tokens of their own.
fn tokens(line: &str) -> Vec<String> {
    let spaced = line.replace("->", " -> ").replace('@', " @ ");
    let chars: Vec<char> = spaced.chars().collect();
    // The first non-whitespace character after each position, found in one backward pass (a
    // forward look-ahead per whitespace character was quadratic in long whitespace runs).
    let mut next_solid = vec![None; chars.len()];
    let mut ahead = None;
    for (i, &c) in chars.iter().enumerate().rev() {
        next_solid[i] = ahead;
        if !c.is_whitespace() {
            ahead = Some(c);
        }
    }
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut depth = 0usize;
    for (i, &c) in chars.iter().enumerate() {
        if c.is_whitespace() {
            let next = next_solid[i];
            if depth > 0 || cur.ends_with('=') || next == Some('=') {
                continue;
            }
            if !cur.is_empty() {
                out.push(std::mem::take(&mut cur));
            }
            continue;
        }
        match c {
            '(' => {
                if depth == 0 && !cur.is_empty() && !cur.ends_with('=') {
                    out.push(std::mem::take(&mut cur));
                }
                depth += 1;
            }
            ')' => depth = depth.saturating_sub(1),
            _ => {}
        }
        cur.push(c);
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const MOCKUP: &str = "* automata netlist · glider-garden.rule\n.rule   life_like  B3/S23\n.neigh  moore  n=8\n.grid   64x40  boundary=torus\n.seed   r_pentomino @ (32,20)\n.run    gens=400\n.probe  population  -> plot1\n.probe  period      -> report\n.end";

    #[test]
    fn prints_the_mockup_script() {
        assert_eq!(RuleScript::default().to_text(), MOCKUP);
        assert_eq!(RuleScript::parse(MOCKUP), Ok(RuleScript::default()));
    }

    #[test]
    fn lenient_spacing_and_case() {
        let text = "*  Automata Netlist - lab.rule\n\t.RULE Life_Like b3s23\n.neigh MOORE\n.grid 64X40\n.seed rpent @( 32 , 20 )\n.run gens = 400\n";
        let s = RuleScript::parse(text).unwrap();
        assert_eq!(s.name, "lab.rule");
        assert_eq!(s.spec(), RunSpec::default());
        assert!(s.probes.is_empty());
    }

    #[test]
    fn custom_neighbourhoods_round_trip() {
        let nb = Neighbourhood::from_offsets([(0, -1), (2, 2), (-2, 1)]).unwrap();
        let s = RuleScript {
            rule: LifeRule::parse_with("B2/S1", nb).unwrap(),
            ..RuleScript::default()
        };
        let text = s.to_text();
        assert!(
            text.contains(".neigh  custom  n=3  (0,-1) (-2,1) (2,2)"),
            "{text}"
        );
        assert_eq!(RuleScript::parse(&text), Ok(s));
        let vn = RuleScript::parse(
            &MOCKUP.replace("moore  n=8", "custom n=4 (0,-1) (-1,0) (1,0) (0,1)"),
        )
        .unwrap();
        assert_eq!(vn.rule.neighbourhood(), Neighbourhood::VON_NEUMANN);
        assert!(vn.to_text().contains(".neigh  vn  n=4"));
    }

    #[test]
    fn errors_name_their_lines() {
        let text = "* x\n.rule life_like B9/S\n.neigh moore\n.grid 0x40 boundary=wobbly\n.seed acorn @ (1,1)\n.run gens=-3\n.probe colour -> plot\n.frob\nhello\n.end\n.rule";
        let errors = RuleScript::parse(text).unwrap_err();
        let lines: Vec<usize> = errors.iter().map(|e| e.line).collect();
        assert_eq!(lines, [2, 4, 4, 6, 7, 8, 9, 11], "{errors:#?}");
        assert_eq!(
            errors[0].to_string(),
            "line 2: 9 neighbours is more than this neighbourhood has (8)"
        );
        let n = RuleScript::parse(&MOCKUP.replace("n=8", "n=7")).unwrap_err();
        assert_eq!(n[0].message, "n=7 doesn't match: moore has 8 cells");
        let off = RuleScript::parse(&MOCKUP.replace("moore  n=8", "custom (0,3)")).unwrap_err();
        assert_eq!((off[0].line, off.len()), (3, 1), "{off:?}");
        let missing = RuleScript::parse(".rule life_like B3/S23\n.end").unwrap_err();
        assert_eq!(missing.len(), 4);
        assert!(
            missing
                .iter()
                .all(|e| e.line == 2 && e.message.starts_with("missing"))
        );
        let dup = RuleScript::parse(&format!("{MOCKUP}\n").replace(".end", ".run gens=3\n.end"))
            .unwrap_err();
        assert_eq!(dup[0].message, ".run is given twice (first on line 6)");
    }

    #[test]
    fn other_families_are_explained() {
        let e = RuleScript::parse(&MOCKUP.replace("life_like", "generations")).unwrap_err();
        assert_eq!(
            e[0].message,
            "Generations rules aren't available yet; only life_like is"
        );
        let e = RuleScript::parse(&MOCKUP.replace("life_like  B3/S23", "B3/S23")).unwrap_err();
        assert_eq!(
            e[0].message,
            "missing the family: write `.rule  life_like  B3/S23`"
        );
    }

    #[test]
    fn seed_position_must_be_the_centre() {
        let e = RuleScript::parse(&MOCKUP.replace("64x40", "80x50")).unwrap_err();
        assert_eq!(e.len(), 1);
        assert_eq!(e[0].line, 5);
        assert!(e[0].message.contains("@ (40,25)"), "{}", e[0].message);
        let s =
            RuleScript::parse(&MOCKUP.replace("64x40", "80x50").replace(" @ (32,20)", "")).unwrap();
        assert!(s.to_text().contains("r_pentomino @ (40,25)"));
    }

    #[test]
    fn soup_defaults_and_round_trip() {
        let s = RuleScript::parse(&MOCKUP.replace("r_pentomino @ (32,20)", "soup")).unwrap();
        assert_eq!(s.seed, Seed::MOCKUP_SOUP);
        assert!(s.to_text().contains(".seed   soup seed=7 density=0.33"));
        let mut t = s.clone();
        t.seed = Seed::Soup {
            seed: 123,
            density: 0.1 + 0.2,
        };
        assert_eq!(RuleScript::parse(&t.to_text()), Ok(t));
    }
}
