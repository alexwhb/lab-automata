//! The Rule lab's analysis: run a spec for N generations keeping every frame, then report peak,
//! final population, the exact first repeat, the period and the verdict (SPEC ENG-5, LAB-2/3/5).

use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicBool, Ordering};

use thiserror::Error;

use crate::grid::{Anchor, Grid, Scratch};
use crate::pattern::{Pattern, library, rle};
use crate::rule::{Boundary, LifeRule};

/// The Rule lab's grid width (64).
pub const LAB_W: usize = 64;
/// The Rule lab's grid height (40).
pub const LAB_H: usize = 40;
/// The Rule lab's generation count (400).
pub const LAB_GENS: usize = 400;
/// The largest grid side a run accepts (larger specs are clamped).
pub const MAX_RUN_DIM: usize = 256;
/// The most generations a run accepts (larger specs are clamped).
pub const MAX_RUN_GENS: usize = 2000;

/// What the grid starts with (LAB-1 Seed).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Seed {
    /// The R-pentomino, centred: at (31, 19) top-left on the 64 × 40 grid.
    RPentomino,
    /// The acorn, centred: at (29, 19) top-left on the 64 × 40 grid.
    Acorn,
    /// A soup from the mockup's LCG (see [`soup_grid`]).
    Soup {
        /// The LCG's starting value (the mockup uses 7).
        seed: u32,
        /// Probability of a live cell inside the soup box (the mockup uses 0.33).
        density: f64,
    },
}

impl Seed {
    /// The mockup's soup: seed 7, density 0.33.
    pub const MOCKUP_SOUP: Seed = Seed::Soup {
        seed: 7,
        density: 0.33,
    };

    /// The script keyword: `r_pentomino`, `acorn`, `soup`.
    pub const fn key(&self) -> &'static str {
        match self {
            Seed::RPentomino => "r_pentomino",
            Seed::Acorn => "acorn",
            Seed::Soup { .. } => "soup",
        }
    }

    /// The button label: "R-pentomino", "Acorn", "Soup".
    pub const fn label(&self) -> &'static str {
        match self {
            Seed::RPentomino => "R-pentomino",
            Seed::Acorn => "Acorn",
            Seed::Soup { .. } => "Soup",
        }
    }

    /// Where a stamp seed is centred: `(⌊w/2⌋, ⌊h/2⌋)` — (32, 20) on the Rule lab's grid.
    pub const fn centre(w: usize, h: usize) -> (usize, usize) {
        (w / 2, h / 2)
    }

    /// The starting grid for a `w × h` grid with this boundary (stamps wrap on a torus and clip
    /// on dead edges if the grid is too small to hold them).
    pub fn grid(&self, w: usize, h: usize, boundary: Boundary) -> Grid {
        let stamp = |id: &str| {
            let mut grid = Grid::new(w, h);
            let (cx, cy) = Seed::centre(grid.width(), grid.height());
            if let Some(p) = library::pattern(id) {
                grid.place(&p, cx as i64, cy as i64, Anchor::Center, boundary);
            }
            grid
        };
        match *self {
            Seed::RPentomino => stamp("rpent"),
            Seed::Acorn => stamp("acorn"),
            Seed::Soup { seed, density } => soup_grid(w, h, seed, density),
        }
    }
}

/// One step of the mockup's soup LCG, `r = (r · 1103515245 + 12345) mod 2³¹`, computed in
/// `f64` exactly as JavaScript does (the product exceeds 2⁵³ and is rounded, so this is *not*
/// the integer LCG — it is what makes seed 7 give the mockup's soup).
pub fn js_lcg_next(r: f64) -> f64 {
    (r * 1_103_515_245.0 + 12_345.0) % 2_147_483_648.0
}

/// The Rule lab's soup: the LCG advances once per cell in row-major order, starting from
/// `seed`, and a cell is alive when it lies strictly inside the central box and
/// `r / 2³¹ < density`. The box is `w/4 < x < 3w/4`, `h/5 < y < 4h/5` — exactly the mockup's
/// `16 < x < 48`, `8 < y < 32` on 64 × 40.
pub fn soup_grid(w: usize, h: usize, seed: u32, density: f64) -> Grid {
    let mut grid = Grid::new(w, h);
    let (w, h) = (grid.width(), grid.height());
    let mut r = f64::from(seed);
    for y in 0..h {
        for x in 0..w {
            r = js_lcg_next(r);
            let inside = 4 * x > w && 4 * x < 3 * w && 5 * y > h && 5 * y < 4 * h;
            if inside && r / 2_147_483_648.0 < density {
                grid.set(x, y, true);
            }
        }
    }
    grid
}

/// Everything a Rule-lab run needs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RunSpec {
    /// The rule, with its neighbourhood.
    pub rule: LifeRule,
    /// Torus or dead edges.
    pub boundary: Boundary,
    /// Grid width.
    pub w: usize,
    /// Grid height.
    pub h: usize,
    /// The starting cells.
    pub seed: Seed,
    /// Generations to run (frames kept: `gens + 1`).
    pub gens: usize,
}

impl Default for RunSpec {
    /// The mockup's spec: B3/S23 Moore r1, torus, 64 × 40, R-pentomino, 400 generations.
    fn default() -> Self {
        RunSpec {
            rule: LifeRule::LIFE,
            boundary: Boundary::Torus,
            w: LAB_W,
            h: LAB_H,
            seed: Seed::RPentomino,
            gens: LAB_GENS,
        }
    }
}

/// Why a [`RunSpec`] is out of range.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SpecError {
    /// Width or height outside `1..=MAX_RUN_DIM`.
    #[error("the grid must be 1 to {MAX_RUN_DIM} cells on each side, not {w}x{h}")]
    GridSize {
        /// Requested width.
        w: usize,
        /// Requested height.
        h: usize,
    },
    /// Generations outside `1..=MAX_RUN_GENS`.
    #[error("gens must be 1 to {MAX_RUN_GENS}, not {0}")]
    Gens(usize),
    /// Soup density outside `0..=1` (or NaN).
    #[error("soup density must be between 0 and 1")]
    Density,
}

impl RunSpec {
    /// Checks the sizes, generation count and density.
    pub fn validate(&self) -> Result<(), SpecError> {
        if !(1..=MAX_RUN_DIM).contains(&self.w) || !(1..=MAX_RUN_DIM).contains(&self.h) {
            return Err(SpecError::GridSize {
                w: self.w,
                h: self.h,
            });
        }
        if !(1..=MAX_RUN_GENS).contains(&self.gens) {
            return Err(SpecError::Gens(self.gens));
        }
        if let Seed::Soup { density, .. } = self.seed
            && !(0.0..=1.0).contains(&density)
        {
            return Err(SpecError::Density);
        }
        Ok(())
    }

    /// The spec with sizes clamped to `1..=MAX_RUN_DIM` and gens to `0..=MAX_RUN_GENS` (what
    /// [`run`] actually runs).
    pub fn clamped(&self) -> RunSpec {
        RunSpec {
            w: self.w.clamp(1, MAX_RUN_DIM),
            h: self.h.clamp(1, MAX_RUN_DIM),
            gens: self.gens.min(MAX_RUN_GENS),
            ..*self
        }
    }
}

/// One generation's cells as a bitset (64 cells per word, row-major).
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Frame {
    w: usize,
    h: usize,
    bits: Vec<u64>,
}

impl Frame {
    /// The live cells of `grid`.
    pub fn from_grid(grid: &Grid) -> Frame {
        let mut bits = vec![0u64; grid.len().div_ceil(64)];
        for (i, &a) in grid.ages().iter().enumerate() {
            if a > 0 {
                bits[i / 64] |= 1 << (i % 64);
            }
        }
        Frame {
            w: grid.width(),
            h: grid.height(),
            bits,
        }
    }

    /// Width in cells.
    pub fn width(&self) -> usize {
        self.w
    }

    /// Height in cells.
    pub fn height(&self) -> usize {
        self.h
    }

    /// Whether `(x, y)` is alive (`false` outside).
    pub fn get(&self, x: usize, y: usize) -> bool {
        x < self.w && y < self.h && {
            let i = y * self.w + x;
            (self.bits[i / 64] >> (i % 64)) & 1 == 1
        }
    }

    /// Number of live cells.
    pub fn population(&self) -> usize {
        self.bits.iter().map(|b| b.count_ones() as usize).sum()
    }

    /// The live cells `(x, y)`, row-major.
    pub fn alive_cells(&self) -> impl Iterator<Item = (usize, usize)> + '_ {
        let w = self.w;
        self.bits.iter().enumerate().flat_map(move |(k, &word)| {
            let mut word = word;
            std::iter::from_fn(move || {
                (word != 0).then(|| {
                    let b = word.trailing_zeros() as usize;
                    word &= word - 1;
                    let i = k * 64 + b;
                    (i % w, i / w)
                })
            })
        })
    }

    /// The whole frame as a `w × h` pattern.
    pub fn to_pattern(&self) -> Pattern {
        Pattern::new(self.w, self.h, self.alive_cells())
    }

    /// The live cells trimmed to their bounding box.
    pub fn trimmed_pattern(&self) -> Pattern {
        self.to_pattern().trimmed()
    }

    /// A grid holding this frame's cells (all newborn).
    pub fn to_grid(&self) -> Grid {
        let mut grid = Grid::new(self.w, self.h);
        for (x, y) in self.alive_cells() {
            grid.set(x, y, true);
        }
        grid
    }

    fn fingerprint(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.bits.hash(&mut hasher);
        hasher.finish()
    }
}

impl std::fmt::Debug for Frame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Frame({}×{}, {} alive)",
            self.w,
            self.h,
            self.population()
        )
    }
}

/// The first repeated state: the frame at `start + period` equals the one at `start`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Repeat {
    /// The earlier generation (the cycle is entered here; the verdict's "at gen N").
    pub start: usize,
    /// The cycle length.
    pub period: usize,
}

impl Repeat {
    /// The generation at which the repeat is seen, `start + period` (the "First repeat" tile).
    pub const fn seen_at(&self) -> usize {
        self.start + self.period
    }
}

/// How a cell looks in the Rule lab's top-down view at a generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CellChange {
    /// Dead now and before.
    Empty,
    /// Alive now, dead the generation before (newborn colour).
    Born,
    /// Alive now and before (or at generation 0).
    Alive,
    /// Died this generation (drawn as a ghost).
    Ghost,
}

/// A finished run: every frame from generation 0 to `gens`, the populations and the first
/// repeat.
#[derive(Debug, Clone, PartialEq)]
pub struct Run {
    spec: RunSpec,
    frames: Vec<Frame>,
    populations: Vec<usize>,
    first_repeat: Option<Repeat>,
}

/// Runs `spec` (clamped, see [`RunSpec::clamped`]): keeps every frame and the population of
/// each, and finds the first repeat exactly — frames are hashed and a hash hit is confirmed
/// cell by cell, so a hash collision can never fake a cycle (LAB-3).
pub fn run(spec: &RunSpec) -> Run {
    run_until(spec, || false).0
}

/// [`run`], stopping early once `cancelled` is set (checked before every generation): returns
/// `None` when cancelled, else exactly what [`run`] returns. For runs on a worker thread that a
/// newer request supersedes (the Rule lab's big specs).
pub fn run_cancellable(spec: &RunSpec, cancelled: &AtomicBool) -> Option<Run> {
    let (run, stopped) = run_until(spec, || cancelled.load(Ordering::Relaxed));
    (!stopped).then_some(run)
}

/// Runs `spec` until `stop()` says so; the run so far and whether it stopped early.
fn run_until(spec: &RunSpec, mut stop: impl FnMut() -> bool) -> (Run, bool) {
    let spec = spec.clamped();
    let mut grid = spec.seed.grid(spec.w, spec.h, spec.boundary);
    let mut scratch = Scratch::default();
    let mut frames = Vec::with_capacity(spec.gens + 1);
    let mut populations = Vec::with_capacity(spec.gens + 1);
    let mut seen: HashMap<u64, Vec<usize>> = HashMap::new();
    let mut first_repeat = None;
    let mut stopped = false;
    for t in 0..=spec.gens {
        if stop() {
            stopped = true;
            break;
        }
        if t > 0 {
            grid.step_with(&spec.rule, spec.boundary, &mut scratch);
        }
        let frame = Frame::from_grid(&grid);
        populations.push(frame.population());
        if first_repeat.is_none() {
            let gens = seen.entry(frame.fingerprint()).or_default();
            match gens.iter().find(|&&g| frames[g] == frame) {
                Some(&start) => {
                    first_repeat = Some(Repeat {
                        start,
                        period: t - start,
                    });
                }
                None => gens.push(t),
            }
        }
        frames.push(frame);
    }
    let run = Run {
        spec,
        frames,
        populations,
        first_repeat,
    };
    (run, stopped)
}

impl Run {
    /// The spec that ran (after clamping).
    pub fn spec(&self) -> &RunSpec {
        &self.spec
    }

    /// The number of generations run (frames are `0..=gens`).
    pub fn gens(&self) -> usize {
        self.spec.gens
    }

    /// Every frame, generation 0 first.
    pub fn frames(&self) -> &[Frame] {
        &self.frames
    }

    /// The frame at `generation`.
    pub fn frame(&self, generation: usize) -> Option<&Frame> {
        self.frames.get(generation)
    }

    /// The population of every generation.
    pub fn populations(&self) -> &[usize] {
        &self.populations
    }

    /// The population at `generation`.
    pub fn population(&self, generation: usize) -> Option<usize> {
        self.populations.get(generation).copied()
    }

    /// The first repeat, if the run cycles within `gens`.
    pub fn first_repeat(&self) -> Option<Repeat> {
        self.first_repeat
    }

    /// The top-down view's cell state at `generation` (born / alive / ghost / empty).
    pub fn cell(&self, generation: usize, x: usize, y: usize) -> CellChange {
        let Some(now) = self.frame(generation) else {
            return CellChange::Empty;
        };
        let before = generation.checked_sub(1).and_then(|g| self.frame(g));
        match (now.get(x, y), before.map(|f| f.get(x, y))) {
            (true, Some(false)) => CellChange::Born,
            (true, _) => CellChange::Alive,
            (false, Some(true)) => CellChange::Ghost,
            (false, _) => CellChange::Empty,
        }
    }

    /// The live cells at `generation`, trimmed to their bounding box.
    pub fn pattern(&self, generation: usize) -> Option<Pattern> {
        self.frame(generation).map(Frame::trimmed_pattern)
    }

    /// The pattern at `generation` as RLE with `rule =` (LAB-5 export): the full text, not the
    /// panel's truncated preview.
    pub fn rle(&self, generation: usize) -> Option<String> {
        self.pattern(generation)
            .map(|p| rle::write(&p, Some(&self.spec.rule), None))
    }

    /// The analysis (LAB-3).
    pub fn report(&self) -> Report {
        let (peak_gen, peak) =
            self.populations
                .iter()
                .copied()
                .enumerate()
                .fold(
                    (0, 0),
                    |best, (g, p)| if p > best.1 || g == 0 { (g, p) } else { best },
                );
        let final_pop = self.populations.last().copied().unwrap_or(0);
        let extinct_gen = self.populations.iter().position(|&p| p == 0);
        let verdict = match (final_pop, self.first_repeat) {
            (0, _) => format!("Dies out by gen {}.", extinct_gen.unwrap_or(self.gens())),
            (_, Some(r)) if r.period == 1 => {
                format!("Freezes into still lifes at gen {}.", r.start)
            }
            (_, Some(r)) => format!(
                "Settles into a period-{} cycle at gen {}.",
                r.period, r.start
            ),
            (_, None) => format!("No repeat within {} gens.", self.gens()),
        };
        Report {
            peak,
            peak_gen,
            final_pop,
            first_repeat: self.first_repeat,
            first_repeat_gen: self.first_repeat.map(|r| r.seen_at()),
            period: self.first_repeat.map(|r| r.period),
            extinct_gen,
            verdict,
        }
    }
}

/// The Rule lab's analysis card.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    /// The highest population.
    pub peak: usize,
    /// The first generation reaching it.
    pub peak_gen: usize,
    /// The population at the last generation.
    pub final_pop: usize,
    /// The first repeat.
    pub first_repeat: Option<Repeat>,
    /// Where the repeat is seen, `start + period` (the "First repeat" tile; 390 for the mockup).
    pub first_repeat_gen: Option<usize>,
    /// The cycle length (1 = still lifes).
    pub period: Option<usize>,
    /// The first generation with no live cells.
    pub extinct_gen: Option<usize>,
    /// "Dies out by gen N." / "Freezes into still lifes at gen N." / "Settles into a period-P
    /// cycle at gen N." / "No repeat within G gens." (N is the repeat's start).
    pub verdict: String,
}

impl Report {
    /// The "Peak population" tile: `"221 @ 148"`.
    pub fn peak_text(&self) -> String {
        format!("{} @ {}", self.peak, self.peak_gen)
    }

    /// The "First repeat" tile: `"gen 390"` or `"none"`.
    pub fn first_repeat_text(&self) -> String {
        self.first_repeat_gen
            .map_or_else(|| "none".to_string(), |g| format!("gen {g}"))
    }

    /// The "Period" tile: `"2"` or `"—"`.
    pub fn period_text(&self) -> String {
        self.period
            .map_or_else(|| "—".to_string(), |p| p.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seeds_sit_where_the_mockup_puts_them() {
        let g = Seed::RPentomino.grid(LAB_W, LAB_H, Boundary::Torus);
        assert_eq!(g.bbox().map(|b| (b.x, b.y)), Some((31, 19)));
        let g = Seed::Acorn.grid(LAB_W, LAB_H, Boundary::Dead);
        assert_eq!(g.bbox().map(|b| (b.x, b.y)), Some((29, 19)));
        let g = Seed::MOCKUP_SOUP.grid(LAB_W, LAB_H, Boundary::Torus);
        let b = g.bbox().unwrap();
        assert!(b.x > 16 && b.x + b.w <= 48 && b.y > 8 && b.y + b.h <= 32);
    }

    #[test]
    fn js_lcg_rounds_like_javascript() {
        // 7 · 1103515245 + 12345 = 7724619060 is exact; mod 2^31 that is 1282168116.
        assert_eq!(js_lcg_next(7.0), 1_282_168_116.0);
        // From the next step on the product exceeds 2^53 and is rounded, so the f64 sequence
        // leaves the exact integer LCG almost at once.
        let (mut r, mut exact) = (7.0, 7u64);
        let mut diverged = None;
        for k in 0..10_000 {
            r = js_lcg_next(r);
            exact = (exact * 1_103_515_245 + 12_345) % 2_147_483_648;
            assert!((0.0..2_147_483_648.0).contains(&r) && r.fract() == 0.0);
            if diverged.is_none() && r != exact as f64 {
                diverged = Some(k);
            }
        }
        assert!(diverged.is_some_and(|k| k < 5), "{diverged:?}");
    }

    #[test]
    fn report_texts() {
        let run = run(&RunSpec::default());
        let r = run.report();
        assert_eq!(r.peak_text(), "221 @ 148");
        assert_eq!(r.first_repeat_text(), "gen 390");
        assert_eq!(r.period_text(), "2");
        assert_eq!(r.verdict, "Settles into a period-2 cycle at gen 388.");
        assert_eq!(run.population(120), Some(148));
        assert_eq!(r.final_pop, 46);
    }

    #[test]
    fn extinction_and_no_repeat_verdicts() {
        let dies = RunSpec {
            rule: LifeRule::parse("B/S").unwrap(),
            gens: 10,
            ..RunSpec::default()
        };
        assert_eq!(run(&dies).report().verdict, "Dies out by gen 1.");
        let short = RunSpec {
            gens: 50,
            ..RunSpec::default()
        };
        let r = run(&short).report();
        assert_eq!(r.verdict, "No repeat within 50 gens.");
        assert_eq!(
            (r.first_repeat_text(), r.period_text()),
            ("none".into(), "—".into())
        );
    }

    #[test]
    fn cell_changes() {
        let run = run(&RunSpec::default());
        let f1 = run.frame(1).unwrap();
        let f0 = run.frame(0).unwrap();
        for y in 0..LAB_H {
            for x in 0..LAB_W {
                let expect = match (f1.get(x, y), f0.get(x, y)) {
                    (true, false) => CellChange::Born,
                    (true, true) => CellChange::Alive,
                    (false, true) => CellChange::Ghost,
                    (false, false) => CellChange::Empty,
                };
                assert_eq!(run.cell(1, x, y), expect);
            }
        }
        assert_eq!(
            run.cell(0, 32, 19),
            CellChange::Alive,
            "gen 0 has nothing to be born from"
        );
        assert_eq!(run.cell(9999, 0, 0), CellChange::Empty);
    }
}
