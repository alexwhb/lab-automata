//! SPEC REF-1: known facts about Life and elementary automata hold.

use std::collections::BTreeSet;

use ap_engine::pattern::library;
use ap_engine::{
    Anchor, Boundary, Elementary, Frame, Grid, LifeRule, Pattern, Rng, Weave, WeaveStart,
};

type Cells = BTreeSet<(usize, usize)>;

fn cells(grid: &Grid) -> Cells {
    grid.alive_cells().collect()
}

fn grid_with(pattern: &Pattern, w: usize, h: usize, x: i64, y: i64) -> Grid {
    let mut grid = Grid::new(w, h);
    grid.place(pattern, x, y, Anchor::TopLeft, Boundary::Dead);
    grid
}

/// The smallest p ≤ max with state(p) == state(0), on a dead-edge grid with a wide margin.
fn period(pattern: &Pattern, rule: &LifeRule, max: usize) -> Option<usize> {
    let mut grid = grid_with(pattern, pattern.width() + 24, pattern.height() + 24, 12, 12);
    let start = Frame::from_grid(&grid);
    (1..=max).find(|_| {
        grid.step(rule, Boundary::Dead);
        Frame::from_grid(&grid) == start
    })
}

#[test]
fn glider_moves_one_cell_diagonally_every_four_generations() {
    let glider = library::pattern("glider").unwrap();
    for boundary in Boundary::ALL {
        let mut grid = Grid::new(40, 40);
        grid.place(&glider, 2, 2, Anchor::TopLeft, boundary);
        let start = cells(&grid);
        for k in 1..=8 {
            for _ in 0..4 {
                grid.step(&LifeRule::LIFE, boundary);
            }
            let expect: Cells = start.iter().map(|&(x, y)| (x + k, y + k)).collect();
            assert_eq!(cells(&grid), expect, "{boundary:?} after {} gens", 4 * k);
        }
    }
    // And it keeps going round a torus.
    let mut grid = Grid::new(12, 12);
    grid.place(&glider, 0, 0, Anchor::TopLeft, Boundary::Torus);
    let start = cells(&grid);
    for _ in 0..48 {
        grid.step(&LifeRule::LIFE, Boundary::Torus);
    }
    assert_eq!(cells(&grid), start, "12 × 4 generations bring it home");
}

#[test]
fn oscillator_periods() {
    let blinker = Pattern::from_rows(&["OOO"]);
    assert_eq!(period(&blinker, &LifeRule::LIFE, 50), Some(2));
    assert_eq!(
        period(&library::pattern("pulsar").unwrap(), &LifeRule::LIFE, 50),
        Some(3)
    );
    assert_eq!(
        period(&library::pattern("penta").unwrap(), &LifeRule::LIFE, 50),
        Some(15)
    );
    let block = Pattern::from_rows(&["OO", "OO"]);
    assert_eq!(period(&block, &LifeRule::LIFE, 5), Some(1));
}

/// 8-connected components of a cell set, as sorted sizes.
fn component_sizes(cells: &Cells) -> Vec<usize> {
    let mut left = cells.clone();
    let mut sizes = Vec::new();
    while let Some(&first) = left.iter().next() {
        left.remove(&first);
        let mut stack = vec![first];
        let mut size = 0;
        while let Some((x, y)) = stack.pop() {
            size += 1;
            for dy in -1i64..=1 {
                for dx in -1i64..=1 {
                    let n = ((x as i64 + dx) as usize, (y as i64 + dy) as usize);
                    if left.remove(&n) {
                        stack.push(n);
                    }
                }
            }
        }
        sizes.push(size);
    }
    sizes.sort_unstable();
    sizes
}

#[test]
fn gosper_gun_has_period_30_and_emits_a_glider_every_30_generations() {
    let gun = library::pattern("gun").unwrap();
    let mut grid = grid_with(&gun, 140, 140, 1, 1);
    let start = cells(&grid);
    assert_eq!(start.len(), 36);
    for t in 1..=300 {
        grid.step(&LifeRule::LIFE, Boundary::Dead);
        if t % 30 == 0 {
            let k = t / 30;
            let now = cells(&grid);
            assert!(
                start.is_subset(&now),
                "the gun is back in its starting phase at gen {t}"
            );
            let gliders: Cells = now.difference(&start).copied().collect();
            assert_eq!(gliders.len(), 5 * k, "gen {t}");
            assert_eq!(
                component_sizes(&gliders),
                vec![5; k],
                "{k} separate gliders at gen {t}"
            );
        }
    }
}

#[test]
fn highlife_replicator_copies_itself() {
    let rep = library::pattern("rep").unwrap();
    let highlife = LifeRule::parse("B36/S23").unwrap();
    let mut grid = grid_with(&rep, 60, 60, 28, 28);
    for _ in 0..12 {
        grid.step(&highlife, Boundary::Dead);
    }
    // After 12 generations: exactly two replicators, two cells up-left and down-right.
    let copies: Cells = [(26, 26), (30, 30)]
        .iter()
        .flat_map(|&(ox, oy)| rep.cells().iter().map(move |&(x, y)| (x + ox, y + oy)))
        .collect();
    assert_eq!(copies.len(), 24);
    assert_eq!(cells(&grid), copies);
    // In Life it does not replicate.
    let mut life = grid_with(&rep, 60, 60, 28, 28);
    for _ in 0..12 {
        life.step(&LifeRule::LIFE, Boundary::Dead);
    }
    assert_ne!(cells(&life), copies);
}

#[test]
fn rule_30_centre_column_is_oeis_a051023() {
    const A051023: [bool; 20] = {
        let d = [1, 1, 0, 1, 1, 1, 0, 0, 1, 1, 0, 0, 0, 1, 0, 1, 1, 0, 0, 1];
        let mut out = [false; 20];
        let mut i = 0;
        while i < 20 {
            out[i] = d[i] == 1;
            i += 1;
        }
        out
    };
    let mut weave = Weave::with_size(201, 64, 30, WeaveStart::Single);
    let mut column = vec![weave.newest()[100]];
    for _ in 1..20 {
        column.push(weave.weave()[100]);
    }
    assert_eq!(column, A051023);
}

#[test]
fn rule_90_row_n_has_two_to_the_popcount_of_n_cells() {
    let mut weave = Weave::with_size(257, 8, 90, WeaveStart::Single);
    for n in 0u32..128 {
        if n > 0 {
            weave.weave();
        }
        let live = weave.newest().iter().filter(|&&c| c).count();
        assert_eq!(live, 1 << n.count_ones(), "row {n}");
    }
}

#[test]
fn rule_184_conserves_cars() {
    for seed in 0..20 {
        let mut weave = Weave::new(184, WeaveStart::Random(seed));
        let cars = weave.newest().iter().filter(|&&c| c).count();
        for _ in 0..300 {
            assert_eq!(
                weave.weave().iter().filter(|&&c| c).count(),
                cars,
                "seed {seed}"
            );
        }
    }
    // A random row of any width, any start.
    let mut rng = Rng::new(7);
    for _ in 0..50 {
        let width = 1 + rng.below(90);
        let row: Vec<bool> = (0..width).map(|_| rng.chance(0.4)).collect();
        let next = Elementary(184).next_row(&row);
        assert_eq!(
            next.iter().filter(|&&c| c).count(),
            row.iter().filter(|&&c| c).count()
        );
    }
}

#[test]
fn rule_150_is_xor_of_three_cells() {
    let mut rng = Rng::new(150);
    let row: Vec<bool> = (0..121).map(|_| rng.chance(0.5)).collect();
    let next = Elementary(150).next_row(&row);
    for i in 0..121 {
        assert_eq!(next[i], row[(i + 120) % 121] ^ row[i] ^ row[(i + 1) % 121]);
    }
}
