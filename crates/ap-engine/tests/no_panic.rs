//! SPEC ENG-8: no panics on any input. Random and garbled strings go to every parser, random
//! edits to every API; whatever parses must also round-trip. Seeded, so failures replay.

use ap_engine::analysis::{MAX_RUN_DIM, MAX_RUN_GENS};
use ap_engine::breed::{self, MUTATION_RANGE};
use ap_engine::pattern::{self, cells, library, rle};
use ap_engine::{
    Anchor, Boundary, CountSet, Elementary, Frame, Grid, History, LifeRule, Mood, Neighbourhood,
    NeighbourhoodPreset, Pattern, PatternFile, Probe, ProbeQuantity, Rng, RuleScript, RunSpec,
    Seed, Terrarium, Weave, WeaveStart, run,
};

const PIECES: &[&str] = &[
    "B",
    "S",
    "b",
    "s",
    "/",
    ",",
    ":",
    "T",
    "0",
    "1",
    "2",
    "3",
    "8",
    "9",
    "10",
    "24",
    "25",
    "99999999999999999999999",
    "o",
    "$",
    "!",
    ".",
    "O",
    "*",
    "A",
    "X",
    "p",
    "y",
    "#N ",
    "#C ",
    "#O ",
    "#",
    "x = ",
    "y = ",
    "rule = ",
    "=",
    "(",
    ")",
    "@",
    "->",
    "-",
    "+",
    " ",
    "  ",
    "\t",
    "\n",
    "\r\n",
    "·",
    "×",
    "é",
    "\u{feff}",
    "🦠",
    "\0",
    ".rule",
    ".neigh",
    ".grid",
    ".seed",
    ".run",
    ".probe",
    ".end",
    "life_like",
    "moore",
    "vn",
    "hex",
    "moore2",
    "custom",
    "n=",
    "64x40",
    "boundary=",
    "torus",
    "dead",
    "r_pentomino",
    "acorn",
    "soup",
    "seed=",
    "density=",
    "gens=",
    "population",
    "period",
    "plot1",
    "(1,1)",
    "(0,0)",
    "(3,0)",
    "(-2,2)",
    "NaN",
    "1e9",
    "-1",
    "0.5",
    "* automata netlist · ",
];

fn random_text(rng: &mut Rng) -> String {
    let n = rng.below(40);
    let mut s = String::new();
    for _ in 0..n {
        if rng.chance(0.15) {
            s.push(char::from_u32(rng.below(0x3000) as u32).unwrap_or('?'));
        } else {
            s.push_str(PIECES[rng.below(PIECES.len())]);
        }
    }
    s
}

/// A valid input with random damage: deletions, duplications, insertions, swaps.
fn garble(valid: &str, rng: &mut Rng) -> String {
    let mut chars: Vec<char> = valid.chars().collect();
    for _ in 0..1 + rng.below(4) {
        let len = chars.len();
        match rng.below(4) {
            0 if len > 0 => {
                let a = rng.below(len);
                let b = (a + 1 + rng.below(6)).min(len);
                chars.drain(a..b);
            }
            1 if len > 0 => {
                let a = rng.below(len);
                let b = (a + 1 + rng.below(8)).min(len);
                let copy: Vec<char> = chars[a..b].to_vec();
                let at = rng.below(chars.len() + 1);
                chars.splice(at..at, copy);
            }
            2 => {
                let at = rng.below(len + 1);
                let piece: Vec<char> = PIECES[rng.below(PIECES.len())].chars().collect();
                chars.splice(at..at, piece);
            }
            _ if len > 1 => {
                let (a, b) = (rng.below(len), rng.below(len));
                chars.swap(a, b);
            }
            _ => {}
        }
    }
    chars.into_iter().collect()
}

fn random_neighbourhood(rng: &mut Rng) -> Neighbourhood {
    match rng.below(3) {
        0 => NeighbourhoodPreset::ALL[rng.below(4)].neighbourhood(),
        _ => Neighbourhood::from_bits(rng.next_u64() as u32),
    }
}

fn random_rule(rng: &mut Rng) -> LifeRule {
    LifeRule::new(
        CountSet::from_bits(rng.next_u64() as u32),
        CountSet::from_bits(rng.next_u64() as u32),
        random_neighbourhood(rng),
    )
}

fn random_pattern(rng: &mut Rng, max: usize) -> Pattern {
    let (w, h) = (rng.below(max), rng.below(max));
    let density = rng.next_f64();
    let cells: Vec<(usize, usize)> = (0..h)
        .flat_map(|y| (0..w).map(move |x| (x, y)))
        .filter(|_| rng.chance(density))
        .collect();
    Pattern::new(w, h, cells)
}

fn random_seed(rng: &mut Rng) -> Seed {
    match rng.below(3) {
        0 => Seed::RPentomino,
        1 => Seed::Acorn,
        _ => Seed::Soup {
            seed: rng.next_u64() as u32,
            density: rng.next_f64(),
        },
    }
}

fn valid_inputs() -> Vec<String> {
    let mut v: Vec<String> = vec![
        "B3/S23".into(),
        "b36s23".into(),
        "23/3".into(),
        "B3,10/S2,3".into(),
        RuleScript::default().to_text(),
        "#N Glider\nx = 3, y = 3, rule = B3/S23\nbo$2bo$3o!".into(),
        "!Name: Glider\n.O.\n..O\nOOO".into(),
    ];
    for s in library::STAMPS {
        v.push(rle::write(
            &s.pattern(),
            Some(&LifeRule::LIFE),
            Some(s.name),
        ));
        v.push(cells::write(&s.pattern(), Some(s.name)));
    }
    v
}

/// How many inputs each parser accepted (to show the fuzzing reaches the success paths too).
#[derive(Default, Debug)]
struct Accepted {
    rules: usize,
    patterns: usize,
    scripts: usize,
}

fn check_rule_text(text: &str, rng: &mut Rng, accepted: &mut Accepted) {
    if let Ok(rule) = LifeRule::parse(text) {
        assert_eq!(LifeRule::parse(&rule.code()), Ok(rule), "{text:?}");
        accepted.rules += 1;
    }
    let nb = random_neighbourhood(rng);
    if let Ok(rule) = LifeRule::parse_with(text, nb) {
        assert!(rule.birth().max().unwrap_or(0) <= nb.len());
        assert_eq!(LifeRule::parse_with(&rule.code(), nb), Ok(rule), "{text:?}");
    }
    let _ = NeighbourhoodPreset::from_key(text);
    let _ = Boundary::from_key(text);
    let _ = ProbeQuantity::from_key(text);
}

fn check_pattern_text(text: &str, accepted: &mut Accepted) {
    let results = [
        rle::parse(text),
        cells::parse(text),
        pattern::parse_any(text),
        pattern::parse_file("x.rle", text),
        pattern::parse_file("x.cells", text),
        pattern::parse_file("x.txt", text),
    ];
    for file in results.into_iter().flatten() {
        let back = rle::parse(&rle::write_file(&file)).expect("our own RLE parses");
        assert_eq!(back.pattern, file.pattern, "{text:?}");
        let _ = file.life_rule();
        accepted.patterns += 1;
    }
}

fn check_script_text(text: &str, accepted: &mut Accepted) {
    match RuleScript::parse(text) {
        Ok(script) => {
            assert_eq!(
                RuleScript::parse(&script.to_text()),
                Ok(script.clone()),
                "{text:?}"
            );
            assert!(script.spec().validate().is_ok());
            accepted.scripts += 1;
        }
        Err(errors) => {
            assert!(!errors.is_empty());
            let lines = text.lines().count().max(1);
            assert!(
                errors.iter().all(|e| (1..=lines).contains(&e.line)),
                "{errors:?} for {text:?}"
            );
            assert!(errors.windows(2).all(|w| w[0].line <= w[1].line));
        }
    }
}

fn check_all(text: &str, rng: &mut Rng, accepted: &mut Accepted) {
    check_rule_text(text, rng, accepted);
    check_pattern_text(text, accepted);
    check_script_text(text, accepted);
}

#[test]
fn parsers_survive_random_strings() {
    let mut rng = Rng::new(0xE1A8);
    let mut accepted = Accepted::default();
    for _ in 0..20_000 {
        let text = random_text(&mut rng);
        check_all(&text, &mut rng, &mut accepted);
    }
    assert!(
        accepted.rules > 10 && accepted.patterns > 1000,
        "{accepted:?}"
    );
}

#[test]
fn parsers_survive_garbled_valid_inputs() {
    let mut rng = Rng::new(0x6A2B);
    let valid = valid_inputs();
    let mut accepted = Accepted::default();
    for _ in 0..20_000 {
        let base = &valid[rng.below(valid.len())];
        let text = garble(base, &mut rng);
        check_all(&text, &mut rng, &mut accepted);
    }
    assert!(
        accepted.rules > 100 && accepted.patterns > 1000 && accepted.scripts > 50,
        "{accepted:?}"
    );
}

#[test]
fn script_round_trips_for_random_valid_scripts() {
    let mut rng = Rng::new(0x5C41);
    let quantities = ProbeQuantity::ALL;
    for _ in 0..3_000 {
        let name: String = (0..rng.below(20))
            .map(|_| ['a', 'Z', '-', '.', '·', 'é', ' ', '_', '7', '*'][rng.below(10)])
            .collect();
        let script = RuleScript {
            name: name.trim().to_string(),
            rule: random_rule(&mut rng),
            w: 1 + rng.below(MAX_RUN_DIM),
            h: 1 + rng.below(MAX_RUN_DIM),
            boundary: Boundary::ALL[rng.below(2)],
            seed: random_seed(&mut rng),
            gens: 1 + rng.below(MAX_RUN_GENS),
            probes: (0..rng.below(4))
                .map(|k| {
                    Probe::new(
                        quantities[rng.below(quantities.len())],
                        format!("t_{k}.x-1"),
                    )
                })
                .collect(),
        };
        let text = script.to_text();
        assert_eq!(RuleScript::parse(&text), Ok(script), "{text}");
    }
}

#[test]
fn rle_round_trips_on_stamps_and_random_patterns() {
    for s in library::STAMPS {
        let p = s.pattern();
        let text = rle::write(&p, Some(&LifeRule::LIFE), Some(s.name));
        let f = rle::parse(&text).unwrap();
        assert_eq!(f.pattern, p, "{}", s.id);
        assert_eq!(f.name.as_deref(), Some(s.name));
        assert_eq!(f.life_rule(), Some(Ok(LifeRule::LIFE)));
        assert!(text.lines().all(|l| l.len() <= rle::LINE_WIDTH));
    }
    // Long body lines wrap between runs, never inside one.
    let wide = Pattern::new(0, 0, (0..500).filter(|x| x % 7 != 0).map(|x| (x, x % 3)));
    let text = rle::write(&wide, None, None);
    assert!(text.lines().skip(1).all(|l| l.len() <= rle::LINE_WIDTH));
    assert!(
        text.lines()
            .skip(1)
            .all(|l| !l.ends_with(|c: char| c.is_ascii_digit())),
        "{text}"
    );
    assert_eq!(rle::parse(&text).unwrap().pattern, wide);
    let mut rng = Rng::new(0x12E);
    for _ in 0..2_000 {
        let p = random_pattern(&mut rng, 120);
        let rule = random_rule(&mut rng);
        let file = PatternFile {
            pattern: p.clone(),
            name: rng.chance(0.5).then(|| "a name".to_string()),
            author: rng.chance(0.3).then(|| "someone".to_string()),
            comments: vec!["one".into(), "two".into()],
            rule: Some(rule.code()),
        };
        let text = rle::write_file(&file);
        let body = text
            .lines()
            .filter(|l| !l.starts_with('#') && !rle::is_header(l));
        assert!(
            body.clone().all(|l| l.chars().count() <= rle::LINE_WIDTH),
            "{text}"
        );
        assert!(body.count() >= 1);
        assert_eq!(rle::parse(&text), Ok(file));
        // Plaintext loses only the trailing margins.
        let plain = cells::parse(&cells::write(&p, None)).unwrap();
        assert_eq!(plain.pattern.trimmed(), p.trimmed());
    }
}

#[test]
fn grid_edits_and_steps_keep_their_invariants() {
    let mut rng = Rng::new(0x6A1D);
    let stamps: Vec<Pattern> = library::STAMPS.iter().map(|s| s.pattern()).collect();
    for _ in 0..300 {
        let mut grid = Grid::new(rng.below(48), rng.below(48));
        let rule = random_rule(&mut rng);
        for _ in 0..60 {
            let boundary = Boundary::ALL[rng.below(2)];
            match rng.below(8) {
                0 => {
                    grid.set(rng.below(60), rng.below(60), rng.chance(0.5));
                }
                1 => {
                    grid.toggle(rng.below(60), rng.below(60));
                }
                2 => {
                    let p = &stamps[rng.below(stamps.len())];
                    let big = [i64::MIN, i64::MAX, -1_000_000_007, 3, -3, 1 << 40];
                    let x = if rng.chance(0.2) {
                        big[rng.below(6)]
                    } else {
                        rng.below(80) as i64 - 20
                    };
                    let y = if rng.chance(0.2) {
                        big[rng.below(6)]
                    } else {
                        rng.below(80) as i64 - 20
                    };
                    let anchor = if rng.chance(0.5) {
                        Anchor::Center
                    } else {
                        Anchor::TopLeft
                    };
                    grid.place(p, x, y, anchor, boundary);
                }
                3 => grid.clear(),
                4 => {
                    let p = Pattern::from_grid_region(
                        &grid,
                        rng.below(70),
                        rng.below(70),
                        rng.below(70),
                        rng.below(usize::MAX),
                    );
                    assert!(p.width() <= grid.width() && p.height() <= grid.height());
                }
                _ => {
                    let before = grid.population();
                    let s = grid.step(&rule, boundary);
                    assert_eq!(s.population, before + s.born - s.died);
                    assert_eq!(s.population, grid.population());
                }
            }
            for (&a, &t) in grid.ages().iter().zip(grid.trails()) {
                assert!(t <= 3);
                assert!(a == 0 || t == 0, "a live cell has no trail");
            }
            if let Some(b) = grid.bbox() {
                assert!(b.x + b.w <= grid.width() && b.y + b.h <= grid.height());
            }
            let f = Frame::from_grid(&grid);
            assert_eq!(f.population(), grid.population());
            assert_eq!(Pattern::from_grid(&grid).population(), grid.population());
        }
    }
}

#[test]
fn rule_and_neighbourhood_editing_never_breaks_the_count_limit() {
    let mut rng = Rng::new(0x0B17);
    let mut rule = LifeRule::LIFE;
    for _ in 0..20_000 {
        match rng.below(6) {
            0 => {
                rule.toggle_birth(rng.below(40));
            }
            1 => {
                rule.toggle_survive(rng.below(40));
            }
            2 => {
                rule.toggle_neighbour(rng.below(9) as i32 - 4, rng.below(9) as i32 - 4);
            }
            3 => rule.set_neighbourhood(random_neighbourhood(&mut rng)),
            4 => {
                rule.set_birth(rng.below(usize::MAX), true);
            }
            _ => {
                let _ = Neighbourhood::from_offsets([(
                    rng.below(9) as i32 - 4,
                    rng.below(9) as i32 - 4,
                )]);
            }
        }
        let n = rule.neighbourhood().len();
        assert!(rule.birth().max().unwrap_or(0) <= n && rule.survive().max().unwrap_or(0) <= n);
        assert!(!rule.neighbourhood().contains(0, 0));
        assert_eq!(
            LifeRule::parse_with(&rule.code(), rule.neighbourhood()),
            Ok(rule)
        );
    }
}

#[test]
fn runs_with_random_and_extreme_specs() {
    let mut rng = Rng::new(0x2A7);
    for _ in 0..150 {
        let spec = RunSpec {
            rule: random_rule(&mut rng),
            boundary: Boundary::ALL[rng.below(2)],
            w: rng.below(40),
            h: rng.below(40),
            seed: match rng.below(4) {
                0 => Seed::Soup {
                    seed: rng.next_u64() as u32,
                    density: [f64::NAN, -1.0, 2.0, f64::INFINITY][rng.below(4)],
                },
                _ => random_seed(&mut rng),
            },
            gens: rng.below(60),
        };
        let r = run(&spec);
        assert_eq!(r.frames().len(), r.gens() + 1);
        let report = r.report();
        assert!(report.peak >= report.final_pop);
        if let Some(rep) = report.first_repeat {
            assert_eq!(r.frame(rep.start), r.frame(rep.seen_at()));
            assert!(rep.period >= 1);
        }
        for g in [0, r.gens(), r.gens() + 1, usize::MAX] {
            let _ = (r.rle(g), r.pattern(g), r.cell(g, 3, 3), r.population(g));
        }
        let _ = spec.validate();
    }
    let huge = RunSpec {
        w: usize::MAX,
        h: 1,
        gens: usize::MAX,
        ..RunSpec::default()
    };
    assert!(huge.validate().is_err());
    let r = run(&huge);
    assert_eq!((r.spec().w, r.gens()), (MAX_RUN_DIM, MAX_RUN_GENS));
}

#[test]
fn weaves_terrariums_and_histories_take_any_edit() {
    let mut rng = Rng::new(0x7EA);
    let mut weave = Weave::new(30, WeaveStart::Single);
    for _ in 0..5_000 {
        match rng.below(5) {
            0 => weave.set_rule(rng.below(256) as u8),
            1 => weave.set_start(if rng.chance(0.5) {
                WeaveStart::Single
            } else {
                WeaveStart::Random(rng.seed())
            }),
            2 => weave.restart(),
            _ => {
                weave.weave();
            }
        }
        assert!(weave.len() <= weave.keep() && !weave.is_empty());
        assert!(weave.rows().all(|r| r.len() == weave.width()));
    }
    for _ in 0..50 {
        let w = Weave::with_size(
            rng.below(300),
            rng.below(300),
            rng.below(256) as u8,
            WeaveStart::Random(rng.seed()),
        );
        assert!(w.width() >= 1 && w.keep() >= 1);
        let _ = Elementary(rng.below(256) as u8).next_row(&vec![true; rng.below(5)]);
    }

    let mut terrarium = Terrarium::with_rules(&[], 1);
    for _ in 0..400 {
        match rng.below(6) {
            0 => {
                terrarium.toggle_keep(rng.below(10));
            }
            1 => terrarium.breed(),
            2 => terrarium.reseed(),
            3 => terrarium.set_mutation(rng.below(usize::MAX)),
            _ => terrarium.tick(),
        }
        assert!(MUTATION_RANGE.contains(&terrarium.mutation()));
        assert!(terrarium.kept().iter().all(|&k| k < terrarium.jars().len()));
        for jar in terrarium.jars() {
            assert!((0.0..=1.0).contains(&jar.liveliness()));
            assert!(!jar.rule().birth().is_empty());
            assert_eq!(jar.rule().neighbourhood(), Neighbourhood::MOORE);
        }
        let _ = (
            terrarium.share_text(),
            terrarium.chosen_rule(),
            terrarium.keep_note(),
        );
    }
    let _ = breed::child(&[], usize::MAX, &mut rng);
    let _ = breed::breed(&[], &[3, 3, 99], 2, &mut rng);
    let b = breed::breed(&[LifeRule::LIFE; 6], &[5, 5, 99, 2], 2, &mut rng);
    assert_eq!(
        b.parents.len(),
        2,
        "repeated and missing indices are ignored"
    );

    let mut history = History::default();
    for _ in 0..5_000 {
        match rng.below(4) {
            0 => history.replace_last(rng.below(1000)),
            1 if rng.chance(0.01) => history.reset(rng.below(5)),
            _ => history.push(rng.below(1000)),
        }
        let _ = history.mood();
    }
    for _ in 0..2_000 {
        let v: Vec<usize> = (0..rng.below(140))
            .map(|_| {
                if rng.chance(0.1) {
                    usize::MAX
                } else {
                    rng.below(50)
                }
            })
            .collect();
        let _ = Mood::classify(&v);
    }
}
