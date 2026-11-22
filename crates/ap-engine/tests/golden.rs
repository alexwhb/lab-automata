//! SPEC REF-2: the mockup's own numbers, from goldens written by `scripts/mockup-goldens.mjs`
//! (which runs the mockup's `class Component` code verbatim). Regenerate with
//! `node scripts/mockup-goldens.mjs`.

use std::collections::BTreeMap;

use ap_engine::elementary::{self, FAMOUS};
use ap_engine::pattern::{library, rle};
use ap_engine::presets::{self, CARTRIDGES, NAMED, TERRARIUM_START};
use ap_engine::script::DEFAULT_NAME;
use ap_engine::{
    Boundary, CountSet, Elementary, History, LifeRule, Mood, NeighbourhoodPreset, Repeat,
    RuleScript, RunSpec, Seed, Weave, WeaveStart, run,
};
use serde::Deserialize;

fn load<T: for<'de> Deserialize<'de>>(name: &str) -> T {
    let path = format!("{}/tests/golden/{name}", env!("CARGO_MANIFEST_DIR"));
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{path}: {e}"))
}

// ------------------------------------------------------------------------------------- Tray

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TrayGolden {
    w: usize,
    h: usize,
    rule: String,
    rule_name: String,
    initial_pop: usize,
    initial_mood: String,
    gens: Vec<TrayGen>,
    snapshots: BTreeMap<usize, Snapshot>,
    moods: Vec<MoodCase>,
    carts: Vec<Cart>,
    stamps: Vec<StampGolden>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TrayGen {
    #[serde(rename = "gen")]
    generation: usize,
    pop: usize,
    born: usize,
    died: usize,
    mood: String,
    mood_text: String,
}

#[derive(Deserialize)]
struct Snapshot {
    age: Vec<u16>,
    trail: Vec<u8>,
}

#[derive(Deserialize)]
struct MoodCase {
    hist: Vec<usize>,
    mood: String,
    text: String,
}

#[derive(Deserialize)]
struct Cart {
    name: String,
    note: String,
    code: String,
}

#[derive(Deserialize)]
struct StampGolden {
    id: String,
    name: String,
    rows: Vec<String>,
}

#[test]
fn tray_starting_garden_matches_the_mockup_for_300_generations() {
    let g: TrayGolden = load("tray.json");
    let mut grid = presets::starting_garden();
    let rule = LifeRule::LIFE;
    assert_eq!((grid.width(), grid.height()), (g.w, g.h));
    assert_eq!(rule.code(), g.rule);
    assert_eq!(
        presets::matching_cartridge(&rule).map(|c| c.name),
        Some(g.rule_name.as_str())
    );
    assert_eq!(grid.population(), g.initial_pop);
    let mut history = History::new(grid.population());
    assert_eq!(history.mood().title(), g.initial_mood);

    let check = |grid: &ap_engine::Grid, generation: usize| {
        let snap = &g.snapshots[&generation];
        assert_eq!(grid.ages(), snap.age.as_slice(), "ages at gen {generation}");
        assert_eq!(
            grid.trails(),
            snap.trail.as_slice(),
            "trails at gen {generation}"
        );
    };
    check(&grid, 0);
    for (k, expect) in g.gens.iter().enumerate() {
        let stats = grid.step(&rule, presets::TRAY_BOUNDARY);
        history.push(stats.population);
        assert_eq!(expect.generation, k + 1);
        assert_eq!(
            (stats.population, stats.born, stats.died),
            (expect.pop, expect.born, expect.died),
            "gen {}",
            expect.generation
        );
        assert_eq!(stats.population, grid.population());
        let mood = history.mood();
        assert_eq!(
            (mood.title(), mood.text()),
            (expect.mood.as_str(), expect.mood_text.as_str()),
            "mood at gen {}",
            expect.generation
        );
        if g.snapshots.contains_key(&expect.generation) {
            check(&grid, expect.generation);
        }
    }
    // The anchor the spec quotes (REF-2): gen 30 has 90 alive, 27 born, 57 died.
    let g30 = &g.gens[29];
    assert_eq!((g30.pop, g30.born, g30.died), (90, 27, 57));
}

#[test]
fn mood_classifier_matches_the_mockup_on_synthetic_histories() {
    let g: TrayGolden = load("tray.json");
    let mut seen = std::collections::BTreeSet::new();
    for case in &g.moods {
        let mood = Mood::classify(&case.hist);
        assert_eq!(
            (mood.title(), mood.text()),
            (case.mood.as_str(), case.text.as_str()),
            "{:?}",
            case.hist
        );
        seen.insert(mood.title());
    }
    assert_eq!(seen.len(), 6, "every mood is exercised: {seen:?}");
}

#[test]
fn cartridges_and_stamps_are_the_mockups() {
    let g: TrayGolden = load("tray.json");
    assert_eq!(g.carts.len(), CARTRIDGES.len());
    for (c, golden) in CARTRIDGES.iter().zip(&g.carts) {
        assert_eq!(
            (c.name, c.note, c.rule.code()),
            (
                golden.name.as_str(),
                golden.note.as_str(),
                golden.code.clone()
            )
        );
    }
    assert_eq!(g.stamps.len(), library::STAMPS.len());
    for (s, golden) in library::STAMPS.iter().zip(&g.stamps) {
        assert_eq!((s.id, s.name), (golden.id.as_str(), golden.name.as_str()));
        assert_eq!(
            s.rows,
            golden
                .rows
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>()
                .as_slice()
        );
    }
}

// ---------------------------------------------------------------------------------- Rule lab

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LabGolden {
    w: usize,
    h: usize,
    gens: usize,
    cases: Vec<LabCase>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LabCase {
    seed: String,
    bound: String,
    neigh: String,
    rule_key: String,
    b: Vec<usize>,
    s: Vec<usize>,
    code: String,
    nb_name: String,
    bound_name: String,
    nb_count: usize,
    pops: Vec<usize>,
    first_repeat: i64,
    period: usize,
    repeat_exact: Option<bool>,
    report: Vec<String>,
    verdict: String,
    script: String,
    pop_now: usize,
    rle: BTreeMap<usize, String>,
}

impl LabCase {
    fn spec(&self, g: &LabGolden) -> RunSpec {
        let nb = NeighbourhoodPreset::from_key(&self.neigh)
            .expect("preset")
            .neighbourhood();
        RunSpec {
            rule: LifeRule::new(
                self.b.iter().copied().collect::<CountSet>(),
                self.s.iter().copied().collect::<CountSet>(),
                nb,
            ),
            boundary: Boundary::from_key(&self.bound).expect("boundary"),
            w: g.w,
            h: g.h,
            seed: match self.seed.as_str() {
                "rpent" => Seed::RPentomino,
                "acorn" => Seed::Acorn,
                _ => Seed::MOCKUP_SOUP,
            },
            gens: g.gens,
        }
    }

    fn label(&self) -> String {
        format!(
            "{} {} {} {}",
            self.seed, self.bound, self.neigh, self.rule_key
        )
    }
}

#[test]
fn rule_lab_matches_the_mockup_for_every_seed_boundary_and_neighbourhood() {
    let g: LabGolden = load("rulelab.json");
    assert_eq!(g.cases.len(), 3 * 2 * 4 * 4);
    let mut cycles = 0;
    for case in &g.cases {
        let label = case.label();
        let spec = case.spec(&g);
        let run = run(&spec);
        let report = run.report();

        assert_eq!(spec.rule.code(), case.code, "{label}");
        assert_eq!(spec.rule.neighbourhood().name(), case.nb_name, "{label}");
        assert_eq!(spec.rule.neighbourhood().len(), case.nb_count, "{label}");
        assert_eq!(spec.boundary.description(), case.bound_name, "{label}");
        assert_eq!(
            run.populations(),
            case.pops.as_slice(),
            "populations, {label}"
        );
        assert_eq!(run.population(120), Some(case.pop_now), "{label}");

        // The mockup's detection is hash-only; the generator checked each of its repeats cell by
        // cell. Where it was real, ours (hash + full compare) must find the same one.
        let mockup = (case.first_repeat >= 0).then(|| Repeat {
            start: case.first_repeat as usize,
            period: case.period,
        });
        match case.repeat_exact {
            Some(false) => {
                // A mockup hash collision: no such case in the current goldens. Ours must then
                // report a later (real) repeat or none, never the fake one.
                assert_ne!(run.first_repeat(), mockup, "{label}");
                continue;
            }
            _ => assert_eq!(run.first_repeat(), mockup, "{label}"),
        }
        cycles += usize::from(mockup.is_some());

        let texts = [
            report.peak_text(),
            report.final_pop.to_string(),
            report.first_repeat_text(),
            report.period_text(),
        ];
        assert_eq!(texts.as_slice(), case.report.as_slice(), "report, {label}");
        assert_eq!(report.verdict, case.verdict, "verdict, {label}");
        assert_eq!(
            RuleScript::from_spec(DEFAULT_NAME, &spec).to_text(),
            case.script,
            "script, {label}"
        );
        assert_eq!(
            RuleScript::parse(&case.script).map(|s| s.spec()),
            Ok(spec),
            "script parses, {label}"
        );

        for (&generation, text) in &case.rle {
            let mockup =
                rle::parse(text).unwrap_or_else(|e| panic!("{label} gen {generation}: {e}"));
            let ours = run.pattern(generation).expect("frame");
            assert_eq!(
                mockup.pattern, ours,
                "RLE cells at gen {generation}, {label}"
            );
            let header = text.lines().next().unwrap_or_default();
            let our_text = run.rle(generation).expect("frame");
            if ours.is_empty() {
                // The mockup writes `x = 0, y = 0` without a rule for an empty frame; we keep it.
                assert_eq!(header, "x = 0, y = 0", "{label}");
            } else {
                assert_eq!(mockup.rule.as_deref(), Some(case.code.as_str()), "{label}");
                assert_eq!(
                    our_text.lines().next(),
                    Some(header),
                    "RLE header at gen {generation}, {label}"
                );
            }
            assert_eq!(
                rle::parse(&our_text).map(|f| f.pattern),
                Ok(ours),
                "{label}"
            );
        }
    }
    assert!(cycles > 50, "most cases settle: {cycles}");
}

#[test]
fn rule_lab_anchor_values() {
    // SPEC REF-2's quoted numbers, independent of the JSON.
    let run = run(&RunSpec::default());
    let r = run.report();
    assert_eq!(run.population(120), Some(148));
    assert_eq!((r.peak, r.peak_gen, r.final_pop), (221, 148, 46));
    assert_eq!((r.first_repeat_gen, r.period), (Some(390), Some(2)));
    assert_eq!(r.verdict, "Settles into a period-2 cycle at gen 388.");
}

// -------------------------------------------------------------------------------------- Loom

#[derive(Deserialize)]
struct LoomGolden {
    w: usize,
    keep: usize,
    threads: Vec<Thread>,
    blurbs: Vec<Blurb>,
    famous: Vec<Famous>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Thread {
    rule: u8,
    rows: Vec<String>,
    count: u64,
    kept: usize,
    row_str: String,
    progress: String,
    blurb: String,
    binary: String,
}

#[derive(Deserialize)]
struct Blurb {
    rule: u8,
    blurb: String,
    binary: String,
    keys: Vec<bool>,
}

#[derive(Deserialize)]
struct Famous {
    n: u8,
    name: String,
    note: String,
}

fn row_string(row: &[bool]) -> String {
    row.iter().map(|&c| if c { '1' } else { '0' }).collect()
}

#[test]
fn loom_threads_match_the_mockup() {
    let g: LoomGolden = load("loom.json");
    for t in &g.threads {
        let mut weave = Weave::new(t.rule, WeaveStart::Single);
        assert_eq!((weave.width(), weave.keep()), (g.w, g.keep));
        assert_eq!(
            row_string(weave.newest()),
            t.rows[0],
            "rule {} row 0",
            t.rule
        );
        for (k, expect) in t.rows.iter().enumerate().skip(1) {
            assert_eq!(
                &row_string(weave.weave()),
                expect,
                "rule {} row {k}",
                t.rule
            );
        }
        assert_eq!((weave.count(), weave.len()), (t.count, t.kept));
        assert_eq!(format!("{:04}", weave.count()), t.row_str);
        assert_eq!(format!("{:.0}%", weave.progress() * 100.0), t.progress);
        assert_eq!(weave.rule().blurb(), t.blurb);
        assert_eq!(weave.rule().binary(), t.binary);
    }
}

#[test]
fn loom_blurbs_binaries_and_keys_for_all_256_rules() {
    let g: LoomGolden = load("loom.json");
    assert_eq!(g.blurbs.len(), 256);
    for b in &g.blurbs {
        let r = Elementary(b.rule);
        assert_eq!((r.blurb(), r.binary()), (b.blurb.clone(), b.binary.clone()));
        let keys: Vec<bool> = (0..8).rev().map(|p| r.output_for(p)).collect();
        assert_eq!(keys, b.keys, "rule {}", b.rule);
    }
    let famous: Vec<(u8, &str, &str)> = g
        .famous
        .iter()
        .map(|f| (f.n, f.name.as_str(), f.note.as_str()))
        .collect();
    let ours: Vec<(u8, &str, &str)> = FAMOUS.iter().map(|f| (f.rule, f.name, f.note)).collect();
    assert_eq!(ours, famous);
    assert_eq!(elementary::blurb(30), "Chaos · Random-looking");
}

// --------------------------------------------------------------------------------- Terrarium

#[derive(Deserialize)]
struct TerrariumGolden {
    named: BTreeMap<String, String>,
    start: Vec<String>,
    names: Vec<String>,
}

#[test]
fn terrarium_names_and_starting_rules_match_the_mockup() {
    let g: TerrariumGolden = load("terrarium.json");
    let ours: BTreeMap<String, String> = NAMED
        .iter()
        .map(|(n, r)| (r.code(), n.to_string()))
        .collect();
    assert_eq!(ours, g.named);
    let start: Vec<String> = TERRARIUM_START.iter().map(LifeRule::code).collect();
    assert_eq!(start, g.start);
    let names: Vec<&str> = TERRARIUM_START
        .iter()
        .map(|r| presets::rule_name(r).unwrap_or("Mutant"))
        .collect();
    assert_eq!(
        names,
        g.names.iter().map(String::as_str).collect::<Vec<_>>()
    );
}
