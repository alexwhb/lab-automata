//! The Tray's simulation state, free of any UI: the 40 × 26 torus, its rule, the generation
//! counter, the last step's born / died counts and the population history the sparkline and
//! the "what is happening" classifier read (SPEC TRAY-2/3/7/8). Every number the screen shows
//! comes from here.

use ap_engine::grid::Anchor;
use ap_engine::pattern::rle;
use ap_engine::presets::{TRAY_BOUNDARY, TRAY_H, TRAY_W, starting_garden};
use ap_engine::{Grid, History, LifeRule, Mood, Pattern, Rng};

/// Random soup density (TRAY-7: 30 %).
pub const SOUP_DENSITY: f64 = 0.3;

/// A snapshot of the Tray's numbers (the readout tiles, the callout, the MCP `status`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrayStats {
    /// Generations stepped since the last Restart / Clear / Random soup / load.
    pub generation: u64,
    /// Live cells now.
    pub population: usize,
    /// Cells born in the last step.
    pub born: usize,
    /// Cells that died in the last step.
    pub died: usize,
    /// The callout's title ("Churning").
    pub mood_title: &'static str,
    /// The callout's sentence.
    pub mood_text: &'static str,
}

/// The Tray's grid, rule and counters.
#[derive(Debug, Clone)]
pub struct Model {
    grid: Grid,
    rule: LifeRule,
    history: History,
    generation: u64,
    born: usize,
    died: usize,
}

impl Default for Model {
    /// The starting garden under Life (what the app opens on).
    fn default() -> Self {
        let grid = starting_garden();
        let history = History::new(grid.population());
        Model {
            grid,
            rule: LifeRule::LIFE,
            history,
            generation: 0,
            born: 0,
            died: 0,
        }
    }
}

impl Model {
    /// The grid.
    pub fn grid(&self) -> &Grid {
        &self.grid
    }

    /// The rule.
    pub fn rule(&self) -> &LifeRule {
        &self.rule
    }

    /// Sets the rule (the cells stay as they are).
    pub fn set_rule(&mut self, rule: LifeRule) {
        self.rule = rule;
    }

    /// The population history (oldest first, at most 120).
    pub fn history(&self) -> &History {
        &self.history
    }

    /// Generations since the last reset.
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// The mood of the population history.
    pub fn mood(&self) -> Mood {
        self.history.mood()
    }

    /// The numbers the readouts show.
    pub fn stats(&self) -> TrayStats {
        let mood = self.mood();
        TrayStats {
            generation: self.generation,
            population: self.grid.population(),
            born: self.born,
            died: self.died,
            mood_title: mood.title(),
            mood_text: mood.text(),
        }
    }

    /// Advances one generation.
    pub fn step(&mut self) {
        let s = self.grid.step(&self.rule, TRAY_BOUNDARY);
        self.generation += 1;
        self.born = s.born;
        self.died = s.died;
        self.history.push(s.population);
    }

    /// Sets one cell (Draw / Erase); the newest history value follows the edit, as the
    /// mockup's `edit()` does. Returns whether anything changed.
    pub fn set_cell(&mut self, x: usize, y: usize, alive: bool) -> bool {
        if x >= self.grid.width() || y >= self.grid.height() {
            return false;
        }
        // Draw makes a newborn even over a live cell (the mockup sets `cells[i] = 1`)
        let changed = self.grid.is_alive(x, y) != alive || alive;
        self.grid.set(x, y, alive);
        self.edited();
        changed
    }

    /// Paints every cell on the straight line from `from` to `to` (both included), so a fast
    /// drag doesn't leave gaps between the cells it was sampled on.
    pub fn paint_line(&mut self, from: (usize, usize), to: (usize, usize), alive: bool) {
        for (x, y) in line_cells(from, to) {
            self.grid.set(x, y, alive);
        }
        self.edited();
    }

    /// Stamps `pattern` centred on `(x, y)`, wrapping around the torus (Stamp tool).
    pub fn stamp(&mut self, pattern: &Pattern, x: usize, y: usize) {
        self.grid
            .place(pattern, x as i64, y as i64, Anchor::Center, TRAY_BOUNDARY);
        self.edited();
    }

    /// Kills every cell and starts the counters over (Clear).
    pub fn clear(&mut self) {
        self.grid.clear();
        self.reset_counters();
    }

    /// The starting garden under Life, counters reset (Restart).
    pub fn restart(&mut self) {
        *self = Model::default();
    }

    /// A 30 % soup from `seed`, counters reset (Random soup). The rule is kept.
    pub fn randomize(&mut self, seed: u64) {
        let mut rng = Rng::new(seed);
        self.grid = Grid::random(TRAY_W, TRAY_H, SOUP_DENSITY, &mut rng);
        self.reset_counters();
    }

    /// Clears the tray and places `pattern` centred on it (wrapping), counters reset. Returns
    /// whether the pattern had to be cropped to fit the tray (see [`fit_to_tray`]).
    pub fn load_pattern(&mut self, pattern: &Pattern) -> bool {
        let (fitted, cropped) = fit_to_tray(pattern);
        self.grid.clear();
        self.grid.place(
            &fitted,
            (TRAY_W / 2) as i64,
            (TRAY_H / 2) as i64,
            Anchor::Center,
            TRAY_BOUNDARY,
        );
        self.reset_counters();
        cropped
    }

    /// The whole tray (40 × 26, so positions survive a round trip) as RLE with `rule =`.
    pub fn rle(&self) -> String {
        let p = Pattern::from_grid_region(&self.grid, 0, 0, TRAY_W, TRAY_H);
        rle::write(&p, Some(&self.rule), None)
    }

    fn edited(&mut self) {
        self.history.replace_last(self.grid.population());
    }

    fn reset_counters(&mut self) {
        self.grid.clear_trails();
        self.generation = 0;
        self.born = 0;
        self.died = 0;
        self.history.reset(self.grid.population());
    }
}

/// `pattern` trimmed to its live cells and, if it is still larger than the tray, cut to the
/// tray-sized window around its centre (a carried 64 × 40 Rule-lab frame would otherwise
/// wrap onto itself). Returns the pattern and whether anything was cut away.
pub fn fit_to_tray(pattern: &Pattern) -> (Pattern, bool) {
    let p = pattern.trimmed();
    if p.width() <= TRAY_W && p.height() <= TRAY_H {
        return (p, false);
    }
    let x0 = p.width().saturating_sub(TRAY_W) / 2;
    let y0 = p.height().saturating_sub(TRAY_H) / 2;
    let cells = p
        .cells()
        .iter()
        .filter(|&&(x, y)| x >= x0 && y >= y0 && x < x0 + TRAY_W && y < y0 + TRAY_H)
        .map(|&(x, y)| (x - x0, y - y0));
    (
        Pattern::new(TRAY_W.min(p.width()), TRAY_H.min(p.height()), cells),
        true,
    )
}

/// The grid cells on the segment `from`–`to` (Bresenham, both ends included).
pub fn line_cells(from: (usize, usize), to: (usize, usize)) -> Vec<(usize, usize)> {
    let (mut x, mut y) = (from.0 as i64, from.1 as i64);
    let (x1, y1) = (to.0 as i64, to.1 as i64);
    let (dx, dy) = ((x1 - x).abs(), -(y1 - y).abs());
    let (sx, sy) = (if x < x1 { 1 } else { -1 }, if y < y1 { 1 } else { -1 });
    let mut err = dx + dy;
    let mut out = Vec::with_capacity((dx - dy) as usize + 1);
    loop {
        out.push((x as usize, y as usize));
        if x == x1 && y == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            err += dx;
            y += sy;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// SPEC REF-2: the starting garden at gen 30 has 90 alive, 27 born, 57 died.
    #[test]
    fn starting_garden_reaches_the_mockups_gen_30() {
        let mut m = Model::default();
        for _ in 0..30 {
            m.step();
        }
        let s = m.stats();
        assert_eq!(
            (s.generation, s.population, s.born, s.died),
            (30, 90, 27, 57)
        );
        assert_eq!(m.history().values().len(), 31);
        assert_eq!(s.mood_title, "Churning");
    }

    #[test]
    fn lines_have_no_gaps() {
        let l = line_cells((0, 0), (5, 2));
        assert_eq!(l.first(), Some(&(0, 0)));
        assert_eq!(l.last(), Some(&(5, 2)));
        for w in l.windows(2) {
            let (a, b) = (w[0], w[1]);
            assert!(a.0.abs_diff(b.0) <= 1 && a.1.abs_diff(b.1) <= 1, "{l:?}");
        }
        assert_eq!(line_cells((3, 3), (3, 3)), [(3, 3)]);
        assert_eq!(line_cells((4, 1), (1, 1)).len(), 4);
    }

    #[test]
    fn edits_update_the_newest_history_value() {
        let mut m = Model::default();
        m.clear();
        assert_eq!(m.history().values(), &[0]);
        m.set_cell(1, 1, true);
        m.paint_line((3, 0), (6, 0), true);
        assert_eq!(m.history().values(), &[5]);
        assert_eq!(m.grid().population(), 5);
        m.set_cell(1, 1, false);
        assert_eq!(m.history().latest(), 4);
        assert!(!m.set_cell(99, 0, true), "outside is ignored");
    }

    #[test]
    fn rle_round_trips_the_whole_tray() {
        let mut m = Model::default();
        for _ in 0..12 {
            m.step();
        }
        let text = m.rle();
        assert!(text.starts_with("x = 40, y = 26, rule = B3/S23"), "{text}");
        let back = ap_engine::pattern::parse_any(&text).unwrap();
        let mut other = Model::default();
        other.load_pattern(&back.pattern);
        let cells = |g: &Grid| g.alive_cells().collect::<Vec<_>>();
        assert_eq!(cells(other.grid()), cells(m.grid()));
    }

    #[test]
    fn big_patterns_are_cropped_to_the_tray() {
        let wide = Pattern::new(64, 3, (0..64).map(|x| (x, 1)));
        let (p, cropped) = fit_to_tray(&wide);
        assert!(cropped);
        assert_eq!((p.width(), p.height(), p.population()), (40, 1, 40));
        let glider = ap_engine::pattern::library::pattern("glider").unwrap();
        assert_eq!(fit_to_tray(&glider), (glider.clone(), false));
    }

    #[test]
    fn soups_differ_by_seed_and_keep_the_rule() {
        let mut m = Model::default();
        m.set_rule(LifeRule::moore(&[3, 6], &[2, 3]));
        m.randomize(1);
        let a = m.grid().clone();
        m.randomize(2);
        assert_ne!(&a, m.grid());
        let pop = m.grid().population() as f64 / (TRAY_W * TRAY_H) as f64;
        assert!((0.2..0.4).contains(&pop), "{pop}");
        assert_eq!(m.rule().code(), "B36/S23");
        assert_eq!(m.generation(), 0);
    }
}
