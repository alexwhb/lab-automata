//! Headless end-to-end scenarios (SPEC DONE-4): what each screen does, driven through the
//! engine's public API only.

use ap_engine::analysis::{LAB_H, LAB_W};
use ap_engine::pattern::{self, library};
use ap_engine::presets::{self, TRAY_BOUNDARY, TRAY_H, TRAY_W};
use ap_engine::{
    Anchor, Boundary, CellChange, Elementary, Grid, History, LifeRule, Mood, Neighbourhood,
    NeighbourhoodPreset, Pattern, Rng, RuleScript, Seed, Terrarium, Weave, WeaveStart, run,
};

/// One Tray "gen" tick, as the screen does it.
fn tray_step(grid: &mut Grid, rule: &LifeRule, history: &mut History) -> ap_engine::StepStats {
    let stats = grid.step(rule, TRAY_BOUNDARY);
    history.push(stats.population);
    stats
}

#[test]
fn tray_garden_over_1000_generations() {
    let mut grid = presets::starting_garden();
    let rule = presets::cartridge("life").unwrap().rule;
    let mut history = History::new(grid.population());
    let mut settled_at = None;
    for g in 1..=1000 {
        let stats = tray_step(&mut grid, &rule, &mut history);
        assert_eq!(stats.population, history.latest());
        if settled_at.is_none() && history.mood() == Mood::Settled {
            settled_at = Some(g);
        }
    }
    // On the 40 × 26 torus the gun's gliders come round and wreck the garden. What is left is
    // three blocks, a beehive and two blinkers: the population is constant, so the mockup's
    // population-only classifier says "Settled" even though the blinkers still blink.
    assert_eq!(settled_at, Some(320));
    assert_eq!(history.mood(), Mood::Settled);
    assert_eq!(history.latest(), 24);
    let then = grid.clone();
    tray_step(&mut grid, &rule, &mut history);
    assert_ne!(
        grid.alive_cells().collect::<Vec<_>>(),
        then.alive_cells().collect::<Vec<_>>()
    );
    tray_step(&mut grid, &rule, &mut history);
    assert_eq!(
        grid.alive_cells().collect::<Vec<_>>(),
        then.alive_cells().collect::<Vec<_>>(),
        "period 2"
    );
    let elders = grid.ages().iter().filter(|&&a| a > 600).count();
    assert_eq!(
        elders,
        3 * 4 + 6 + 2,
        "the blocks, the beehive and the blinkers' centres"
    );
    assert_eq!(history.values().len(), 120);
}

#[test]
fn tray_drawing_stamping_sharing_and_pasting() {
    let mut grid = presets::starting_garden();
    let mut rule = LifeRule::LIFE;
    let mut history = History::new(grid.population());
    for _ in 0..30 {
        tray_step(&mut grid, &rule, &mut history);
    }
    assert_eq!(grid.population(), 90);

    // Draw: press on a dead cell paints alive, drag paints the same value.
    let (x, y) = (0..TRAY_W)
        .map(|x| (x, 25))
        .find(|&(x, y)| !grid.is_alive(x, y))
        .unwrap();
    let paint = !grid.is_alive(x, y);
    for dx in 0..3 {
        grid.set((x + dx) % TRAY_W, y, paint);
    }
    history.replace_last(grid.population());
    assert!(grid.is_alive(x, y) && grid.age(x, y) == 1);

    // Stamp a glider centred on the corner: it wraps round the torus.
    let glider = library::pattern("glider").unwrap();
    grid.clear();
    grid.place(&glider, 0, 0, Anchor::Center, TRAY_BOUNDARY);
    assert_eq!(grid.population(), 5);
    assert!(grid.is_alive(TRAY_W - 1, 1), "wrapped to the far edge");

    // A custom rule (no cartridge lit), then back to HighLife by its bits.
    rule.toggle_survive(2);
    assert!(presets::matching_cartridge(&rule).is_none());
    rule.toggle_survive(2);
    rule.toggle_birth(6);
    assert_eq!(
        presets::matching_cartridge(&rule).map(|c| c.name),
        Some("HighLife")
    );

    // Share → clipboard RLE → paste → armed as a stamp → placed in a fresh tray.
    let shared = pattern::rle::write(&Pattern::from_grid(&grid), Some(&rule), None);
    assert!(shared.contains("rule = B36/S23"));
    let pasted = pattern::parse_any(&shared).unwrap();
    assert_eq!(pasted.life_rule(), Some(Ok(rule)));
    let mut fresh = Grid::new(TRAY_W, TRAY_H);
    fresh.place(&pasted.pattern, 10, 10, Anchor::TopLeft, TRAY_BOUNDARY);
    assert_eq!(fresh.population(), 5);

    // Random soup (30 %) then Clear.
    let mut rng = Rng::new(9);
    let soup = Grid::random(TRAY_W, TRAY_H, 0.3, &mut rng);
    let p = soup.population() as f64 / (TRAY_W * TRAY_H) as f64;
    assert!((0.25..0.35).contains(&p), "{p}");
    let mut cleared = soup.clone();
    cleared.clear();
    history.reset(cleared.population());
    assert_eq!(history.mood(), Mood::Extinct);
}

#[test]
fn terrarium_breeds_five_broods_deterministically() {
    fn play(seed: u64) -> Terrarium {
        let mut t = Terrarium::new(seed);
        t.set_mutation(3);
        for brood in 0..5 {
            for _ in 0..20 {
                t.tick();
            }
            // Keep the liveliest jar (and the second on odd broods), then breed.
            let mut order: Vec<usize> = (0..t.jars().len()).collect();
            order.sort_by(|&a, &b| {
                t.jars()[b]
                    .liveliness()
                    .total_cmp(&t.jars()[a].liveliness())
            });
            t.toggle_keep(order[0]);
            if brood % 2 == 1 {
                t.toggle_keep(order[1]);
            }
            let kept: Vec<LifeRule> = t.kept().iter().map(|&k| *t.jars()[k].rule()).collect();
            let label = t.breed_label();
            t.breed();
            assert_eq!(label, format!("Breed brood {}", t.brood()));
            for (i, rule) in kept.iter().enumerate() {
                assert_eq!(t.jars()[i].rule(), rule, "kept jars carry over first");
                assert_eq!(t.jars()[i].liveliness(), 0.0, "with a fresh soup");
            }
        }
        t
    }
    let a = play(42);
    assert_eq!(a, play(42), "same seed, same Terrarium");
    assert_ne!(a, play(43));
    assert_eq!(a.brood(), 6);
    let broods: Vec<u32> = a.lineage().newest_first().map(|e| e.brood).collect();
    assert_eq!(broods, [5, 4, 3, 2, 1]);
    let parents: Vec<usize> = a
        .lineage()
        .newest_first()
        .map(|e| e.parents.len())
        .collect();
    assert_eq!(parents, [1, 2, 1, 2, 1]);
    for jar in a.jars() {
        assert!(!jar.rule().birth().is_empty());
        assert!(jar.population() > 0);
        let _ = jar.name();
    }
    assert_eq!(a.share_text().lines().count(), 6);
    // NAV-4: nothing kept → jar 1's rule goes to the Tray / Rule lab.
    assert_eq!(a.chosen_rule(), *a.jars()[0].rule());
}

#[test]
fn rule_lab_script_edit_run_report_export() {
    // The default script runs the mockup's spec.
    let mut script = RuleScript::default();
    let mut spec = script.spec();
    let report = run(&spec).report();
    assert_eq!(report.verdict, "Settles into a period-2 cycle at gen 388.");

    // Edit the text: HighLife from an acorn on dead edges.
    let edited = script
        .to_text()
        .replace("B3/S23", "B36/S23")
        .replace("r_pentomino", "acorn")
        .replace("boundary=torus", "boundary=dead");
    script = RuleScript::parse(&edited).unwrap();
    spec = script.spec();
    assert_eq!(spec.rule, LifeRule::parse("B36/S23").unwrap());
    assert_eq!((spec.seed, spec.boundary), (Seed::Acorn, Boundary::Dead));
    let r = run(&spec);
    let report = r.report();
    assert_eq!(report.final_pop, r.populations()[400]);
    assert!(!report.verdict.is_empty());

    // A broken edit reports its line; the screen keeps the last valid script.
    let broken = edited.replace("gens=400", "gens=lots");
    let errors = RuleScript::parse(&broken).unwrap_err();
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].line, 6);
    assert_eq!(script.spec(), spec, "still the last valid spec");

    // Edit the spec (toggle a neighbour; counts above N drop) → the text is rewritten.
    let mut rule = spec.rule;
    for (dx, dy) in [(-1, -1), (1, -1), (-1, 1), (1, 1)] {
        rule.toggle_neighbour(dx, dy);
    }
    assert_eq!(
        rule.neighbourhood().preset(),
        Some(NeighbourhoodPreset::VonNeumann)
    );
    assert_eq!(rule.code(), "B3/S23", "B6 dropped with 4 neighbours");
    rule.toggle_neighbour(2, 0);
    spec.rule = rule;
    script.set_spec(&spec);
    let text = script.to_text();
    assert!(
        text.contains(".neigh  custom  n=5  (0,-1) (-1,0) (1,0) (2,0) (0,1)"),
        "{text}"
    );
    assert_eq!(RuleScript::parse(&text), Ok(script.clone()));

    // Scrub, export RLE, open in the Playground (NAV-4): the scrubbed cells go to the Tray.
    let r = run(&spec);
    let scrub = 120.min(r.gens());
    let rle = r.rle(scrub).unwrap();
    assert!(rle.starts_with("x = ") && rle.contains("rule = B3/S23"));
    let frame = r.frame(scrub).unwrap();
    let mut changes = [0usize; 4];
    for y in 0..LAB_H {
        for x in 0..LAB_W {
            changes[match r.cell(scrub, x, y) {
                CellChange::Empty => 0,
                CellChange::Born => 1,
                CellChange::Alive => 2,
                CellChange::Ghost => 3,
            }] += 1;
        }
    }
    assert_eq!(changes[1] + changes[2], frame.population());
    let cells = r.pattern(scrub).unwrap();
    let mut tray = Grid::new(TRAY_W, TRAY_H);
    tray.place(
        &cells,
        (TRAY_W / 2) as i64,
        (TRAY_H / 2) as i64,
        Anchor::Center,
        TRAY_BOUNDARY,
    );
    assert!(tray.population() <= cells.population());
}

#[test]
fn rule_lab_neighbourhood_presets_and_soup() {
    for preset in NeighbourhoodPreset::ALL {
        let rule = LifeRule::LIFE.with_neighbourhood(preset.neighbourhood());
        for seed in [Seed::RPentomino, Seed::Acorn, Seed::MOCKUP_SOUP] {
            let spec = ap_engine::RunSpec {
                rule,
                seed,
                ..Default::default()
            };
            let r = run(&spec);
            assert_eq!(r.frames().len(), 401);
            let report = r.report();
            assert_eq!(
                report.final_pop,
                r.populations()[400],
                "{preset:?} {seed:?}"
            );
        }
    }
    let empty = LifeRule::LIFE.with_neighbourhood(Neighbourhood::empty());
    assert_eq!(empty.code(), "B/S", "no neighbours, no counts above 0");
}

#[test]
fn loom_keys_presets_and_share() {
    let mut weave = Weave::new(30, WeaveStart::Single);
    for _ in 0..80 {
        weave.weave();
    }
    assert_eq!(weave.count(), 81);
    // Press key 111 (bit 7) → rule 158, restarted.
    weave.set_rule(weave.rule().toggled(7).number());
    assert_eq!(
        (weave.rule(), weave.count(), weave.len()),
        (Elementary(158), 1, 1)
    );
    assert_eq!(weave.rule().blurb(), "Uncharted rule");
    // −1 / +1 wrap.
    weave.set_rule(0u8.wrapping_sub(1));
    assert_eq!(weave.rule().number(), 255);
    // Famous threads.
    for f in ap_engine::elementary::FAMOUS {
        weave.set_rule(f.rule);
        assert_eq!(weave.rule().famous(), Some(&f));
    }
    // Random start, share as text.
    let mut rng = Rng::new(1);
    weave.set_start(WeaveStart::Random(rng.seed()));
    for _ in 0..5 {
        weave.weave();
    }
    let share = weave.share_text();
    assert!(share.starts_with("Rule 150\n"));
    assert_eq!(share.lines().count(), 1 + 6);
    assert!(share.lines().skip(1).all(|l| l.chars().count() == 121));
}
