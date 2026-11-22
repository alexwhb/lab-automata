//! The Rule lab (SCR-4, LAB-1…5, NAV-4, DONE-2/3) driven through AccessKit like a user: the
//! default analysis is the mockup's (and the engine's), the spec controls re-run and rewrite
//! the script, typing into the script updates the controls, errors are shown per line and
//! recover, scrub / Play, Export (clipboard and files), Open in Playground, the carry-over from
//! the Terrarium, and renders at 1440 × 960 and 390.

mod common;

use ap_engine::{Boundary, LifeRule, Report, RuleScript, RunSpec, Seed, analysis};
use automata_playground::Route;
use automata_playground::ctx::Carry;
use common::*;
use eframe::egui::{Key, Modifiers, accesskit::Role};
use egui_kittest::kittest::{NodeT, Queryable};

/// The Rule lab at the mockup's 1440 × 960 board.
fn lab() -> App {
    app().at(Route::RuleLab).board().build()
}

/// The engine's report for `spec` (what the screen must show, DONE-2).
fn engine(spec: &RunSpec) -> Report {
    analysis::run(spec).report()
}

fn spec(h: &App) -> RunSpec {
    h.state().screens().rule_lab.spec()
}

/// The script editor (the `.lbl` above it has the same words).
fn editor(h: &App) -> egui_kittest::Node<'_> {
    h.get_by_role_and_label(Role::MultilineTextInput, "Rule script")
}

fn script(h: &App) -> String {
    editor(h).value().unwrap_or_default()
}

/// The four analysis tiles and the verdict show `r`.
fn assert_analysis(h: &App, r: &Report) {
    for label in [
        format!("Peak population: {}", r.peak_text()),
        format!("Final population: {}", r.final_pop),
        format!("First repeat: {}", r.first_repeat_text()),
        format!("Period: {}", r.period_text()),
    ] {
        assert!(has_label(h, &label), "missing tile {label:?}");
    }
    assert!(has_label(h, &r.verdict), "missing verdict {:?}", r.verdict);
}

/// Replaces the editor's text by typing: focus, ⌘A, type.
fn retype(h: &mut App, text: &str) {
    reveal(h, "Rule script", Some(Role::MultilineTextInput));
    editor(h).focus();
    h.step();
    h.key_press_modifiers(Modifiers::COMMAND, Key::A);
    h.step();
    editor(h).type_text(text);
    settle(h);
}

/// Clicks somewhere neutral (the page ground under the run column) to leave the editor.
fn blur(h: &mut App) {
    editor(h).focus();
    h.key_press(Key::Escape);
    settle(h);
}

/// LAB-2/3, REF-2, DONE-2: the default lab is the mockup's, with every number from the engine.
#[test]
fn opens_on_the_mockups_numbers() {
    let h = lab();
    let want = engine(&RunSpec::default());
    assert_eq!(want.peak_text(), "221 @ 148");
    assert_eq!(want.final_pop, 46);
    assert_eq!(want.first_repeat_text(), "gen 390");
    assert_eq!(want.period_text(), "2");
    assert_eq!(want.verdict, "Settles into a period-2 cycle at gen 388.");
    assert_analysis(&h, &want);
    for label in [
        "B3/S23",
        "Moore r1 · wraps at edges",
        "gen 120 / 400 · 148 alive",
        "8 cells",
        "Boundary · 64 × 40",
        "Population across 400 generations",
        "Top-down view of the grid, 64 by 40, generation 120, 148 alive",
    ] {
        assert!(has_label(&h, label), "missing {label:?}");
    }
    get(
        &h,
        "Top-down view of the grid, 64 by 40, generation 120, 148 alive",
        Role::Image,
    );
    get(&h, "Population across 400 generations", Role::Image);
    // the script is the mockup's netlist, in an editable text input
    assert_eq!(script(&h), RuleScript::default().to_text());
    assert_eq!(editor(&h).accesskit_node().role(), Role::MultilineTextInput);
    // the RLE of generation 120, cut like the mockup
    let rle = analysis::run(&RunSpec::default()).rle(120).unwrap();
    assert!(rle.starts_with("x = 64, y = 34, rule = B3/S23"));
    let node = h.get_by_label_contains("RLE of generation 120: x = 64, y = 34, rule = B3/S23");
    let shown = node
        .accesskit_node()
        .label()
        .or_else(|| node.value())
        .unwrap();
    // the engine's RLE (with `n$` runs) fits in the mockup's 420-character preview
    let body: String = rle.lines().skip(1).collect();
    assert_eq!(
        shown,
        format!(
            "RLE of generation 120: {}\n{body}",
            rle.lines().next().unwrap()
        )
    );
    assert_eq!(
        h.state().screens().rule_lab.rle(),
        rle,
        "the API's RLE is full"
    );
    // the spec controls show the spec
    assert_eq!(pressed(&h, "Life-like"), Some(true));
    assert_eq!(pressed(&h, "Moore r1"), Some(true));
    assert_eq!(pressed(&h, "Born with 3"), Some(true));
    assert_eq!(pressed(&h, "Born with 2"), Some(false));
    assert_eq!(pressed(&h, "Survives with 2"), Some(true));
    assert_eq!(pressed(&h, "Survives with 3"), Some(true));
    assert_eq!(pressed(&h, "Torus"), Some(true));
    assert_eq!(pressed(&h, "R-pentomino"), Some(true));
    assert!(has_label(&h, "Born with 8") && !has_label(&h, "Born with 9"));
    get(&h, "This cell", Role::Button);
    assert!(disabled(&h, "This cell"));
}

/// DONE-3: the other families are shown, disabled, and say why.
#[test]
fn other_families_are_disabled_and_explained() {
    let h = lab();
    assert!(!disabled(&h, "Life-like"));
    for f in ["Generations", "Larger than Life", "Lenia"] {
        assert!(disabled(&h, f), "{f} should be disabled");
        let desc = h.get_by_label(f).accesskit_node().description();
        assert_eq!(desc.as_deref(), Some("Not in this version"), "{f}");
    }
    let group = child_labels(&h, "Rule family", Role::Group, Role::Button);
    assert_eq!(
        group,
        ["Life-like", "Generations", "Larger than Life", "Lenia"]
    );
}

/// LAB-1: toggling neighbourhood cells updates "N cells", the bit rows (bits above N go),
/// the preset highlight and the script's `.neigh` line; presets do the same.
#[test]
fn neighbourhood_editor_and_presets() {
    let mut h = lab();
    click_button(&mut h, "Neighbour -1, -1");
    assert!(has_label(&h, "7 cells"));
    assert_eq!(pressed(&h, "Neighbour -1, -1"), Some(false));
    assert!(!has_label(&h, "Born with 8"), "bit 8 is gone with N = 7");
    assert!(has_label(&h, "Born with 7"));
    for p in ["Moore r1", "von Neumann", "Hexagonal", "Moore r2"] {
        assert_eq!(pressed(&h, p), Some(false), "no preset matches 7 cells");
    }
    assert!(
        script(&h).contains(".neigh  custom  n=7  (0,-1) (1,-1)"),
        "{}",
        script(&h)
    );
    assert!(has_label(&h, "Custom (7 cells) · wraps at edges"));
    // the analysis is the engine's for that neighbourhood
    assert_analysis(&h, &engine(&spec(&h)));

    // back on: Moore r1 lights up again
    click_button(&mut h, "Neighbour -1, -1");
    assert!(has_label(&h, "8 cells"));
    assert_eq!(pressed(&h, "Moore r1"), Some(true));
    assert!(script(&h).contains(".neigh  moore  n=8"));

    // Survives with 8, then von Neumann (N = 4) drops it
    click_button(&mut h, "Survives with 8");
    assert_eq!(pressed(&h, "Survives with 8"), Some(true));
    assert!(script(&h).contains("B3/S238"));
    click_button(&mut h, "von Neumann");
    assert!(has_label(&h, "4 cells"));
    assert_eq!(pressed(&h, "von Neumann"), Some(true));
    assert_eq!(pressed(&h, "Moore r1"), Some(false));
    assert!(!has_label(&h, "Survives with 5") && !has_label(&h, "Survives with 8"));
    assert!(has_label(&h, "Survives with 4"));
    assert_eq!(spec(&h).rule.code(), "B3/S23", "8 dropped with N = 4");
    assert!(script(&h).contains(".rule   life_like  B3/S23"));
    assert!(script(&h).contains(".neigh  vn  n=4"));

    for (preset, n, key) in [
        ("Hexagonal", 6, "hex"),
        ("Moore r2", 24, "moore2"),
        ("Moore r1", 8, "moore"),
    ] {
        click_button(&mut h, preset);
        assert!(has_label(&h, &format!("{n} cells")), "{preset}");
        assert_eq!(pressed(&h, preset), Some(true));
        assert!(
            script(&h).contains(&format!(".neigh  {key}  n={n}")),
            "{preset}"
        );
        assert!(has_label(&h, &format!("Born with {n}")));
        assert!(!has_label(&h, &format!("Born with {}", n + 1)));
    }
    // a cell outside r1 turns Moore r1 into a custom 9
    click_button(&mut h, "Neighbour -2, -2");
    assert!(has_label(&h, "9 cells") && has_label(&h, "Born with 9"));
}

/// LAB-1/3: bits, boundary and seed re-run the analysis and rewrite their script lines.
#[test]
fn bits_boundary_and_seed_rerun() {
    let mut h = lab();
    click_button(&mut h, "Born with 6");
    assert_eq!(pressed(&h, "Born with 6"), Some(true));
    assert!(has_label(&h, "B36/S23"));
    assert!(script(&h).contains(".rule   life_like  B36/S23"));
    let highlife = engine(&spec(&h));
    assert_eq!(spec(&h).rule, LifeRule::parse("B36/S23").unwrap());
    assert_analysis(&h, &highlife);
    click_button(&mut h, "Born with 6");
    assert!(has_label(&h, "B3/S23"));

    click_button(&mut h, "Dead edge");
    assert_eq!(pressed(&h, "Dead edge"), Some(true));
    assert_eq!(pressed(&h, "Torus"), Some(false));
    assert_eq!(spec(&h).boundary, Boundary::Dead);
    assert!(has_label(&h, "Moore r1 · dead edges"));
    assert!(script(&h).contains(".grid   64x40  boundary=dead"));
    let dead = engine(&spec(&h));
    assert_ne!(
        dead,
        engine(&RunSpec::default()),
        "dead edges change the run"
    );
    assert_analysis(&h, &dead);

    click_button(&mut h, "Acorn");
    assert_eq!(pressed(&h, "Acorn"), Some(true));
    assert_eq!(spec(&h).seed, Seed::Acorn);
    assert!(script(&h).contains(".seed   acorn @ (32,20)"));
    assert_analysis(&h, &engine(&spec(&h)));

    click_button(&mut h, "Soup");
    assert_eq!(spec(&h).seed, Seed::MOCKUP_SOUP);
    assert!(script(&h).contains(".seed   soup seed=7 density=0.33"));
    assert_analysis(&h, &engine(&spec(&h)));

    click_button(&mut h, "Torus");
    click_button(&mut h, "R-pentomino");
    assert_eq!(spec(&h), RunSpec::default());
    assert_eq!(
        script(&h),
        RuleScript::default().to_text(),
        "back to the mockup's text"
    );
}

/// LAB-2/5: the scrub slider moves the generation, the alive count, the well and the RLE.
#[test]
fn scrub_moves_everything() {
    let mut h = lab();
    let run = analysis::run(&RunSpec::default());
    get(&h, "Scrub", Role::Slider).set_value_accesskit(200.0);
    settle(&mut h);
    let s = &h.state().screens().rule_lab;
    assert_eq!(s.generation(), 200);
    assert_eq!(s.alive(), run.population(200).unwrap());
    let alive = run.population(200).unwrap();
    assert!(has_label(&h, &format!("gen 200 / 400 · {alive} alive")));
    assert!(has_label(
        &h,
        &format!("Top-down view of the grid, 64 by 40, generation 200, {alive} alive")
    ));
    assert!(has_label(&h, "Pattern export · RLE: gen 200"));
    let rle = run.rle(200).unwrap();
    let header = rle.lines().next().unwrap();
    assert!(
        h.query_by_label_contains(&format!("RLE of generation 200: {header}"))
            .is_some()
    );
    assert_eq!(h.state().screens().rule_lab.rle(), rle);

    // arrow keys step it; clicking the track jumps (and pauses Play)
    click_button(&mut h, "Play");
    assert!(h.state().screens().rule_lab.playing);
    let slider = get(&h, "Scrub", Role::Slider).rect();
    get(&h, "Scrub", Role::Slider).click();
    settle(&mut h);
    let s = &h.state().screens().rule_lab;
    assert!(!s.playing, "scrubbing pauses");
    assert!(
        (195..=205).contains(&s.generation()),
        "middle of {slider:?}: {}",
        s.generation()
    );

    // the control API
    h.state_mut().screens_mut().rule_lab.scrub(5);
    settle(&mut h);
    assert!(has_label(
        &h,
        &format!("gen 005 / 400 · {} alive", run.population(5).unwrap())
    ));
}

/// LAB-2: Play advances 0 → 400 at 60 ms a generation and loops; Pause stops it.
#[test]
fn play_advances_and_loops() {
    let mut h = lab();
    click_button(&mut h, "Play");
    assert!(has_label(&h, "Pause"));
    assert!(step_until(&mut h, 120, |h| h
        .state()
        .screens()
        .rule_lab
        .generation()
        >= 123));
    h.state_mut().screens_mut().rule_lab.scrub(398);
    settle(&mut h);
    click_button(&mut h, "Play");
    assert!(step_until(&mut h, 120, |h| h
        .state()
        .screens()
        .rule_lab
        .generation()
        < 5));
    click_button(&mut h, "Pause");
    let g = h.state().screens().rule_lab.generation();
    h.run_steps(30);
    assert_eq!(h.state().screens().rule_lab.generation(), g);
    assert!(has_label(&h, "Play"));
}

/// LAB-4: typing into the script updates the controls and the analysis.
#[test]
fn typing_valid_edits_updates_controls_and_analysis() {
    let mut h = lab();
    // a real keystroke: line 2, end, four left, "6" → B36/S23
    reveal(&mut h, "Rule script", Some(Role::MultilineTextInput));
    editor(&h).focus();
    h.step();
    h.key_press_modifiers(Modifiers::COMMAND, Key::ArrowUp);
    h.key_press(Key::ArrowDown);
    h.key_press(Key::End);
    for _ in 0..4 {
        h.key_press(Key::ArrowLeft);
    }
    h.step();
    editor(&h).type_text("6");
    settle(&mut h);
    assert!(
        script(&h).contains(".rule   life_like  B36/S23"),
        "{}",
        script(&h)
    );
    assert_eq!(pressed(&h, "Born with 6"), Some(true));
    assert!(has_label(&h, "B36/S23"));
    assert_analysis(&h, &engine(&spec(&h)));
    assert_eq!(h.state().screens().rule_lab.errors(), []);

    // boundary=dead and a soup with its own seed and density
    let text = RuleScript::default()
        .to_text()
        .replace("boundary=torus", "boundary=dead")
        .replace("r_pentomino @ (32,20)", "soup seed=9 density=0.4");
    retype(&mut h, &text);
    assert_eq!(script(&h), text, "the text stays as typed");
    let s = spec(&h);
    assert_eq!(s.boundary, Boundary::Dead);
    assert_eq!(
        s.seed,
        Seed::Soup {
            seed: 9,
            density: 0.4
        }
    );
    assert_eq!(pressed(&h, "Dead edge"), Some(true));
    assert_eq!(pressed(&h, "Soup"), Some(true));
    assert_eq!(pressed(&h, "Born with 6"), Some(false));
    assert_analysis(&h, &engine(&s));

    // a bigger grid and fewer generations reach the scrubber and the labels
    let text = text
        .replace("64x40", "48x30")
        .replace("gens=400", "gens=200");
    retype(&mut h, &text);
    assert!(has_label(&h, "Boundary · 48 × 30"));
    assert!(has_label(&h, "Population across 200 generations"));
    assert!(has_label_containing(&h, "/ 200 ·"));
    blur(&mut h);
}

/// LAB-4: an invalid edit shows a line-numbered error, keeps the last valid analysis, and
/// recovers when fixed; the controls never clobber the line being fixed.
#[test]
fn invalid_edits_show_errors_and_recover() {
    let mut h = lab();
    let good = RuleScript::default().to_text();
    let before = engine(&RunSpec::default());
    retype(&mut h, &good.replace("B3/S23", "B3/Q23"));
    let errors = h.state().screens().rule_lab.errors().to_vec();
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert_eq!(errors[0].line, 2);
    let msg = format!("Line 2: {}", errors[0].message);
    assert!(has_label(&h, &msg), "error shown: {msg}");
    assert!(has_label(&h, "The last valid script is still in effect."));
    let desc = editor(&h).accesskit_node().description().unwrap();
    assert!(desc.starts_with("1 error. Line 2:"), "{desc}");
    assert_analysis(&h, &before);
    assert!(has_label(&h, "B3/S23"), "last valid rule still shown");

    // a control changes another line while the error stays; the text keeps the broken line
    blur(&mut h);
    click_button(&mut h, "Dead edge");
    assert!(
        script(&h).contains("B3/Q23"),
        "the broken line stays: {}",
        script(&h)
    );
    assert!(script(&h).contains("boundary=dead"));
    assert_analysis(&h, &engine(&spec(&h)));

    // fixing it applies the text, which now includes the control's change
    let fixed = script(&h).replace("B3/Q23", "B3/S2");
    retype(&mut h, &fixed);
    assert_eq!(h.state().screens().rule_lab.errors(), []);
    assert!(!has_label(&h, &msg));
    assert!(editor(&h).accesskit_node().description().is_none());
    let s = spec(&h);
    assert_eq!(
        (s.rule.code().as_str(), s.boundary),
        ("B3/S2", Boundary::Dead)
    );
    assert_analysis(&h, &engine(&s));

    // several errors at once, each with its line, never a panic
    retype(
        &mut h,
        ".rule life_like B3/S23\n.neigh hex n=9\n.grid 0x0\n.end",
    );
    let errors = h.state().screens().rule_lab.errors().to_vec();
    assert!(errors.len() >= 2, "{errors:?}");
    for e in &errors {
        assert!(has_label(&h, &format!("Line {}: {}", e.line, e.message)));
    }
    retype(&mut h, "");
    assert!(!h.state().screens().rule_lab.errors().is_empty());
    assert_eq!(spec(&h), s, "still the last valid spec");
}

/// The control API (for the MCP bridge).
#[test]
fn control_api() {
    let mut h = lab();
    let lab = &mut h.state_mut().screens_mut().rule_lab;
    assert_eq!(lab.script_text(), RuleScript::default().to_text());
    assert_eq!(lab.generation(), 120);
    lab.set_rule(LifeRule::parse("B36/S23").unwrap());
    assert_eq!(lab.spec().rule.code(), "B36/S23");
    assert!(lab.script_text().contains("B36/S23"));
    let highlife = RunSpec {
        rule: LifeRule::parse("B36/S23").unwrap(),
        ..RunSpec::default()
    };
    assert_eq!(lab.report(), engine(&highlife));
    let errs = lab.set_script("nonsense").unwrap_err();
    assert!(!errs.is_empty());
    assert!(lab.script_text().contains("B36/S23"), "unchanged on error");
    lab.set_script(&RuleScript::default().to_text()).unwrap();
    assert_eq!(lab.spec(), RunSpec::default());
    lab.scrub(9999);
    assert_eq!(lab.generation(), 400);
    assert_eq!(
        lab.rle(),
        analysis::run(&RunSpec::default()).rle(400).unwrap()
    );
    settle(&mut h);
    assert!(has_label(&h, "gen 400 / 400 · 46 alive"));
}

/// LAB-5: Export's menu copies the RLE / script and saves .rle / .rule files.
#[test]
fn export_menu() {
    let mut h = lab();
    let run = analysis::run(&RunSpec::default());
    click_button(&mut h, "Export");
    get(&h, "Export", Role::Menu);
    let items = child_labels(&h, "Export", Role::Menu, Role::MenuItem);
    assert_eq!(
        items,
        ["Copy RLE", "Copy script", "Save .rle…", "Save .rule…"]
    );
    assert_eq!(
        get(&h, "Export", Role::Button)
            .accesskit_node()
            .data()
            .is_expanded(),
        Some(true)
    );

    click_role(&mut h, "Copy RLE", Role::MenuItem);
    assert_eq!(h.state().last_copied(), run.rle(120).as_deref());
    assert!(toasts(&h).contains(&"Copied generation 120 as RLE".to_string()));
    assert!(h.query_by_label("Copy RLE").is_none(), "the menu closes");

    click_button(&mut h, "Export");
    click_role(&mut h, "Copy script", Role::MenuItem);
    assert_eq!(
        h.state().last_copied(),
        Some(RuleScript::default().to_text().as_str())
    );

    click_button(&mut h, "Export");
    click_role(&mut h, "Save .rle…", Role::MenuItem);
    click_button(&mut h, "Export");
    click_role(&mut h, "Save .rule…", Role::MenuItem);
    let saved = h.state().saved_files().to_vec();
    assert_eq!(saved.len(), 2);
    let out = h.state().services().out_dir.clone();
    assert_eq!(saved[0], out.join("glider-garden.rle"));
    assert_eq!(saved[1], out.join("glider-garden.rule"));
    assert_eq!(
        std::fs::read_to_string(&saved[0]).unwrap(),
        run.rle(120).unwrap() + "\n"
    );
    let rule_file = std::fs::read_to_string(&saved[1]).unwrap();
    assert_eq!(rule_file, RuleScript::default().to_text() + "\n");
    assert_eq!(
        RuleScript::parse(&rule_file).unwrap(),
        RuleScript::default()
    );
    assert!(toasts(&h).contains(&"Saved glider-garden.rle".to_string()));
    let _ = std::fs::remove_dir_all(&out);

    // Escape and a click outside close it
    click_button(&mut h, "Export");
    assert!(has_label(&h, "Copy RLE"));
    press(&mut h, Key::Escape);
    settle(&mut h);
    assert!(!has_label(&h, "Copy RLE"));
    click_button(&mut h, "Export");
    h.get_by_label("Moore r1 · wraps at edges").click();
    settle(&mut h);
    assert!(!has_label(&h, "Copy RLE"));
}

/// NAV-4: Open in Playground goes to the Tray with the rule and the scrubbed cells.
#[test]
fn open_in_playground_carries_rule_and_cells() {
    let mut h = lab();
    click_button(&mut h, "Born with 6");
    get(&h, "Scrub", Role::Slider).set_value_accesskit(150.0);
    settle(&mut h);
    let rle = h.state().screens().rule_lab.rle();
    click_button(&mut h, "Open in Playground");
    assert_eq!(route(&h), Route::Tray);
    let got = h.state().screens().tray.received.clone().expect("carried");
    assert_eq!(got.rule, "B36/S23");
    assert_eq!(got.from, Route::RuleLab);
    assert_eq!(got.rle.as_deref(), Some(rle.as_str()));
    assert!(rle.contains("rule = B36/S23"));
}

/// NAV-4: "Analyse in Rule lab" from the Terrarium sets the rule (Moore) and re-runs.
#[test]
fn receives_a_rule_from_the_terrarium() {
    let mut h = lab();
    click_button(&mut h, "von Neumann");
    h.state_mut().navigate_with(
        Route::RuleLab,
        Carry::rule("B3678/S34678", Route::Terrarium),
    );
    settle(&mut h);
    let s = spec(&h);
    assert_eq!(s.rule, LifeRule::parse("B3678/S34678").unwrap());
    assert_eq!(s.rule.neighbourhood().len(), 8, "Moore");
    assert!(has_label(&h, "B3678/S34678"));
    assert!(script(&h).contains(".rule   life_like  B3678/S34678"));
    assert_analysis(&h, &engine(&s));
    assert!(toasts(&h).iter().any(|t| t.contains("B3678/S34678")));
}

/// DONE-1: renders at the 1440 × 960 board and at 390 wide (stacked, scrolling) without
/// panics; the well and plot stay inside their column.
#[test]
fn renders_at_1440_and_390() {
    let mut h = lab();
    let img = render(&mut h);
    assert_eq!((img.width(), img.height()), (1440, 960));
    let well = get(
        &h,
        "Top-down view of the grid, 64 by 40, generation 120, 148 alive",
        Role::Image,
    )
    .rect();
    assert!(well.left() >= 316.0 && well.right() <= 1084.0, "{well:?}");

    let mut p = app().at(Route::RuleLab).phone().build();
    let _ = render(&mut p);
    for (label, role) in [
        ("Rule script", Role::MultilineTextInput),
        ("Scrub", Role::Slider),
    ] {
        reveal(&mut p, label, Some(role));
        let r = get(&p, label, role).rect();
        assert!(
            r.left() >= -0.5 && r.right() <= PHONE_W + 0.5,
            "{label} {r:?}"
        );
    }
    let well = p.get_by_label_contains("Top-down view of the grid").rect();
    assert!(well.width() <= PHONE_W - 31.0, "{well:?}");
    let full = render_full(Route::RuleLab, PHONE_W, 2899.0);
    assert!(full.height() >= 2899);
}
