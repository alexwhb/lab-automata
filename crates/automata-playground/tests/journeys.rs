//! End-to-end journeys through the real app, as a user would take them: every action goes
//! through AccessKit (buttons, links, tabs, sliders, text inputs, focus) or real input events
//! (pointer presses on the drawn cells, key presses, a paste), and every check reads what the
//! GUI shows (labels, values, the clipboard) against an independent run of the engine. The
//! screens' state is only read to compare cell sets the GUI can't spell out.
//!
//! - (a) [`first_launch_runs_draws_stamps_and_round_trips_a_pattern`] — autoplay, speed,
//!   readouts vs the engine (REF-2's gen 30), drawing, a wrapping stamp, Share → ⌘V → place.
//! - (b) [`terrarium_to_rule_lab_to_tray`] — keep, breed three broods, lineage, Analyse in
//!   Rule lab, edit the script, scrub, Open in Playground, the cells cropped as documented.
//! - (c) [`loom_builds_rule_90_and_weaves_sierpinski`] — the rule keys, weaving, Gould's
//!   sequence in the shared text.
//! - (d) [`series_open_and_back`] — NAV-3's Open buttons with a recording launcher (the header
//!   has no app switcher for now, so the journey starts on the Series screen).
//! - (e) [`keyboard_only`] — NAV-5: ⌘1–⌘4, Tab / Enter, Space, `S`, arrow-key drawing.
//! - (f) the main journeys at 390 px: [`first_launch_journey_on_a_phone`],
//!   [`terrarium_journey_on_a_phone`], [`phone_focus_scrolls_the_page`].
//! - (g) [`every_screen_renders_names_and_describes_itself`] — a screenshot per screen at
//!   1440 and 390, every interactive node named (A11Y-1), canvas descriptions that update
//!   (A11Y-4).

mod common;

use ap_engine::analysis;
use ap_engine::mood::History;
use ap_engine::pattern::{self, library};
use ap_engine::presets::{TRAY_H, TRAY_W, starting_garden};
use ap_engine::{Anchor, Boundary, Grid, LifeRule, Pattern, RuleScript, Seed};
use automata_playground::screens::loom::key_name;
use automata_playground::{AppId, Launcher, Route};
use common::*;
use eframe::egui::{Event, Key, Modifiers, Pos2, accesskit::Role};
use egui_kittest::kittest::{NodeT, Queryable};

// ---- reading the GUI ------------------------------------------------------------------------

/// Every node's text (label or value) with its role.
fn texts(h: &App) -> Vec<(Role, String)> {
    h.root()
        .children_recursive()
        .map(|n| (n.accesskit_node().role(), text_of(&n)))
        .collect()
}

/// The first text that `pick` accepts.
fn find_text(h: &App, pick: impl Fn(&str) -> bool) -> Option<String> {
    texts(h).into_iter().map(|(_, t)| t).find(|t| pick(t))
}

/// A `word 0042` counter in the header (`gen`, `row`, `brood` — the last unpadded).
fn counter(h: &App, word: &str) -> u64 {
    let prefix = format!("{word} ");
    find_text(h, |t| {
        t.strip_prefix(&prefix)
            .is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()))
    })
    .and_then(|t| t[prefix.len()..].parse().ok())
    .unwrap_or_else(|| panic!("no `{word} N` counter"))
}

/// A `"<key>: <number>"` label (the stat tiles).
fn tile(h: &App, key: &str) -> u64 {
    let prefix = format!("{key}: ");
    find_text(h, |t| t.starts_with(&prefix))
        .and_then(|t| t[prefix.len()..].parse().ok())
        .unwrap_or_else(|| panic!("no tile {key:?}"))
}

/// The label of the only node with `role` whose label starts with `prefix`.
fn role_label(h: &App, role: Role, prefix: &str) -> String {
    let found: Vec<String> = texts(h)
        .into_iter()
        .filter(|(r, t)| *r == role && t.starts_with(prefix))
        .map(|(_, t)| t)
        .collect();
    assert_eq!(
        found.len(),
        1,
        "one {role:?} starting {prefix:?}: {found:?}"
    );
    found[0].clone()
}

/// The tray canvas's accessible description (A11Y-4).
fn tray_canvas(h: &App) -> String {
    role_label(h, Role::Grid, "Cell tray")
}

/// What the app put on the clipboard last.
fn clipboard(h: &App) -> String {
    h.state()
        .last_copied()
        .expect("something was copied")
        .to_string()
}

fn alive(h: &App) -> Vec<(usize, usize)> {
    h.state().screens().tray.grid().alive_cells().collect()
}

/// Where tray cell `(x, y)` is drawn (through the perspective, last frame).
fn cell_at(h: &App, x: usize, y: usize) -> Pos2 {
    h.state()
        .screens()
        .tray
        .cell_center(x, y)
        .unwrap_or_else(|| panic!("cell ({x}, {y}) not drawn"))
}

/// A pointer press and release on tray cell `(x, y)`, scrolled into view first.
fn press_cell(h: &mut App, x: usize, y: usize) {
    let label = tray_canvas(h);
    reveal(h, &label, Some(Role::Grid));
    let p = cell_at(h, x, y);
    h.hover_at(p);
    h.drag_at(p);
    h.drop_at(p);
    settle(h);
}

/// The cells of `p` placed with `anchor` at `(x, y)` on an empty tray (torus).
fn placed(p: &Pattern, x: i64, y: i64, anchor: Anchor) -> Vec<(usize, usize)> {
    let mut g = Grid::new(TRAY_W, TRAY_H);
    g.place(p, x, y, anchor, Boundary::Torus);
    g.alive_cells().collect()
}

fn union(a: &[(usize, usize)], b: &[(usize, usize)]) -> Vec<(usize, usize)> {
    let mut v: Vec<_> = a.iter().chain(b).copied().collect();
    v.sort_by_key(|&(x, y)| (y, x));
    v.dedup();
    v
}

fn sorted(mut v: Vec<(usize, usize)>) -> Vec<(usize, usize)> {
    v.sort_by_key(|&(x, y)| (y, x));
    v
}

// ---- (a) first launch on the Tray -------------------------------------------------------------

/// First launch with autoplay: the Tray runs at its speed, and every readout (counter, chip,
/// canvas description, tiles, sparkline, mood) follows an independent engine run of the
/// starting garden — at gen 30 the mockup's 90 alive / 27 born / 57 died (REF-2). Then draw
/// a shape (keyboard and pointer), stamp a glider across the corner, Share, Clear, paste the
/// RLE back (⌘V) and place it: the same cells.
fn first_launch_journey(w: f32, hgt: f32) {
    let mut h = app().autoplay().size(w, hgt).build();
    assert_eq!(route(&h), Route::Tray, "first launch opens the Tray");
    get(&h, "Pause", Role::Button); // running
    assert!(has_label(&h, "Speed: 10 gen/s"));

    // 10 gen/s: one generation per 0.1 s frame, up to gen 30
    let g0 = counter(&h, "gen");
    assert!(g0 < 30, "{g0}");
    for g in g0..30 {
        h.step();
        assert_eq!(counter(&h, "gen"), g + 1, "one generation per frame");
    }
    // while running, every readout shows the same generation as the header (REF-2: gen 30)
    check_tray_against_engine(&h, 30);
    assert_eq!(tile(&h, "Alive"), 90);
    assert_eq!(tile(&h, "Born"), 27);
    assert_eq!(tile(&h, "Died"), 57);
    click_button(&mut h, "Pause");
    get(&h, "Run", Role::Button);
    let paused_at = counter(&h, "gen");
    h.run_steps(5);
    assert_eq!(counter(&h, "gen"), paused_at, "paused");
    check_tray_against_engine(&h, paused_at);

    // 20 gen/s for one simulated second: +20
    get(&h, "Speed", Role::Slider).set_value_accesskit(20.0);
    settle(&mut h);
    assert!(has_label(&h, "Speed: 20 gen/s"));
    click_button(&mut h, "Run");
    let g = counter(&h, "gen");
    h.run_steps(10);
    assert_eq!(counter(&h, "gen"), g + 20, "20 gen/s for 1 s");
    click_button(&mut h, "Pause");
    let g1 = counter(&h, "gen");
    check_tray_against_engine(&h, g1);

    // a clean tray, then draw: a blinker with the keyboard, a block with the pointer
    click_button(&mut h, "Clear");
    assert!(has_label(&h, "gen 0000 · 0 alive"));
    assert!(alive(&h).is_empty());
    let grid = tray_canvas(&h);
    reveal(&mut h, &grid, Some(Role::Grid));
    get(&h, &grid, Role::Grid).focus();
    settle(&mut h);
    press(&mut h, Key::Space); // the cursor starts at the centre (20, 13)
    press(&mut h, Key::ArrowRight);
    press(&mut h, Key::Space);
    press(&mut h, Key::ArrowRight);
    press(&mut h, Key::Space);
    settle(&mut h);
    let blinker = vec![(20, 13), (21, 13), (22, 13)];
    assert_eq!(alive(&h), blinker);
    for (x, y) in [(5, 5), (6, 5), (5, 6), (6, 6)] {
        press_cell(&mut h, x, y);
    }
    let drawn = union(&blinker, &[(5, 5), (6, 5), (5, 6), (6, 6)]);
    assert_eq!(alive(&h), drawn);
    assert!(tray_canvas(&h).ends_with("generation 0, 7 alive"));

    // stamp a glider centred on the corner: it wraps onto all four edges
    click_button(&mut h, "Glider");
    assert_eq!(pressed(&h, "Glider"), Some(true));
    assert!(has_label(&h, "Click to place Glider"));
    press_cell(&mut h, 0, 0);
    let glider = library::pattern("glider").expect("glider");
    let with_glider = union(&drawn, &placed(&glider, 0, 0, Anchor::Center));
    assert_eq!(alive(&h), with_glider);
    assert!(
        with_glider.iter().any(|&(x, _)| x == TRAY_W - 1)
            && with_glider.iter().any(|&(_, y)| y == TRAY_H - 1),
        "the glider wrapped: {with_glider:?}"
    );

    // Share → the clipboard holds the whole tray as RLE
    click_button(&mut h, "Share");
    assert!(toasts(&h).iter().any(|t| t == "Copied the tray as RLE"));
    let rle = clipboard(&h);
    let file = pattern::parse_any(&rle).expect("Share copies valid RLE");
    assert_eq!(file.rule.as_deref(), Some("B3/S23"));
    assert_eq!(
        placed(&file.pattern, 0, 0, Anchor::TopLeft),
        with_glider,
        "the RLE is the tray"
    );

    // Clear, paste it back, place it where it was
    click_button(&mut h, "Clear");
    assert!(alive(&h).is_empty());
    h.key_press_modifiers(Modifiers::COMMAND, Key::V);
    h.event(Event::Paste(rle.clone()));
    settle(&mut h);
    assert_eq!(
        pressed(&h, "Pasted pattern"),
        Some(true),
        "armed as a stamp"
    );
    assert!(has_label(&h, "Click to place Pasted pattern"));
    // the stamp is the pattern's live box, placed centred on the pressed cell
    let bbox = file.pattern.trimmed();
    let (bx, by) = {
        let min_x = file.pattern.cells().iter().map(|c| c.0).min().unwrap();
        let min_y = file.pattern.cells().iter().map(|c| c.1).min().unwrap();
        (min_x, min_y)
    };
    let x = (bx + bbox.width() / 2) % TRAY_W;
    let y = (by + bbox.height() / 2) % TRAY_H;
    press_cell(&mut h, x, y);
    assert_eq!(
        alive(&h),
        with_glider,
        "the same cells after the round trip"
    );
    assert!(tray_canvas(&h).ends_with(&format!("{} alive", with_glider.len())));
}

/// The Tray at generation `gen` matches the starting garden run `gen` times under Life: the
/// cells, the counter, the chip, the canvas description, the tiles, the sparkline's summary
/// and the mood.
fn check_tray_against_engine(h: &App, generation: u64) {
    let mut g = starting_garden();
    let mut history = History::new(g.population());
    let mut last = None;
    for _ in 0..generation {
        let s = g.step(&LifeRule::LIFE, Boundary::Torus);
        history.push(s.population);
        last = Some(s);
    }
    let s = last.expect("at least one step");
    let pop = g.population();
    assert_eq!(alive(h), g.alive_cells().collect::<Vec<_>>(), "cells");
    assert_eq!(counter(h, "gen"), generation);
    assert!(
        has_label(h, &format!("gen {generation:04} · {pop} alive")),
        "chip"
    );
    assert_eq!(
        tray_canvas(h),
        format!("Cell tray, 40 by 26, generation {generation}, {pop} alive")
    );
    assert_eq!(tile(h, "Generation"), generation);
    assert_eq!(tile(h, "Alive"), pop as u64);
    assert_eq!(tile(h, "Born"), s.born as u64);
    assert_eq!(tile(h, "Died"), s.died as u64);
    let spark = get(h, "Population over the last 120 generations", Role::Image)
        .accesskit_node()
        .value()
        .expect("the sparkline's summary");
    let peak = history.values().iter().max().copied().unwrap_or(0);
    assert_eq!(
        spark,
        format!(
            "{} generations, latest {pop}, peak {peak}",
            history.values().len()
        )
    );
    let mood = history.mood();
    let callout = child_labels(h, "What is happening", Role::Group, Role::Label);
    assert_eq!(callout, [mood.title(), mood.text()], "the mood callout");
}

#[test]
fn first_launch_runs_draws_stamps_and_round_trips_a_pattern() {
    first_launch_journey(W, H);
}

/// (f) The same journey on a phone: the columns stack and the page scrolls to every control.
#[test]
fn first_launch_journey_on_a_phone() {
    first_launch_journey(PHONE_W, PHONE_H);
}

// ---- (b) Terrarium → Rule lab → Tray ----------------------------------------------------------

/// The code and name shown on jar card `n` (1-based).
fn jar_card(h: &App, n: usize) -> (String, String) {
    let labels = child_labels(h, &format!("Jar {n}"), Role::Group, Role::Label);
    let code = labels
        .iter()
        .find(|l| LifeRule::parse(l).is_ok())
        .unwrap_or_else(|| panic!("jar {n} shows no rule code: {labels:?}"))
        .clone();
    let name = labels
        .iter()
        .skip_while(|l| **l != code)
        .nth(1)
        .cloned()
        .unwrap_or_default();
    (code, name)
}

fn jar_codes(h: &App) -> Vec<String> {
    (1..=6).map(|n| jar_card(h, n).0).collect()
}

/// The lineage row of brood `n`: its parents' codes.
fn lineage(h: &App, n: u64) -> Vec<String> {
    let labels = child_labels(h, &format!("brood {n} parents"), Role::Group, Role::Label);
    assert_eq!(labels[0], format!("brood {n}"));
    labels[1..].to_vec()
}

/// A child's bits come from its parents, then `mutation` flips: where every parent agrees,
/// the child differs in at most `mutation` places (plus B3 for a child left without births).
fn plausible_child(child: &str, parents: &[String], mutation: usize) -> bool {
    let bits = |code: &str| {
        let r = LifeRule::parse(code).expect("a rule");
        let mut v = Vec::with_capacity(18);
        for n in 0..=8 {
            v.push(r.born_with(n));
            v.push(r.survives_with(n));
        }
        v
    };
    let c = bits(child);
    let ps: Vec<Vec<bool>> = parents.iter().map(|p| bits(p)).collect();
    let flips = (0..c.len())
        .filter(|&i| ps.iter().all(|p| p[i] == ps[0][i]) && c[i] != ps[0][i])
        .count();
    flips <= mutation + 1
}

/// Replaces the script editor's text by typing (focus, ⌘A, type) and leaves the editor.
fn retype_script(h: &mut App, text: &str) {
    reveal(h, "Rule script", Some(Role::MultilineTextInput));
    get(h, "Rule script", Role::MultilineTextInput).focus();
    h.step();
    h.key_press_modifiers(Modifiers::COMMAND, Key::A);
    h.step();
    get(h, "Rule script", Role::MultilineTextInput).type_text(text);
    settle(h);
    h.key_press(Key::Escape);
    settle(h);
}

/// Keep two jars, breed three broods (kept jars carry over; with none kept all six are
/// parents), read the lineage, "Analyse in Rule lab" (jar 1's rule when none is kept), switch
/// the script to `boundary=dead` and an acorn, scrub to the run's peak, "Open in Playground":
/// the Tray has that rule and that generation's cells, cropped to 40 × 26 around their
/// centre and centred (as `tray::State::load_pattern` documents).
fn terrarium_journey(w: f32, hgt: f32) {
    let mut h = app().at(Route::Terrarium).size(w, hgt).build();
    assert_eq!(counter(&h, "brood"), 1);
    assert!(has_label(&h, "Nothing bred yet."));
    let start = jar_codes(&h);
    assert_eq!(
        start,
        [
            "B3/S23",
            "B36/S23",
            "B3678/S34678",
            "B3/S12345",
            "B36/S125",
            "B368/S245"
        ],
        "TERR-1's starting rules"
    );

    // brood 2: two kept jars carry over, the other four are their children
    click_button(&mut h, "Keep jar 2");
    click_button(&mut h, "Keep jar 5");
    assert_eq!(pressed(&h, "Keep jar 2"), Some(true));
    assert_eq!(pressed(&h, "Keep jar 5"), Some(true));
    assert!(has_label(&h, "2 kept as parents"));
    let kept = vec![start[1].clone(), start[4].clone()];
    click_button(&mut h, "Breed brood 2");
    assert_eq!(counter(&h, "brood"), 2);
    let b2 = jar_codes(&h);
    assert_eq!(b2[..2], kept[..], "kept jars come first, unchanged");
    for child in &b2[2..] {
        assert!(plausible_child(child, &kept, 2), "{child} from {kept:?}");
    }
    assert_eq!(lineage(&h, 2 - 1), kept, "the lineage lists the parents");
    assert_eq!(pressed(&h, "Keep jar 1"), Some(false), "keeps are cleared");
    assert!(has_label(&h, "None kept: all six become parents"));

    // brood 3: none kept, all six are parents
    click_button(&mut h, "Breed brood 3");
    let b3 = jar_codes(&h);
    for child in &b3 {
        assert!(plausible_child(child, &b2, 2), "{child} from {b2:?}");
    }
    assert_eq!(lineage(&h, 2), b2);

    // brood 4: keep jar 1 again
    click_button(&mut h, "Keep jar 1");
    assert_eq!(pressed(&h, "Keep jar 1"), Some(true));
    assert!(has_label(&h, "1 kept as parents"));
    click_button(&mut h, "Breed brood 4");
    assert_eq!(counter(&h, "brood"), 4);
    let b4 = jar_codes(&h);
    assert_eq!(b4[0], b3[0]);
    assert_eq!(lineage(&h, 3), [b3[0].clone()]);
    // newest first: brood 3, 2, 1
    let order: Vec<String> = texts(&h)
        .into_iter()
        .filter(|(r, t)| *r == Role::Group && t.ends_with(" parents") && t.starts_with("brood "))
        .map(|(_, t)| t)
        .collect();
    assert_eq!(
        order,
        ["brood 3 parents", "brood 2 parents", "brood 1 parents"]
    );

    // Analyse in Rule lab: none kept → jar 1
    let chosen = b4[0].clone();
    click_link(&mut h, "Analyse in Rule lab");
    assert_eq!(route(&h), Route::RuleLab);
    assert!(has_label(&h, &chosen), "the Rule lab shows {chosen}");
    let script = get(&h, "Rule script", Role::MultilineTextInput)
        .accesskit_node()
        .value()
        .unwrap_or_default();
    let rule_line = format!(".rule   life_like  {chosen}");
    assert!(script.contains(&rule_line), "{script}");

    // edit the script: dead edges, an acorn
    let edited = script
        .replace("boundary=torus", "boundary=dead")
        .replace("r_pentomino", "acorn");
    retype_script(&mut h, &edited);
    let spec = RuleScript::parse(&edited).expect("a valid script").spec();
    assert_eq!(spec.boundary, Boundary::Dead);
    assert_eq!(spec.seed, Seed::Acorn);
    assert_eq!(spec.rule, LifeRule::parse(&chosen).unwrap());
    assert_eq!(pressed(&h, "Dead edge"), Some(true));
    assert_eq!(pressed(&h, "Acorn"), Some(true));
    assert!(has_label(&h, "Moore r1 · dead edges"));

    // scrub to the peak
    let run = analysis::run(&spec);
    let report = run.report();
    let g = report.peak_gen.clamp(1, spec.gens);
    get(&h, "Scrub", Role::Slider).set_value_accesskit(g as f64);
    settle(&mut h);
    let pop = run.population(g).unwrap();
    assert!(has_label(
        &h,
        &format!("gen {g:03} / {} · {pop} alive", spec.gens)
    ));
    assert!(has_label(
        &h,
        &format!("Peak population: {}", report.peak_text())
    ));
    assert!(has_label(&h, &report.verdict));

    // Open in Playground → the Tray
    click_button(&mut h, "Open in Playground");
    assert_eq!(route(&h), Route::Tray);
    assert!(has_label(&h, &chosen), "the Tray's rule panel");
    let frame = run.frame(g).expect("the frame").trimmed_pattern();
    let (x0, y0) = (
        frame.width().saturating_sub(TRAY_W) / 2,
        frame.height().saturating_sub(TRAY_H) / 2,
    );
    let cropped = Pattern::new(
        frame.width().min(TRAY_W),
        frame.height().min(TRAY_H),
        frame
            .cells()
            .iter()
            .filter(|&&(x, y)| x >= x0 && y >= y0 && x < x0 + TRAY_W && y < y0 + TRAY_H)
            .map(|&(x, y)| (x - x0, y - y0)),
    );
    let want = placed(
        &cropped,
        (TRAY_W / 2) as i64,
        (TRAY_H / 2) as i64,
        Anchor::Center,
    );
    assert_eq!(sorted(alive(&h)), sorted(want.clone()));
    assert_eq!(
        tray_canvas(&h),
        format!("Cell tray, 40 by 26, generation 0, {} alive", want.len())
    );
    assert!(
        toasts(&h)
            .iter()
            .any(|t| t.contains(&format!("{chosen} from Rule lab"))),
        "{:?}",
        toasts(&h)
    );
}

#[test]
fn terrarium_to_rule_lab_to_tray() {
    terrarium_journey(W, H);
}

/// (f) The Terrarium journey on a phone.
#[test]
fn terrarium_journey_on_a_phone() {
    terrarium_journey(PHONE_W, PHONE_H);
}

// ---- (c) the Loom --------------------------------------------------------------------------

/// The rule number the aside shows ("Rule 30, binary 00011110").
fn loom_rule(h: &App) -> u8 {
    let t = find_text(h, |t| t.starts_with("Rule ") && t.contains(", binary "))
        .expect("the rule number");
    let (n, bin) = t["Rule ".len()..].split_once(", binary ").unwrap();
    let n: u8 = n.parse().unwrap();
    assert_eq!(bin, format!("{n:08b}"), "the binary follows the number");
    n
}

/// Presses the key for pattern `p` (its name says what it makes now).
fn press_key(h: &mut App, p: u8) {
    let rule = loom_rule(h);
    click_button(h, &key_name(rule, p));
    assert_eq!(loom_rule(h), rule ^ (1 << p), "key {p:03b} flips bit {p}");
}

/// From rule 30, clear every bit with the keys (rule 0), then set 90's: the aside, the
/// famous threads, the print head and the canvas follow. Weave from a single cell at 14
/// rows/s, pause, Share: every row `n` of the shared text has 2^popcount(n) cells (Gould's
/// sequence, REF-1).
#[test]
fn loom_builds_rule_90_and_weaves_sierpinski() {
    let mut h = harness(Route::Loom);
    assert_eq!(loom_rule(&h), 30);
    assert_eq!(pressed(&h, "30 Chaos"), Some(true));
    for p in 0..8 {
        if loom_rule(&h) >> p & 1 == 1 {
            press_key(&mut h, p);
        }
    }
    assert_eq!(loom_rule(&h), 0);
    assert!(has_label(&h, "Uncharted rule"));
    assert_eq!(pressed(&h, "30 Chaos"), Some(false));
    for p in 0..8 {
        if 90u8 >> p & 1 == 1 {
            press_key(&mut h, p);
        }
    }
    assert_eq!(loom_rule(&h), 90);
    assert_eq!(pressed(&h, "90 Sierpiński"), Some(true));
    assert!(has_label(&h, "Sierpiński · Fractal triangles"));
    for p in 0..8u8 {
        let on = 90u8 >> p & 1 == 1;
        assert_eq!(pressed(&h, &key_name(90, p)), Some(on), "key {p:03b}");
    }
    assert_eq!(pressed(&h, "Single cell"), Some(true));
    assert_eq!(counter(&h, "row"), 1, "a rule change restarts the thread");

    // weave at 14 rows/s: 42 rows in three simulated seconds
    click_button(&mut h, "Weave");
    get(&h, "Pause", Role::Button);
    let r0 = counter(&h, "row");
    h.run_steps(30);
    assert_eq!(counter(&h, "row") - r0, 42, "3 s at 14 rows/s");
    click_button(&mut h, "Pause");
    let rows = counter(&h, "row");
    assert!(rows < 72, "every row still visible: {rows}");
    assert_eq!(
        role_label(&h, Role::Image, "Woven pattern"),
        format!("Woven pattern, rule 90, 121 cells wide, {rows} rows woven")
    );

    click_button(&mut h, "Share");
    let text = clipboard(&h);
    let mut lines = text.lines();
    assert_eq!(lines.next(), Some("Rule 90"));
    let lines: Vec<&str> = lines.collect();
    assert_eq!(lines.len() as u64, rows, "every row (fewer than 72)");
    for (n, line) in lines.iter().enumerate() {
        assert_eq!(line.chars().count(), 121);
        let live = line.chars().filter(|&c| c == '█').count();
        assert_eq!(live, 1 << (n as u32).count_ones(), "row {n}: {line}");
    }
    // the apex row is the single centre cell
    assert_eq!(lines[0].chars().position(|c| c == '█'), Some(60));
}

// ---- (d) the Series ---------------------------------------------------------------------------

/// The Series with Chem's card highlighted: Open launches it through the launcher; Bio's Open
/// is disabled; the brand goes back to the Tray.
#[test]
fn series_open_and_back() {
    let launcher = Launcher::recording(&[AppId::Chem]);
    let mut h = app().launcher(launcher.clone()).build();
    assert_eq!(route(&h), Route::Tray);
    h.state_mut().open_series(AppId::Chem);
    settle(&mut h);
    assert_eq!(route(&h), Route::Series);
    assert!(has_label(&h, "series"), "the document name");
    for app in AppId::ALL {
        assert_eq!(
            selected(&h, app.name(), Role::Group),
            Some(app == AppId::Chem),
            "{app:?}"
        );
    }
    click_button(&mut h, "Open Chem Playground");
    assert_eq!(launcher.calls(), [AppId::Chem]);
    assert!(toasts(&h).iter().any(|t| t == "Opening Chem Playground…"));
    let bio = "Open Bio Playground (planned, not built yet)";
    assert!(disabled(&h, bio));
    h.get_by_label(bio).click();
    settle(&mut h);
    assert_eq!(
        launcher.calls(),
        [AppId::Chem],
        "the planned app never launches"
    );

    click_link(&mut h, "Automata Playground");
    assert_eq!(route(&h), Route::Tray);
    assert!(has_label(&h, "glider-garden"));
}

// ---- (e) keyboard only ------------------------------------------------------------------------

/// Presses Tab until the focused node's label is `want` (at most `max` presses).
fn tab_to(h: &mut App, want: &str, shift: bool, max: usize) {
    for _ in 0..max {
        if focused_label(h).as_deref() == Some(want) {
            return;
        }
        if shift {
            h.key_press_modifiers(Modifiers::SHIFT, Key::Tab);
        } else {
            h.key_press(Key::Tab);
        }
        h.step();
    }
    assert_eq!(
        focused_label(h).as_deref(),
        Some(want),
        "Tab never reached it"
    );
}

/// NAV-5 without the pointer: ⌘1–⌘4 switch screens, Space runs and pauses, `S` steps, Tab
/// reaches the header's Step and the tool row's Clear and Enter / Space press them, Tab
/// focuses the tray where the arrows and Space draw a blinker, Shift-Tab back to Run, and the
/// blinker blinks.
#[test]
fn keyboard_only() {
    let mut h = harness(Route::Tray);
    for (key, r) in [
        (Key::Num2, Route::Loom),
        (Key::Num3, Route::Terrarium),
        (Key::Num4, Route::RuleLab),
        (Key::Num1, Route::Tray),
    ] {
        press_cmd(&mut h, key);
        settle(&mut h);
        assert_eq!(route(&h), r, "⌘{key:?}");
    }

    // Space: Run / Pause; S: one step
    let g = counter(&h, "gen");
    press(&mut h, Key::Space);
    get(&h, "Pause", Role::Button);
    h.run_steps(5);
    press(&mut h, Key::Space);
    settle(&mut h);
    get(&h, "Run", Role::Button);
    let g1 = counter(&h, "gen");
    assert!(g1 >= g + 5, "ran: {g} → {g1}");
    press(&mut h, Key::S);
    settle(&mut h);
    assert_eq!(counter(&h, "gen"), g1 + 1, "S steps");

    // Tab to the header's Step, Enter presses it
    tab_to(&mut h, "Step", false, 30);
    press(&mut h, Key::Enter);
    settle(&mut h);
    assert_eq!(counter(&h, "gen"), g1 + 2);
    // on to Clear in the tool row, Space presses it
    tab_to(&mut h, "Clear", false, 40);
    press(&mut h, Key::Space);
    settle(&mut h);
    assert!(alive(&h).is_empty(), "Clear by keyboard");
    // the tray is next in reading order: the arrows move the cursor, Space draws
    h.key_press(Key::Tab);
    h.step();
    let focused = focused_label(&h).unwrap_or_default();
    assert!(focused.starts_with("Cell tray"), "{focused}");
    press(&mut h, Key::ArrowLeft);
    press(&mut h, Key::Space);
    press(&mut h, Key::ArrowRight);
    press(&mut h, Key::Space);
    press(&mut h, Key::ArrowRight);
    press(&mut h, Key::Space);
    settle(&mut h);
    assert_eq!(alive(&h), [(19, 13), (20, 13), (21, 13)]);

    // back to Run with Shift-Tab, Enter: the blinker blinks (period 2)
    let drawn_at = counter(&h, "gen");
    tab_to(&mut h, "Run", true, 60);
    press(&mut h, Key::Enter);
    get(&h, "Pause", Role::Button);
    let horizontal = vec![(19, 13), (20, 13), (21, 13)];
    let vertical = vec![(20, 12), (20, 13), (20, 14)];
    for _ in 0..4 {
        h.step();
        let g = counter(&h, "gen");
        let want = if (g - drawn_at).is_multiple_of(2) {
            &horizontal
        } else {
            &vertical
        };
        assert_eq!(&alive(&h), want, "gen {g}");
    }
}

/// (f) On a phone, keyboard focus and AccessKit "scroll into view" move the page to controls
/// in the stacked asides (an aside's own scroll area used to swallow the request).
#[test]
fn phone_focus_scrolls_the_page() {
    let mut h = app().at(Route::Loom).phone().build();
    let view = viewport(&h);
    let far = get(&h, "Surprise me", Role::Button).rect();
    assert!(far.top() > view.bottom(), "starts below the fold: {far:?}");
    tab_to(&mut h, "Surprise me", false, 60);
    settle(&mut h);
    let r = get(&h, "Surprise me", Role::Button).rect();
    assert!(viewport(&h).contains_rect(r), "focus scrolled it in: {r:?}");
    let before = loom_rule(&h);
    press(&mut h, Key::Enter);
    settle(&mut h);
    assert_ne!(loom_rule(&h), before);

    let mut h = app().at(Route::Terrarium).phone().build();
    let far = get(&h, "Breed brood 2", Role::Button).rect();
    assert!(far.top() > viewport(&h).bottom(), "{far:?}");
    get(&h, "Breed brood 2", Role::Button).scroll_to_me();
    settle(&mut h);
    let r = get(&h, "Breed brood 2", Role::Button).rect();
    assert!(
        viewport(&h).contains_rect(r),
        "ScrollIntoView reached the page: {r:?}"
    );
}

// ---- (g) every screen: screenshot, names, descriptions --------------------------------------

/// Roles a user acts on (A11Y-1).
const INTERACTIVE: [Role; 9] = [
    Role::Button,
    Role::Link,
    Role::Tab,
    Role::CheckBox,
    Role::Slider,
    Role::TextInput,
    Role::MultilineTextInput,
    Role::Grid,
    Role::MenuItem,
];

/// The route's canvas: its role and the start of its description.
fn canvas(route: Route) -> Option<(Role, &'static str)> {
    match route {
        Route::Tray => Some((Role::Grid, "Cell tray")),
        Route::Loom => Some((Role::Image, "Woven pattern")),
        Route::Terrarium => Some((Role::Image, "Jar running rule")),
        Route::RuleLab => Some((Role::Image, "Top-down view of the grid")),
        Route::Series => None,
    }
}

/// Changes what the route's canvas shows, through the GUI.
fn advance(h: &mut App, route: Route) {
    match route {
        Route::Tray => click_button(h, "Step"),
        Route::Loom | Route::Terrarium => {
            let run = if route == Route::Loom { "Weave" } else { "Run" };
            click_button(h, run);
            h.run_steps(6);
            click_button(h, "Pause");
        }
        Route::RuleLab => {
            get(h, "Scrub", Role::Slider).set_value_accesskit(200.0);
            settle(h);
        }
        Route::Series => {}
    }
}

fn smoke(w: f32, hgt: f32, tag: &str) {
    let mut h = app().size(w, hgt).build();
    for r in Route::ALL {
        goto(&mut h, r);
        let shot = screenshot(&mut h, &format!("journey-{}-{tag}", r.id()));
        assert!(shot.exists());
        // A11Y-1: every interactive node has a name
        for (role, label, rect) in nodes_with_roles(&h, &INTERACTIVE) {
            assert!(
                !label.trim().is_empty(),
                "{r:?} {tag}: unnamed {role:?} at {rect:?}"
            );
        }
        // A11Y-4: canvases describe themselves, and the description follows the content
        if let Some((role, prefix)) = canvas(r) {
            let first = texts(&h)
                .into_iter()
                .find(|(ro, t)| *ro == role && t.starts_with(prefix))
                .map(|(_, t)| t)
                .unwrap_or_else(|| panic!("{r:?}: no canvas description {prefix:?}"));
            advance(&mut h, r);
            let all: Vec<String> = texts(&h)
                .into_iter()
                .filter(|(ro, t)| *ro == role && t.starts_with(prefix))
                .map(|(_, t)| t)
                .collect();
            assert!(!all.is_empty(), "{r:?}: the description went away");
            assert!(
                !all.contains(&first) || r == Route::Terrarium && all.len() > 1,
                "{r:?} {tag}: {first:?} didn't change ({all:?})"
            );
        }
    }
}

#[test]
fn every_screen_renders_names_and_describes_itself() {
    smoke(W, H, "1440");
    smoke(PHONE_W, PHONE_H, "390");
}

/// The jars' descriptions follow their populations as they tick.
#[test]
fn jar_descriptions_follow_the_jars() {
    let mut h = harness(Route::Terrarium);
    let before: Vec<String> = texts(&h)
        .into_iter()
        .filter(|(r, t)| *r == Role::Image && t.starts_with("Jar running rule"))
        .map(|(_, t)| t)
        .collect();
    assert_eq!(before.len(), 6);
    click_button(&mut h, "Run");
    h.run_steps(8);
    click_button(&mut h, "Pause");
    let after: Vec<String> = texts(&h)
        .into_iter()
        .filter(|(r, t)| *r == Role::Image && t.starts_with("Jar running rule"))
        .map(|(_, t)| t)
        .collect();
    assert_eq!(after.len(), 6);
    assert_ne!(before, after, "populations changed");
    for (n, d) in after.iter().enumerate() {
        let (code, _) = jar_card(&h, n + 1);
        assert!(d.starts_with(&format!("Jar running rule {code}, ")), "{d}");
    }
}
