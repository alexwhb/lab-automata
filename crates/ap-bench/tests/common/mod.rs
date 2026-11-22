//! Demo data for ap-bench's tests and example: a minimal Life-like stepper that follows the
//! Tray mockup's `advance()` (ages, trails), the mockup's starting garden, an elementary CA and
//! a jar soup. Test-only: the app uses `ap-engine`.
#![allow(dead_code)]

use ap_bench::{CellVisual, TopCell};

/// The Tray mockup's Gosper gun and pulsar (`this.P`).
pub const GUN: [&str; 9] = [
    "........................O",
    "......................O.O",
    "............OO......OO............OO",
    "...........O...O....OO............OO",
    "OO........O.....O...OO",
    "OO........O...O.OO....O.O",
    "..........O.....O.......O",
    "...........O...O",
    "............OO",
];
pub const PULSAR: [&str; 13] = [
    "..OOO...OOO..",
    ".............",
    "O....O.O....O",
    "O....O.O....O",
    "O....O.O....O",
    "..OOO...OOO..",
    ".............",
    "..OOO...OOO..",
    "O....O.O....O",
    "O....O.O....O",
    "O....O.O....O",
    ".............",
    "..OOO...OOO..",
];
pub const RPENT: [&str; 3] = [".OO", "OO.", ".O."];

/// A torus of Life-like cells with ages and ghost trails.
#[derive(Clone)]
pub struct Garden {
    pub w: usize,
    pub h: usize,
    pub age: Vec<u16>,
    pub trail: Vec<u8>,
    pub birth: [bool; 9],
    pub survive: [bool; 9],
    pub generation: u64,
}

impl Garden {
    pub fn new(w: usize, h: usize) -> Garden {
        let mut birth = [false; 9];
        let mut survive = [false; 9];
        birth[3] = true;
        survive[2] = true;
        survive[3] = true;
        Garden {
            w,
            h,
            age: vec![0; w * h],
            trail: vec![0; w * h],
            birth,
            survive,
            generation: 0,
        }
    }

    /// The Tray's starting garden: a Gosper gun at (1, 1) and a pulsar at (26, 12), B3/S23.
    pub fn mockup() -> Garden {
        let mut g = Garden::new(40, 26);
        g.place(&GUN, 1, 1);
        g.place(&PULSAR, 26, 12);
        g
    }

    pub fn place(&mut self, rows: &[&str], x0: usize, y0: usize) {
        for (y, row) in rows.iter().enumerate() {
            for (x, ch) in row.chars().enumerate() {
                if ch == 'O' {
                    let i = ((y0 + y) % self.h) * self.w + (x0 + x) % self.w;
                    self.age[i] = 1;
                }
            }
        }
    }

    pub fn step(&mut self) {
        let (w, h) = (self.w, self.h);
        let mut age = vec![0u16; w * h];
        let mut trail = vec![0u8; w * h];
        for y in 0..h {
            for x in 0..w {
                let mut n = 0;
                for dy in [h - 1, 0, 1] {
                    for dx in [w - 1, 0, 1] {
                        if (dx, dy) != (0, 0) && self.age[((y + dy) % h) * w + (x + dx) % w] > 0 {
                            n += 1;
                        }
                    }
                }
                let i = y * w + x;
                let a = self.age[i];
                let v = if a > 0 {
                    if self.survive[n] {
                        a.saturating_add(1)
                    } else {
                        0
                    }
                } else if self.birth[n] {
                    1
                } else {
                    0
                };
                age[i] = v;
                trail[i] = if v > 0 {
                    0
                } else if a > 0 {
                    3
                } else {
                    self.trail[i].saturating_sub(1)
                };
            }
        }
        self.age = age;
        self.trail = trail;
        self.generation += 1;
    }

    pub fn run(&mut self, gens: usize) -> &mut Self {
        for _ in 0..gens {
            self.step();
        }
        self
    }

    pub fn alive(&self) -> usize {
        self.age.iter().filter(|&&a| a > 0).count()
    }

    pub fn visual(&self, c: usize, r: usize) -> CellVisual {
        let i = r * self.w + c;
        match self.age[i] {
            0 if self.trail[i] > 0 => CellVisual::Trail(self.trail[i]),
            0 => CellVisual::Empty,
            a => CellVisual::Live(a),
        }
    }

    /// Top-down kinds against the previous generation.
    pub fn top_cell(&self, prev: &Garden, x: usize, y: usize) -> TopCell {
        let i = y * self.w + x;
        match (prev.age[i] > 0, self.age[i] > 0) {
            (false, true) => TopCell::Born,
            (true, true) => TopCell::Alive,
            (true, false) => TopCell::Ghost,
            (false, false) => TopCell::Empty,
        }
    }
}

/// The Rule lab's default run: R-pentomino at (31, 19) on a 64 × 40 torus, B3/S23.
pub fn rule_lab(generation: usize) -> (Garden, Garden) {
    let mut g = Garden::new(64, 40);
    g.place(&RPENT, 31, 19);
    let mut prev = g.clone();
    for _ in 0..generation {
        prev = g.clone();
        g.step();
    }
    (prev, g)
}

/// An elementary CA weave: `rows` rows of `rule` from a single centre cell, 121 wide.
pub fn weave(rule: u8, rows: usize) -> Vec<Vec<bool>> {
    let w = 121;
    let mut row = vec![false; w];
    row[w / 2] = true;
    let mut out = vec![row];
    while out.len() < rows {
        let r = out.last().unwrap();
        let next = (0..w)
            .map(|i| {
                let p = (r[(i + w - 1) % w] as u8) << 2 | (r[i] as u8) << 1 | r[(i + 1) % w] as u8;
                (rule >> p) & 1 == 1
            })
            .collect();
        out.push(next);
    }
    out
}

/// A deterministic 32 × 32 jar soup (38 % in the centre 20 × 20), stepped `gens` times.
pub fn jar(seed: u64, gens: usize) -> Garden {
    let mut g = Garden::new(32, 32);
    let mut s = seed
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    for y in 6..26 {
        for x in 6..26 {
            s = s
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            if (s >> 33) as f64 / (1u64 << 31) as f64 * 0.5 < 0.38 * 0.5 {
                g.age[y * 32 + x] = 1;
            }
        }
    }
    g.run(gens);
    g
}
