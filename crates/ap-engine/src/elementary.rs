//! Elementary (Wolfram) cellular automata: the Loom (SPEC ENG-4, LOOM-1…6).

use std::collections::VecDeque;

use crate::rng::Rng;

/// The Loom's row width (121 cells).
pub const LOOM_WIDTH: usize = 121;
/// How many of the newest rows the Loom keeps (72).
pub const LOOM_ROWS: usize = 72;
/// Widths and kept-row counts are clamped to `1..=MAX_WEAVE_DIM`.
pub const MAX_WEAVE_DIM: usize = 4096;

/// An elementary rule 0–255: bit `p` of the number is the new cell for the neighbourhood
/// pattern `p = left·4 + centre·2 + right`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, PartialOrd, Ord)]
pub struct Elementary(pub u8);

impl Elementary {
    /// The rule number.
    pub const fn number(self) -> u8 {
        self.0
    }

    /// The new cell for pattern `p` (only its low three bits are used).
    pub const fn output_for(self, p: u8) -> bool {
        (self.0 >> (p & 7)) & 1 == 1
    }

    /// The new cell for the three cells above it.
    pub const fn output(self, left: bool, centre: bool, right: bool) -> bool {
        self.output_for(((left as u8) << 2) | ((centre as u8) << 1) | right as u8)
    }

    /// The rule with pattern `p`'s output flipped (a Loom key press).
    pub const fn toggled(self, p: u8) -> Elementary {
        Elementary(self.0 ^ (1 << (p & 7)))
    }

    /// The next row; the row wraps around (the Loom's `weave()`).
    pub fn next_row(self, row: &[bool]) -> Vec<bool> {
        let mut out = Vec::with_capacity(row.len());
        self.next_row_into(row, &mut out);
        out
    }

    /// [`next_row`](Self::next_row) into a reused buffer.
    pub fn next_row_into(self, row: &[bool], out: &mut Vec<bool>) {
        out.clear();
        let w = row.len();
        out.extend((0..w).map(|i| self.output(row[(i + w - 1) % w], row[i], row[(i + 1) % w])));
    }

    /// The 8-bit binary, most significant first (`30` → `"00011110"`).
    pub fn binary(self) -> String {
        format!("{:08b}", self.0)
    }

    /// The famous thread with this number, if any.
    pub fn famous(self) -> Option<&'static FamousThread> {
        famous(self.0)
    }

    /// The blurb: `"<name> · <note>"` for a famous rule, else `"Uncharted rule"`.
    pub fn blurb(self) -> String {
        blurb(self.0)
    }
}

/// The next row of `rule` from `row`, wrapping (the Loom's `weave()` formula).
pub fn next_row(rule: u8, row: &[bool]) -> Vec<bool> {
    Elementary(rule).next_row(row)
}

/// The three cells (left, centre, right) of key pattern `p` (LOOM-3 shows keys 7 → 0).
pub const fn key_pattern(p: u8) -> [bool; 3] {
    [(p >> 2) & 1 == 1, (p >> 1) & 1 == 1, p & 1 == 1]
}

/// A famous rule in the Loom's list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FamousThread {
    /// The rule number.
    pub rule: u8,
    /// Its name ("Chaos").
    pub name: &'static str,
    /// The note ("Random-looking").
    pub note: &'static str,
}

/// The six famous threads, in the Loom's order (LOOM-5).
pub const FAMOUS: [FamousThread; 6] = [
    FamousThread {
        rule: 30,
        name: "Chaos",
        note: "Random-looking",
    },
    FamousThread {
        rule: 90,
        name: "Sierpiński",
        note: "Fractal triangles",
    },
    FamousThread {
        rule: 110,
        name: "Universal",
        note: "Turing-complete",
    },
    FamousThread {
        rule: 184,
        name: "Traffic",
        note: "Traffic flow",
    },
    FamousThread {
        rule: 54,
        name: "Particles",
        note: "Colliding particles",
    },
    FamousThread {
        rule: 150,
        name: "Lace",
        note: "XOR of three cells",
    },
];

/// The famous thread for `rule`, if any.
pub fn famous(rule: u8) -> Option<&'static FamousThread> {
    FAMOUS.iter().find(|f| f.rule == rule)
}

/// The rule panel's blurb (LOOM-4): `"Chaos · Random-looking"` or `"Uncharted rule"`.
pub fn blurb(rule: u8) -> String {
    match famous(rule) {
        Some(f) => format!("{} · {}", f.name, f.note),
        None => "Uncharted rule".to_string(),
    }
}

/// How a thread's first row starts (LOOM-6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WeaveStart {
    /// One live cell at the centre, `⌊width / 2⌋`.
    Single,
    /// Each cell alive with probability ½, from this seed.
    Random(u64),
}

impl WeaveStart {
    /// The first row for a given width.
    pub fn first_row(self, width: usize) -> Vec<bool> {
        match self {
            WeaveStart::Single => {
                let mut row = vec![false; width];
                if let Some(c) = row.get_mut(width / 2) {
                    *c = true;
                }
                row
            }
            WeaveStart::Random(seed) => {
                let mut rng = Rng::new(seed);
                (0..width).map(|_| rng.chance(0.5)).collect()
            }
        }
    }
}

/// A thread being woven: the newest rows (at most `keep`) of an elementary CA, plus the total
/// number of rows woven since the start (the `row 0000` counter counts the first row too).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Weave {
    rule: Elementary,
    start: WeaveStart,
    width: usize,
    keep: usize,
    rows: VecDeque<Vec<bool>>,
    count: u64,
}

impl Weave {
    /// The Loom's thread: 121 cells, the newest 72 rows kept.
    pub fn new(rule: u8, start: WeaveStart) -> Weave {
        Weave::with_size(LOOM_WIDTH, LOOM_ROWS, rule, start)
    }

    /// A thread of any width and history length (each clamped to `1..=MAX_WEAVE_DIM`).
    pub fn with_size(width: usize, keep: usize, rule: u8, start: WeaveStart) -> Weave {
        let mut weave = Weave {
            rule: Elementary(rule),
            start,
            width: width.clamp(1, MAX_WEAVE_DIM),
            keep: keep.clamp(1, MAX_WEAVE_DIM),
            rows: VecDeque::new(),
            count: 0,
        };
        weave.restart();
        weave
    }

    /// The rule.
    pub fn rule(&self) -> Elementary {
        self.rule
    }

    /// How the first row starts.
    pub fn start(&self) -> WeaveStart {
        self.start
    }

    /// Cells per row.
    pub fn width(&self) -> usize {
        self.width
    }

    /// How many rows are kept.
    pub fn keep(&self) -> usize {
        self.keep
    }

    /// The kept rows, oldest first.
    pub fn rows(&self) -> impl ExactSizeIterator<Item = &[bool]> + DoubleEndedIterator {
        self.rows.iter().map(Vec::as_slice)
    }

    /// The newest row.
    pub fn newest(&self) -> &[bool] {
        self.rows.back().map_or(&[], Vec::as_slice)
    }

    /// Number of kept rows (1 to `keep`).
    pub fn len(&self) -> usize {
        self.rows.len()
    }

    /// Always `false`: a thread has at least its first row. (Here for API completeness.)
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// Rows woven since the start, including the first (the `row 0000` counter).
    pub fn count(&self) -> u64 {
        self.count
    }

    /// How full the fabric is, kept rows / `keep`, in `0..=1` (the print head's progress bar).
    pub fn progress(&self) -> f64 {
        self.rows.len() as f64 / self.keep as f64
    }

    /// Weaves one row from the newest; drops the oldest beyond `keep`. Returns the new row.
    pub fn weave(&mut self) -> &[bool] {
        let next = self.rule.next_row(self.newest());
        self.rows.push_back(next);
        while self.rows.len() > self.keep {
            self.rows.pop_front();
        }
        self.count += 1;
        self.newest()
    }

    /// Changes the rule and restarts the thread (LOOM-4).
    pub fn set_rule(&mut self, rule: u8) {
        self.rule = Elementary(rule);
        self.restart();
    }

    /// Changes how the thread starts and restarts it.
    pub fn set_start(&mut self, start: WeaveStart) {
        self.start = start;
        self.restart();
    }

    /// Starts over from the first row ("New thread").
    pub fn restart(&mut self) {
        self.rows.clear();
        self.rows.push_back(self.start.first_row(self.width));
        self.count = 1;
    }

    /// The kept rows as text, one line per row, oldest first.
    pub fn rows_text(&self, alive: char, dead: char) -> String {
        let mut out = String::new();
        for row in &self.rows {
            out.extend(row.iter().map(|&c| if c { alive } else { dead }));
            out.push('\n');
        }
        out
    }

    /// What Share copies (LOOM-6): `"Rule N"` and the kept rows, `#` alive and `.` dead.
    pub fn share_text(&self) -> String {
        format!("Rule {}\n{}", self.rule.0, self.rows_text('#', '.'))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rule_bits_and_keys() {
        let r = Elementary(30);
        assert_eq!(r.binary(), "00011110");
        assert!(r.output(true, false, false) && !r.output(true, true, true));
        assert_eq!(r.toggled(7), Elementary(158));
        assert_eq!(key_pattern(6), [true, true, false]);
        assert_eq!(blurb(110), "Universal · Turing-complete");
        assert_eq!(blurb(31), "Uncharted rule");
    }

    #[test]
    fn weave_keeps_the_newest_rows() {
        let mut w = Weave::new(90, WeaveStart::Single);
        assert_eq!((w.len(), w.count()), (1, 1));
        assert!(w.newest()[60]);
        for _ in 0..100 {
            w.weave();
        }
        assert_eq!((w.len(), w.count()), (LOOM_ROWS, 101));
        assert!((w.progress() - 1.0).abs() < 1e-12);
        w.set_rule(30);
        assert_eq!((w.len(), w.count()), (1, 1));
        let a = Weave::new(30, WeaveStart::Random(9));
        assert_eq!(a, Weave::new(30, WeaveStart::Random(9)));
        assert!(w.share_text().starts_with("Rule 30\n"));
    }

    #[test]
    fn tiny_rows_wrap_onto_themselves() {
        assert_eq!(next_row(1, &[false]), [true]);
        assert!(next_row(30, &[]).is_empty());
        let mut w = Weave::with_size(0, 0, 30, WeaveStart::Single);
        w.weave();
        assert_eq!((w.width(), w.len()), (1, 1));
    }
}
