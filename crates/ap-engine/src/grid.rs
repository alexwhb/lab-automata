//! The cell grid: ages, ghost trails, editing, placing patterns and stepping a Life-like rule
//! (SPEC ENG-1, TRAY-2, TRAY-3).

use std::fmt;

use crate::pattern::Pattern;
use crate::rng::Rng;
use crate::rule::{Boundary, LifeRule, Neighbourhood};

/// Grid dimensions are clamped to `1..=MAX_GRID_DIM`.
pub const MAX_GRID_DIM: usize = 1024;

/// How many generations a dead cell's ghost trail lasts.
pub const TRAIL_LENGTH: u8 = 3;

/// Counts of one generation step.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct StepStats {
    /// Cells born this generation.
    pub born: usize,
    /// Cells that died this generation.
    pub died: usize,
    /// Live cells after the step.
    pub population: usize,
}

/// Where a placed pattern's `(x, y)` sits on the pattern.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Anchor {
    /// `(x, y)` is the pattern's top-left cell.
    #[default]
    TopLeft,
    /// `(x, y)` is the pattern's centre: the top-left goes to `(x − ⌊w/2⌋, y − ⌊h/2⌋)`, as the
    /// Tray's Stamp tool does.
    Center,
}

/// An axis-aligned box of cells.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BBox {
    /// Left column.
    pub x: usize,
    /// Top row.
    pub y: usize,
    /// Width in cells (≥ 1).
    pub w: usize,
    /// Height in cells (≥ 1).
    pub h: usize,
}

/// A `w × h` grid of cells. Each cell has an `age` (0 dead, 1 newborn, then +1 per generation
/// survived, saturating at `u16::MAX`) and a ghost `trail` (3 right after it dies, fading by 1
/// per generation to 0; always 0 while alive). Coordinates are `(x, y)` with `x` to the right
/// and `y` down; out-of-range reads return dead, out-of-range writes are ignored.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Grid {
    w: usize,
    h: usize,
    age: Vec<u16>,
    trail: Vec<u8>,
}

impl Grid {
    /// An empty grid; dimensions are clamped to `1..=MAX_GRID_DIM`.
    pub fn new(w: usize, h: usize) -> Grid {
        let w = w.clamp(1, MAX_GRID_DIM);
        let h = h.clamp(1, MAX_GRID_DIM);
        Grid {
            w,
            h,
            age: vec![0; w * h],
            trail: vec![0; w * h],
        }
    }

    /// A grid whose cells are each alive with probability `density` (the Tray's Random soup uses
    /// 0.3). All live cells are newborn; no trails.
    pub fn random(w: usize, h: usize, density: f64, rng: &mut Rng) -> Grid {
        let mut grid = Grid::new(w, h);
        for a in &mut grid.age {
            if rng.chance(density) {
                *a = 1;
            }
        }
        grid
    }

    /// Width in cells.
    pub fn width(&self) -> usize {
        self.w
    }

    /// Height in cells.
    pub fn height(&self) -> usize {
        self.h
    }

    /// Number of cells, `w × h`.
    pub fn len(&self) -> usize {
        self.age.len()
    }

    /// Always `false`: a grid has at least one cell. (Here for API completeness.)
    pub fn is_empty(&self) -> bool {
        self.age.is_empty()
    }

    /// The row-major index of `(x, y)`, if inside.
    pub fn index(&self, x: usize, y: usize) -> Option<usize> {
        (x < self.w && y < self.h).then(|| y * self.w + x)
    }

    /// The age of `(x, y)` (0 when dead or outside).
    pub fn age(&self, x: usize, y: usize) -> u16 {
        self.index(x, y).map_or(0, |i| self.age[i])
    }

    /// The ghost trail of `(x, y)` (0–3; 0 when outside).
    pub fn trail(&self, x: usize, y: usize) -> u8 {
        self.index(x, y).map_or(0, |i| self.trail[i])
    }

    /// Whether `(x, y)` is alive.
    pub fn is_alive(&self, x: usize, y: usize) -> bool {
        self.age(x, y) > 0
    }

    /// All ages, row-major.
    pub fn ages(&self) -> &[u16] {
        &self.age
    }

    /// All trails, row-major.
    pub fn trails(&self) -> &[u8] {
        &self.trail
    }

    /// Sets `(x, y)` alive (as a newborn, age 1, even if it was already alive — as the Tray's
    /// Draw tool does) or dead (its trail is left as is). Returns `false` when outside.
    pub fn set(&mut self, x: usize, y: usize, alive: bool) -> bool {
        match self.index(x, y) {
            Some(i) => {
                self.set_index(i, alive);
                true
            }
            None => false,
        }
    }

    fn set_index(&mut self, i: usize, alive: bool) {
        if alive {
            self.age[i] = 1;
            self.trail[i] = 0;
        } else {
            self.age[i] = 0;
        }
    }

    /// Flips `(x, y)`; returns its new state, or `None` when outside.
    pub fn toggle(&mut self, x: usize, y: usize) -> Option<bool> {
        let alive = !self.is_alive(x, y);
        self.set(x, y, alive).then_some(alive)
    }

    /// Kills every cell and clears every trail.
    pub fn clear(&mut self) {
        self.age.fill(0);
        self.trail.fill(0);
    }

    /// Clears the ghost trails only.
    pub fn clear_trails(&mut self) {
        self.trail.fill(0);
    }

    /// Number of live cells.
    pub fn population(&self) -> usize {
        self.age.iter().filter(|&&a| a > 0).count()
    }

    /// The live cells' `(x, y)`, row-major.
    pub fn alive_cells(&self) -> impl Iterator<Item = (usize, usize)> + '_ {
        let w = self.w;
        self.age
            .iter()
            .enumerate()
            .filter(|(_, a)| **a > 0)
            .map(move |(i, _)| (i % w, i / w))
    }

    /// The smallest box holding every live cell, or `None` when the grid is empty.
    pub fn bbox(&self) -> Option<BBox> {
        let (mut x0, mut y0, mut x1, mut y1) = (usize::MAX, usize::MAX, 0, 0);
        for (x, y) in self.alive_cells() {
            x0 = x0.min(x);
            x1 = x1.max(x);
            y0 = y0.min(y);
            y1 = y1.max(y);
        }
        (x0 != usize::MAX).then(|| BBox {
            x: x0,
            y: y0,
            w: x1 - x0 + 1,
            h: y1 - y0 + 1,
        })
    }

    /// Places `pattern`'s live cells (as newborns) with `(x, y)` at `anchor`. On a torus the
    /// pattern wraps around the edges; with dead edges it is clipped. Dead pattern cells don't
    /// erase anything. Returns how many cells were set.
    pub fn place(
        &mut self,
        pattern: &Pattern,
        x: i64,
        y: i64,
        anchor: Anchor,
        boundary: Boundary,
    ) -> usize {
        let (ox, oy) = match anchor {
            Anchor::TopLeft => (x, y),
            Anchor::Center => (
                x - (pattern.width() / 2) as i64,
                y - (pattern.height() / 2) as i64,
            ),
        };
        let (w, h) = (self.w as i64, self.h as i64);
        let mut placed = 0;
        for &(px, py) in pattern.cells() {
            let (cx, cy) = (ox.saturating_add(px as i64), oy.saturating_add(py as i64));
            let target = match boundary {
                Boundary::Torus => Some((cx.rem_euclid(w), cy.rem_euclid(h))),
                Boundary::Dead => (cx >= 0 && cy >= 0 && cx < w && cy < h).then_some((cx, cy)),
            };
            if let Some((tx, ty)) = target {
                self.set_index(ty as usize * self.w + tx as usize, true);
                placed += 1;
            }
        }
        placed
    }

    /// Advances one generation under `rule` — exactly the Tray mockup's `advance()`:
    /// a live cell with a survival count gets `age + 1`, otherwise it dies (`trail = 3`); a dead
    /// cell with a birth count is born (`age = 1`); every other dead cell's trail fades by one.
    pub fn step(&mut self, rule: &LifeRule, boundary: Boundary) -> StepStats {
        let mut scratch = Scratch::default();
        self.step_with(rule, boundary, &mut scratch)
    }

    /// [`step`](Self::step) reusing buffers (the Rule lab steps hundreds of times).
    pub(crate) fn step_with(
        &mut self,
        rule: &LifeRule,
        boundary: Boundary,
        scratch: &mut Scratch,
    ) -> StepStats {
        scratch.count(self, rule.neighbourhood(), boundary);
        let birth = rule.birth().bits();
        let survive = rule.survive().bits();
        let mut stats = StepStats::default();
        for ((a, t), &n) in self
            .age
            .iter_mut()
            .zip(self.trail.iter_mut())
            .zip(&scratch.counts)
        {
            if *a > 0 {
                if (survive >> n) & 1 == 1 {
                    *a = a.saturating_add(1);
                    *t = 0;
                    stats.population += 1;
                } else {
                    *a = 0;
                    *t = TRAIL_LENGTH;
                    stats.died += 1;
                }
            } else if (birth >> n) & 1 == 1 {
                *a = 1;
                *t = 0;
                stats.born += 1;
                stats.population += 1;
            } else {
                *t = t.saturating_sub(1);
            }
        }
        stats
    }
}

impl fmt::Debug for Grid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Grid({}×{}, {} alive)",
            self.w,
            self.h,
            self.population()
        )
    }
}

/// Neighbour counting, branch-free: the live cells are copied into a plane padded by 2 on every
/// side (wrapped for a torus, zeros for dead edges), then each neighbour offset adds one shifted
/// copy of that plane to the counts, row by row (a loop the compiler vectorises). The cost is
/// `N × w × h` byte additions per generation whatever the rule or population.
#[derive(Default)]
pub(crate) struct Scratch {
    padded: Vec<u8>,
    counts: Vec<u8>,
}

const PAD: usize = 2;

impl Scratch {
    fn count(&mut self, grid: &Grid, neighbourhood: Neighbourhood, boundary: Boundary) {
        let (w, h) = (grid.w, grid.h);
        let pw = w + 2 * PAD;
        let ph = h + 2 * PAD;
        self.padded.clear();
        self.padded.resize(pw * ph, 0);
        let wrap = |v: usize, n: usize| (v as isize - PAD as isize).rem_euclid(n as isize) as usize;
        for py in 0..ph {
            let sy = match boundary {
                Boundary::Torus => wrap(py, h),
                Boundary::Dead if py >= PAD && py < h + PAD => py - PAD,
                Boundary::Dead => continue,
            };
            let src = &grid.age[sy * w..(sy + 1) * w];
            let dst = &mut self.padded[py * pw..(py + 1) * pw];
            for (d, &a) in dst[PAD..PAD + w].iter_mut().zip(src) {
                *d = u8::from(a > 0);
            }
            if boundary == Boundary::Torus {
                for px in (0..PAD).chain(w + PAD..pw) {
                    dst[px] = u8::from(src[wrap(px, w)] > 0);
                }
            }
        }
        self.counts.clear();
        self.counts.resize(w * h, 0);
        for (dx, dy) in neighbourhood.offsets() {
            for y in 0..h {
                let row = (y as isize + PAD as isize + dy as isize) as usize;
                let col = (PAD as isize + dx as isize) as usize;
                let start = row * pw + col;
                let src = &self.padded[start..start + w];
                let dst = &mut self.counts[y * w..(y + 1) * w];
                for (d, &s) in dst.iter_mut().zip(src) {
                    *d += s;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn blinker() -> Grid {
        let mut g = Grid::new(5, 5);
        for x in 1..4 {
            g.set(x, 2, true);
        }
        g
    }

    #[test]
    fn blinker_oscillates_with_ages_and_trails() {
        let mut g = blinker();
        let s = g.step(&LifeRule::LIFE, Boundary::Dead);
        assert_eq!(
            s,
            StepStats {
                born: 2,
                died: 2,
                population: 3
            }
        );
        assert_eq!(g.age(2, 2), 2, "the centre survived");
        assert_eq!(g.age(2, 1), 1, "newborn");
        assert_eq!(g.trail(1, 2), 3, "just died");
        g.step(&LifeRule::LIFE, Boundary::Dead);
        assert_eq!(g.trail(1, 2), 0, "reborn");
        assert_eq!(g.trail(2, 1), 3);
        g.step(&LifeRule::LIFE, Boundary::Dead);
        assert_eq!(g.age(2, 2), 4);
        assert_eq!(g.trail(1, 2), 3);
        assert_eq!(g.trail(2, 3), 0);
    }

    #[test]
    fn trails_fade_over_three_generations() {
        let mut g = Grid::new(4, 4);
        g.set(1, 1, true);
        g.step(&LifeRule::LIFE, Boundary::Torus);
        let fades: Vec<u8> = (0..4)
            .map(|_| {
                let t = g.trail(1, 1);
                g.step(&LifeRule::LIFE, Boundary::Torus);
                t
            })
            .collect();
        assert_eq!(fades, [3, 2, 1, 0]);
    }

    #[test]
    fn torus_wraps_and_dead_edges_clip() {
        let mut g = Grid::new(5, 5);
        for y in 0..3 {
            g.set(0, y, true);
        }
        let mut t = g.clone();
        t.step(&LifeRule::LIFE, Boundary::Torus);
        assert!(t.is_alive(4, 1), "wrapped to the right edge");
        g.step(&LifeRule::LIFE, Boundary::Dead);
        assert_eq!(g.population(), 2);
    }

    #[test]
    fn place_centres_wraps_and_clips() {
        let glider = Pattern::from_rows(&[".O.", "..O", "OOO"]);
        let mut g = Grid::new(10, 10);
        assert_eq!(g.place(&glider, 0, 0, Anchor::Center, Boundary::Torus), 5);
        assert!(
            g.is_alive(0, 9),
            "(1,0) of the glider lands on (0,-1) → (0,9)"
        );
        assert!(g.is_alive(9, 1));
        let mut d = Grid::new(10, 10);
        assert_eq!(d.place(&glider, 0, 0, Anchor::Center, Boundary::Dead), 3);
        assert_eq!(
            d.bbox(),
            Some(BBox {
                x: 0,
                y: 0,
                w: 2,
                h: 2
            })
        );
    }

    #[test]
    fn small_and_degenerate_grids_do_not_panic() {
        let mut g = Grid::new(0, 0);
        assert_eq!((g.width(), g.height()), (1, 1));
        g.set(0, 0, true);
        // On a 1×1 torus each of the 24 Moore r2 offsets wraps onto the cell itself.
        // (`S24` would mean counts 2 and 4; a lone 24 is written `S24,`.)
        let rule = LifeRule::parse_with("B/S24,", Neighbourhood::MOORE2).unwrap();
        assert!(rule.survives_with(24) && !rule.survives_with(2));
        assert_eq!(g.step(&rule, Boundary::Torus).population, 1);
        assert_eq!(g.step(&rule, Boundary::Dead).population, 0);
        assert!(!g.set(5, 5, true));
        assert_eq!(g.toggle(3, 0), None);
    }

    #[test]
    fn ages_saturate() {
        let mut g = Grid::new(4, 4);
        for (x, y) in [(1, 1), (2, 1), (1, 2), (2, 2)] {
            g.set(x, y, true);
        }
        let block = LifeRule::LIFE;
        for _ in 0..70_000 {
            g.step(&block, Boundary::Dead);
        }
        assert_eq!(g.age(1, 1), u16::MAX);
    }
}
