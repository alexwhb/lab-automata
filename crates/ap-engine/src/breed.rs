//! The Terrarium: six jars of Life-like soups, their liveliness, and breeding new rules from
//! the kept ones (SPEC TERR-1…4, ENG-6).

use std::collections::VecDeque;

use crate::grid::Grid;
use crate::presets::{self, TERRARIUM_START};
use crate::rng::Rng;
use crate::rule::{Boundary, CountSet, LifeRule, Neighbourhood};

/// A jar's side (32 cells).
pub const JAR_SIZE: usize = 32;
/// How many jars the Terrarium has (6).
pub const JAR_COUNT: usize = 6;
/// The liveliness meter averages this many ticks (12).
pub const ACTIVITY_WINDOW: usize = 12;
/// Live-cell probability of a fresh soup (0.38), inside `5 < x < 26`, `5 < y < 26`.
pub const SOUP_DENSITY: f64 = 0.38;
/// The mutation slider's range (bit flips per child).
pub const MUTATION_RANGE: std::ops::RangeInclusive<usize> = 1..=4;
/// The mutation slider's default (2).
pub const DEFAULT_MUTATION: usize = 2;
/// The lineage keeps the last 8 broods.
pub const LINEAGE_LEN: usize = 8;

/// One jar: a 32 × 32 torus running its own Moore rule from a soup.
#[derive(Debug, Clone, PartialEq)]
pub struct Jar {
    rule: LifeRule,
    grid: Grid,
    activity: VecDeque<f64>,
}

impl Jar {
    /// A jar running `rule` (moved onto Moore r1) on a fresh soup.
    pub fn new(rule: LifeRule, rng: &mut Rng) -> Jar {
        let mut jar = Jar {
            rule: rule.with_neighbourhood(Neighbourhood::MOORE),
            grid: Grid::new(JAR_SIZE, JAR_SIZE),
            activity: VecDeque::with_capacity(ACTIVITY_WINDOW + 1),
        };
        jar.reseed(rng);
        jar
    }

    /// A fresh soup (same rule), and the activity window emptied: each cell with
    /// `5 < x < 26` and `5 < y < 26` is alive with probability 0.38.
    pub fn reseed(&mut self, rng: &mut Rng) {
        self.grid.clear();
        for y in 0..JAR_SIZE {
            for x in 0..JAR_SIZE {
                if x > 5 && x < 26 && y > 5 && y < 26 && rng.chance(SOUP_DENSITY) {
                    self.grid.set(x, y, true);
                }
            }
        }
        self.activity.clear();
    }

    /// The jar's rule.
    pub fn rule(&self) -> &LifeRule {
        &self.rule
    }

    /// The jar's cells.
    pub fn grid(&self) -> &Grid {
        &self.grid
    }

    /// Advances one generation; returns the fraction of the 1024 cells that changed (born +
    /// died) and records it in the activity window.
    pub fn tick(&mut self) -> f64 {
        let stats = self.grid.step(&self.rule, Boundary::Torus);
        let changed = (stats.born + stats.died) as f64 / (JAR_SIZE * JAR_SIZE) as f64;
        self.activity.push_back(changed);
        while self.activity.len() > ACTIVITY_WINDOW {
            self.activity.pop_front();
        }
        changed
    }

    /// The changed fractions of the last (up to) 12 ticks, oldest first.
    pub fn activity(&self) -> impl ExactSizeIterator<Item = f64> + '_ {
        self.activity.iter().copied()
    }

    /// The liveliness meter, `min(1, mean activity × 6)` (0 before the first tick).
    pub fn liveliness(&self) -> f64 {
        if self.activity.is_empty() {
            return 0.0;
        }
        let mean = self.activity.iter().sum::<f64>() / self.activity.len() as f64;
        (mean * 6.0).min(1.0)
    }

    /// Live cells.
    pub fn population(&self) -> usize {
        self.grid.population()
    }

    /// The rule's name, or "Mutant".
    pub fn name(&self) -> &'static str {
        presets::rule_name(&self.rule).unwrap_or("Mutant")
    }
}

/// How lively a jar looks (the meter's colour).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Liveliness {
    /// Below 0.08: grey.
    Quiet,
    /// In between: the accent.
    Lively,
    /// Above 0.8: coral.
    Frantic,
}

impl Liveliness {
    /// The band for a meter value, with the mockup's thresholds.
    pub fn of(value: f64) -> Liveliness {
        if value < 0.08 {
            Liveliness::Quiet
        } else if value > 0.8 {
            Liveliness::Frantic
        } else {
            Liveliness::Lively
        }
    }
}

/// A child rule (TERR-2, the mockup's `child()`): two parents are drawn at random (possibly the
/// same one), each of the 17 bits B1–B8, S0–S8 is copied from one of the two at random, then
/// `mutation` random bits among those 17 are flipped (the same bit may flip twice). A child
/// with no birth counts gets B3. With no parents, Life is the only parent. The child is Moore.
pub fn child(parents: &[LifeRule], mutation: usize, rng: &mut Rng) -> LifeRule {
    const LIFE: [LifeRule; 1] = [LifeRule::LIFE];
    let parents = if parents.is_empty() {
        &LIFE[..]
    } else {
        parents
    };
    let a = parents[rng.below(parents.len())];
    let b = parents[rng.below(parents.len())];
    // bits[0..8] = B1..=B8, bits[8..17] = S0..=S8.
    let mut bits = [false; 17];
    for (k, n) in (1..=8).enumerate() {
        bits[k] = if rng.chance(0.5) { a } else { b }.born_with(n);
    }
    for n in 0..=8 {
        bits[8 + n] = if rng.chance(0.5) { a } else { b }.survives_with(n);
    }
    for _ in 0..mutation.min(1024) {
        let i = rng.below(bits.len());
        bits[i] = !bits[i];
    }
    let mut birth: CountSet = (1..=8).filter(|&n| bits[n - 1]).collect();
    let survive: CountSet = (0..=8).filter(|&n| bits[8 + n]).collect();
    if birth.is_empty() {
        birth = CountSet::of(&[3]);
    }
    LifeRule::new(birth, survive, Neighbourhood::MOORE)
}

/// The rules of a new brood and the parents they came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Brood {
    /// The new jars' rules: the kept parents first, unchanged, then children.
    pub rules: Vec<LifeRule>,
    /// The parents (the kept jars in keep order, or every jar when none is kept).
    pub parents: Vec<LifeRule>,
}

/// Breeds a brood of `rules.len()` jars (TERR-2): the parents are the jars at `kept` (in that
/// order; out-of-range and repeated indices are ignored), or all of them when `kept` is empty;
/// the kept parents carry over unchanged in the first places and the rest are
/// [`child`]ren of the parents.
pub fn breed(rules: &[LifeRule], kept: &[usize], mutation: usize, rng: &mut Rng) -> Brood {
    let kept = valid_kept(kept, rules.len());
    let parents: Vec<LifeRule> = if kept.is_empty() {
        rules.to_vec()
    } else {
        kept.iter().map(|&k| rules[k]).collect()
    };
    let rules = (0..rules.len())
        .map(|i| {
            if i < kept.len() {
                parents[i]
            } else {
                child(&parents, mutation, rng)
            }
        })
        .collect();
    Brood { rules, parents }
}

fn valid_kept(kept: &[usize], len: usize) -> Vec<usize> {
    let mut out = Vec::new();
    for &k in kept {
        if k < len && !out.contains(&k) {
            out.push(k);
        }
    }
    out
}

/// One lineage line: a brood's number and its parents' codes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineageEntry {
    /// The brood the parents belonged to.
    pub brood: u32,
    /// The parents' rule codes.
    pub parents: Vec<String>,
}

/// The last 8 broods' parents (TERR-3).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Lineage {
    entries: VecDeque<LineageEntry>,
}

impl Lineage {
    /// Records a brood; the oldest beyond 8 is dropped.
    pub fn push(&mut self, entry: LineageEntry) {
        self.entries.push_back(entry);
        while self.entries.len() > LINEAGE_LEN {
            self.entries.pop_front();
        }
    }

    /// The entries, newest first (as listed).
    pub fn newest_first(&self) -> impl ExactSizeIterator<Item = &LineageEntry> {
        self.entries.iter().rev()
    }

    /// Number of entries (0–8).
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether nothing has been bred yet ("Nothing bred yet.").
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// The whole Terrarium bench: jars, keeps, brood counter, lineage, mutation and its own
/// seeded generator (so a fixed seed replays exactly).
#[derive(Debug, Clone, PartialEq)]
pub struct Terrarium {
    jars: Vec<Jar>,
    kept: Vec<usize>,
    brood: u32,
    lineage: Lineage,
    mutation: usize,
    rng: Rng,
}

impl Terrarium {
    /// The starting Terrarium: Life, HighLife, Day & Night, Maze, 2×2, Move on fresh soups;
    /// brood 1, nothing kept, mutation 2.
    pub fn new(seed: u64) -> Terrarium {
        Terrarium::with_rules(&TERRARIUM_START, seed)
    }

    /// A Terrarium with other rules (any number of jars, at least one: an empty list gives the
    /// starting rules).
    pub fn with_rules(rules: &[LifeRule], seed: u64) -> Terrarium {
        let rules = if rules.is_empty() {
            &TERRARIUM_START[..]
        } else {
            rules
        };
        let mut rng = Rng::new(seed);
        let jars = rules.iter().map(|&r| Jar::new(r, &mut rng)).collect();
        Terrarium {
            jars,
            kept: Vec::new(),
            brood: 1,
            lineage: Lineage::default(),
            mutation: DEFAULT_MUTATION,
            rng,
        }
    }

    /// The jars.
    pub fn jars(&self) -> &[Jar] {
        &self.jars
    }

    /// Jar `i`.
    pub fn jar(&self, i: usize) -> Option<&Jar> {
        self.jars.get(i)
    }

    /// The kept jars' indices, in the order they were kept.
    pub fn kept(&self) -> &[usize] {
        &self.kept
    }

    /// Whether jar `i` is kept.
    pub fn is_kept(&self, i: usize) -> bool {
        self.kept.contains(&i)
    }

    /// Keep / Kept: flips jar `i`; returns whether it is now kept (`false` for no such jar).
    pub fn toggle_keep(&mut self, i: usize) -> bool {
        if i >= self.jars.len() {
            return false;
        }
        if let Some(pos) = self.kept.iter().position(|&k| k == i) {
            self.kept.remove(pos);
            false
        } else {
            self.kept.push(i);
            true
        }
    }

    /// The current brood number (starts at 1).
    pub fn brood(&self) -> u32 {
        self.brood
    }

    /// The lineage.
    pub fn lineage(&self) -> &Lineage {
        &self.lineage
    }

    /// Bit flips per child (1–4).
    pub fn mutation(&self) -> usize {
        self.mutation
    }

    /// Sets the mutation, clamped to 1–4.
    pub fn set_mutation(&mut self, mutation: usize) {
        self.mutation = mutation.clamp(*MUTATION_RANGE.start(), *MUTATION_RANGE.end());
    }

    /// Ticks every jar once.
    pub fn tick(&mut self) {
        for jar in &mut self.jars {
            jar.tick();
        }
    }

    /// Fresh soup: re-seeds every jar, same rules.
    pub fn reseed(&mut self) {
        for jar in &mut self.jars {
            jar.reseed(&mut self.rng);
        }
    }

    /// The parents the next brood would use: the kept jars' rules, or every jar's.
    pub fn parents(&self) -> Vec<LifeRule> {
        if self.kept.is_empty() {
            self.jars.iter().map(|j| j.rule).collect()
        } else {
            self.kept.iter().map(|&k| self.jars[k].rule).collect()
        }
    }

    /// "Breed brood N+1": kept jars carry over (fresh soup), the rest become children; the
    /// lineage records this brood's parents; keeps are cleared; the brood number goes up.
    pub fn breed(&mut self) {
        let rules: Vec<LifeRule> = self.jars.iter().map(|j| j.rule).collect();
        let brood = breed(&rules, &self.kept, self.mutation, &mut self.rng);
        self.lineage.push(LineageEntry {
            brood: self.brood,
            parents: brood.parents.iter().map(LifeRule::code).collect(),
        });
        self.jars = brood
            .rules
            .iter()
            .map(|&r| Jar::new(r, &mut self.rng))
            .collect();
        self.kept.clear();
        self.brood = self.brood.saturating_add(1);
    }

    /// The breed button's label: "Breed brood N+1".
    pub fn breed_label(&self) -> String {
        format!("Breed brood {}", self.brood.saturating_add(1))
    }

    /// The note under it: "N kept as parents" or "None kept: all six become parents".
    pub fn keep_note(&self) -> String {
        if self.kept.is_empty() {
            "None kept: all six become parents".to_string()
        } else {
            format!("{} kept as parents", self.kept.len())
        }
    }

    /// The rule to carry to another screen (NAV-4): the first kept jar's, or jar 1's.
    pub fn chosen_rule(&self) -> LifeRule {
        let i = self.kept.first().copied().unwrap_or(0);
        self.jars.get(i).map_or(LifeRule::LIFE, |j| j.rule)
    }

    /// What Share copies (TERR-4): one line per jar, `"Jar 1 · B3/S23 · Life"`.
    pub fn share_text(&self) -> String {
        self.jars
            .iter()
            .enumerate()
            .map(|(i, j)| format!("Jar {} · {} · {}\n", i + 1, j.rule.code(), j.name()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jars_start_like_the_mockup() {
        let t = Terrarium::new(1);
        let names: Vec<&str> = t.jars().iter().map(Jar::name).collect();
        assert_eq!(
            names,
            ["Life", "HighLife", "Day & Night", "Maze", "2×2", "Move"]
        );
        for jar in t.jars() {
            let b = jar.grid().bbox().unwrap();
            assert!(b.x > 5 && b.y > 5 && b.x + b.w <= 26 && b.y + b.h <= 26);
            let p = jar.population() as f64 / 400.0;
            assert!((0.25..0.5).contains(&p), "{p}");
            assert_eq!(jar.liveliness(), 0.0);
        }
    }

    #[test]
    fn liveliness_is_the_scaled_mean() {
        let mut jar = Jar::new(LifeRule::LIFE, &mut Rng::new(5));
        let fractions: Vec<f64> = (0..20).map(|_| jar.tick()).collect();
        let last: Vec<f64> = fractions[8..].to_vec();
        assert_eq!(jar.activity().collect::<Vec<_>>(), last);
        let mean = last.iter().sum::<f64>() / 12.0;
        assert!((jar.liveliness() - (mean * 6.0).min(1.0)).abs() < 1e-12);
        assert_eq!(Liveliness::of(0.05), Liveliness::Quiet);
        assert_eq!(Liveliness::of(0.9), Liveliness::Frantic);
        assert_eq!(Liveliness::of(0.5), Liveliness::Lively);
    }

    #[test]
    fn children_inherit_bits_and_never_lack_births() {
        let mut rng = Rng::new(11);
        let parents = [LifeRule::LIFE];
        for _ in 0..200 {
            let c = child(&parents, 0, &mut rng);
            assert_eq!(c, LifeRule::LIFE, "one parent, no mutation: a clone");
        }
        let empty = [LifeRule::parse("B/S").unwrap()];
        for _ in 0..200 {
            let c = child(&empty, 1, &mut rng);
            assert!(!c.birth().is_empty());
            assert!(!c.born_with(0));
            let flipped = c.birth().len() + c.survive().len();
            assert!(c.birth() == CountSet::of(&[3]) || flipped == 1, "{c:?}");
        }
    }

    #[test]
    fn kept_jars_carry_over_first() {
        let mut t = Terrarium::new(3);
        assert_eq!(t.keep_note(), "None kept: all six become parents");
        assert!(t.toggle_keep(4));
        assert!(t.toggle_keep(1));
        assert!(!t.toggle_keep(9));
        assert_eq!(t.keep_note(), "2 kept as parents");
        assert_eq!(t.chosen_rule().code(), "B36/S125");
        assert_eq!(t.breed_label(), "Breed brood 2");
        t.breed();
        assert_eq!(t.jars()[0].rule().code(), "B36/S125");
        assert_eq!(t.jars()[1].rule().code(), "B36/S23");
        assert_eq!(t.brood(), 2);
        assert!(t.kept().is_empty());
        let e = t.lineage().newest_first().next().unwrap();
        assert_eq!(
            (e.brood, e.parents.clone()),
            (1, vec!["B36/S125".to_string(), "B36/S23".to_string()])
        );
    }

    #[test]
    fn lineage_keeps_eight() {
        let mut t = Terrarium::new(8);
        for _ in 0..11 {
            t.breed();
        }
        assert_eq!(t.lineage().len(), LINEAGE_LEN);
        assert_eq!(t.lineage().newest_first().next().map(|e| e.brood), Some(11));
        assert_eq!(t.lineage().newest_first().last().map(|e| e.brood), Some(4));
        t.set_mutation(99);
        assert_eq!(t.mutation(), 4);
        t.set_mutation(0);
        assert_eq!(t.mutation(), 1);
    }
}
