//! Terrarium · breed (SCR-3, TERR-1…4, NAV-4) through the real app: the six jars, Keep and
//! the keep note, breeding with and without kept parents, the lineage, Mutation, Fresh soup,
//! running, the hand-offs to the Tray and the Rule lab, Share, the keyboard, accessibility,
//! and renders at 1440 and 390.

mod common;

use ap_engine::LifeRule;
use ap_engine::presets::{TERRARIUM_START, rule_name};
use automata_playground::Route;
use automata_playground::screens::terrarium;
use common::*;
use eframe::egui::{self, Key, accesskit::Role};
use egui_kittest::kittest::{NodeT, Queryable};

// ---- helpers --------------------------------------------------------------------------------

fn terr(h: &App) -> &terrarium::State {
    &h.state().screens().terrarium
}

fn terr_mut(h: &mut App) -> &mut terrarium::State {
    &mut h.state_mut().screens_mut().terrarium
}

/// A 390 px wide app as tall as its page, so nothing needs the page to scroll (a column's own
/// scroll area keeps `scroll_to_me` from scrolling the page; see PROGRESS.md › Requests).
fn phone_full(route: Route) -> App {
    let mut h = app().at(route).phone().build();
    let need = h.state().content_height().ceil();
    h.set_size(egui::vec2(PHONE_W, need.max(PHONE_H)));
    settle(&mut h);
    h
}

fn codes(h: &App) -> Vec<String> {
    terr(h).jars().into_iter().map(|j| j.code).collect()
}

fn populations(h: &App) -> Vec<usize> {
    terr(h).jars().into_iter().map(|j| j.population).collect()
}

fn start_codes() -> Vec<String> {
    TERRARIUM_START.iter().map(LifeRule::code).collect()
}

// ---- the jars (TERR-1) ----------------------------------------------------------------------

/// Six jars with the starting rules, names, populations and descriptions; the code is never
/// under the jar (the mockup's overlap is fixed).
#[test]
fn six_jars_with_their_rules() {
    let h = harness(Route::Terrarium);
    let jars = terr(&h).jars();
    assert_eq!(
        codes(&h),
        [
            "B3/S23",
            "B36/S23",
            "B3678/S34678",
            "B3/S12345",
            "B36/S125",
            "B368/S245"
        ]
    );
    let names: Vec<&str> = jars.iter().map(|j| j.name).collect();
    assert_eq!(
        names,
        ["Life", "HighLife", "Day & Night", "Maze", "2×2", "Move"]
    );
    for (i, j) in jars.iter().enumerate() {
        let card = get(&h, &format!("Jar {}", i + 1), Role::Group).rect();
        let jar = get(
            &h,
            &format!("Jar running rule {}, {} cells alive", j.code, j.population),
            Role::Image,
        )
        .rect();
        assert!(card.contains_rect(jar), "jar {i} inside its card");
        assert!(j.population > 0 && !j.kept && j.liveliness == 0.0);
        assert!(has_label(&h, &j.code));
        assert!(has_label(&h, j.name));
        assert!(has_label(&h, &format!("{} alive", j.population)));
        assert_eq!(pressed(&h, &format!("Keep jar {}", i + 1)), Some(false));
        // TERR-1: the rule code sits below the whole jar (slab and shadow included)
        let code = h
            .query_all_by_label(&j.code)
            .map(|n| n.rect())
            .find(|r| card.contains_rect(*r))
            .expect("code label in the card");
        assert!(
            code.top() >= jar.bottom(),
            "jar {i}: {jar:?} covers {code:?}"
        );
    }
}

/// Keep / Kept toggle (pressed state) and the note says which jars are parents.
#[test]
fn keep_toggles_and_the_note_follows() {
    let mut h = harness(Route::Terrarium);
    assert!(has_label(&h, "None kept: all six become parents"));
    click_button(&mut h, "Keep jar 2");
    assert_eq!(pressed(&h, "Keep jar 2"), Some(true));
    assert!(terr(&h).jars()[1].kept);
    assert!(has_label(&h, "1 kept as parents"));
    click_button(&mut h, "Keep jar 5");
    assert!(has_label(&h, "2 kept as parents"));
    click_button(&mut h, "Keep jar 2");
    assert_eq!(pressed(&h, "Keep jar 2"), Some(false));
    assert!(has_label(&h, "1 kept as parents"));
    assert_eq!(
        terr(&h).chosen_rule(),
        "B36/S125",
        "jar 5 is now the first kept"
    );
    click_button(&mut h, "Keep jar 5");
    assert!(has_label(&h, "None kept: all six become parents"));
}

// ---- breeding (TERR-2/3) --------------------------------------------------------------------

/// With nothing kept all six are parents: six children, brood 2, a lineage entry listing the
/// six parents.
#[test]
fn breeding_with_nothing_kept() {
    let mut h = harness(Route::Terrarium);
    assert!(has_label(&h, "Nothing bred yet."));
    assert!(has_label(&h, "brood 1"));
    click_button(&mut h, "Breed brood 2");
    assert_eq!(terr(&h).brood(), 2);
    assert!(has_label(&h, "brood 2"), "the header counter");
    get(&h, "Breed brood 3", Role::Button);
    assert!(!has_label(&h, "Nothing bred yet."));
    let lineage = terr(&h).lineage();
    assert_eq!(lineage.len(), 1);
    assert_eq!(lineage[0].brood, 1);
    assert_eq!(lineage[0].parents, start_codes());
    // egui puts a label's text in its AccessKit value
    let labels: Vec<String> = get(&h, "brood 1 parents", Role::Group)
        .children_recursive()
        .filter(|n| n.accesskit_node().role() == Role::Label)
        .filter_map(|n| n.accesskit_node().value())
        .collect();
    assert_eq!(labels[0], "brood 1");
    assert_eq!(labels[1..], start_codes()[..], "one chip per parent");
    // six children, every one a valid Moore rule with a name or "Mutant"
    let jars = terr(&h).jars();
    assert_eq!(jars.len(), 6);
    for j in &jars {
        let rule = LifeRule::parse(&j.code).expect("a valid child rule");
        assert!(!rule.birth().is_empty(), "a child always has a birth count");
        assert_eq!(j.name, rule_name(&rule).unwrap_or("Mutant"));
        assert!(has_label(&h, &j.code));
    }
    // a second brood lists newest first
    click_button(&mut h, "Breed brood 3");
    let lineage = terr(&h).lineage();
    assert_eq!(lineage.iter().map(|l| l.brood).collect::<Vec<_>>(), [2, 1]);
    let first = get(&h, "brood 2 parents", Role::Group).rect();
    let second = get(&h, "brood 1 parents", Role::Group).rect();
    assert!(first.bottom() < second.top(), "newest first, apart");
}

/// Kept jars carry over unchanged in the first slots (in keep order) and are the only
/// parents; keeps are cleared after breeding.
#[test]
fn breeding_with_two_kept() {
    let mut h = harness(Route::Terrarium);
    click_button(&mut h, "Keep jar 4");
    click_button(&mut h, "Keep jar 1");
    click_button(&mut h, "Breed brood 2");
    let c = codes(&h);
    assert_eq!(c[0], "B3/S12345", "jar 4 kept first");
    assert_eq!(c[1], "B3/S23");
    assert_eq!(
        terr(&h).lineage()[0].parents,
        ["B3/S12345".to_string(), "B3/S23".to_string()]
    );
    assert!(terr(&h).jars().iter().all(|j| !j.kept));
    assert!(has_label(&h, "None kept: all six become parents"));
    assert_eq!(terr(&h).jars()[0].name, "Maze");
}

/// The Mutation slider: 1–4 bit flips (default 2).
#[test]
fn mutation_slider() {
    let mut h = harness(Route::Terrarium);
    assert_eq!(terr(&h).mutation(), 2);
    assert!(has_label(&h, "Mutation: 2 bit flips"));
    get(&h, "Mutation", Role::Slider).increment_accesskit();
    settle(&mut h);
    assert_eq!(terr(&h).mutation(), 3);
    assert!(has_label(&h, "Mutation: 3 bit flips"));
    get(&h, "Mutation", Role::Slider).set_value_accesskit(9.0);
    settle(&mut h);
    assert_eq!(terr(&h).mutation(), 4);
    get(&h, "Mutation", Role::Slider).set_value_accesskit(1.0);
    settle(&mut h);
    assert_eq!(terr(&h).mutation(), 1);
    assert!(has_label(&h, "Mutation: 1 bit flip"));
}

// ---- running and soup (TERR-1/4) ------------------------------------------------------------

/// Fresh soup re-seeds every jar: new populations, same rules.
#[test]
fn fresh_soup_reseeds() {
    let mut h = harness(Route::Terrarium);
    let before = populations(&h);
    click_button(&mut h, "Fresh soup");
    assert_eq!(codes(&h), start_codes(), "rules unchanged");
    assert_ne!(populations(&h), before, "new soups");
    for (i, j) in terr(&h).jars().iter().enumerate() {
        get(
            &h,
            &format!("Jar running rule {}, {} cells alive", j.code, j.population),
            Role::Image,
        );
        assert_eq!(j.liveliness, 0.0, "jar {i}: the activity window restarts");
    }
}

/// Run ticks the jars every 140 ms (populations and liveliness change, the meters show it);
/// Pause stops them.
#[test]
fn running_changes_populations_and_liveliness() {
    let mut h = harness(Route::Terrarium);
    let before = populations(&h);
    click_button(&mut h, "Run");
    assert!(terr(&h).running());
    get(&h, "Pause", Role::Button);
    h.run_steps(10); // 1 s of jar time: 7 ticks
    let after = terr(&h).jars();
    assert_ne!(populations(&h), before);
    assert!(after.iter().all(|j| j.liveliness > 0.0), "{after:?}");
    for j in &after {
        let percent = (j.liveliness * 100.0).round();
        assert!(has_label(&h, &format!("Liveliness {percent}%")));
    }
    click_button(&mut h, "Pause");
    let paused = populations(&h);
    h.run_steps(5);
    assert_eq!(populations(&h), paused);
    // Space toggles too (NAV-5), S ticks once while paused
    press(&mut h, Key::Space);
    assert!(terr(&h).running());
    press(&mut h, Key::Space);
    let grids = |h: &App| -> Vec<ap_engine::Grid> {
        terr(h)
            .terrarium()
            .jars()
            .iter()
            .map(|j| j.grid().clone())
            .collect()
    };
    let before = grids(&h);
    press(&mut h, Key::S);
    assert_ne!(grids(&h), before, "one generation");
    assert!(!terr(&h).running());
}

/// Like the real app (autoplay on), the jars start running when first shown.
#[test]
fn autoplay_runs_on_enter() {
    let mut h = app().at(Route::Terrarium).autoplay().build();
    assert!(terr(&h).running());
    get(&h, "Pause", Role::Button);
    let before = populations(&h);
    h.run_steps(4);
    assert_ne!(populations(&h), before);
}

// ---- hand-offs and Share (NAV-4, TERR-4) ----------------------------------------------------

/// Plant in the Tray / Analyse in Rule lab carry the first kept jar's rule (or jar 1's).
#[test]
fn favourites_go_to_the_tray_and_the_rule_lab() {
    let mut h = harness(Route::Terrarium);
    click_button(&mut h, "Keep jar 3");
    click_button(&mut h, "Keep jar 2");
    click_link(&mut h, "Plant in the Tray");
    assert_eq!(route(&h), Route::Tray);
    assert_eq!(h.state().screens().tray.rule().code(), "B3678/S34678");

    goto(&mut h, Route::Terrarium);
    assert_eq!(
        pressed(&h, "Keep jar 3"),
        Some(true),
        "state survives navigation"
    );
    click_button(&mut h, "Keep jar 3");
    click_button(&mut h, "Keep jar 2");
    click_link(&mut h, "Analyse in Rule lab");
    assert_eq!(route(&h), Route::RuleLab);
    assert_eq!(
        h.state().screens().rule_lab.spec().rule.code(),
        "B3/S23",
        "nothing kept: jar 1"
    );
}

/// Share copies the six codes with their names and says so.
#[test]
fn share_copies_the_jars() {
    let mut h = harness(Route::Terrarium);
    click_button(&mut h, "Share");
    let copied = h.state().last_copied().expect("copied").to_string();
    assert_eq!(copied, terr(&h).share_text());
    let lines: Vec<&str> = copied.lines().collect();
    assert_eq!(lines.len(), 6);
    assert_eq!(lines[0], "Jar 1 · B3/S23 · Life");
    assert_eq!(lines[4], "Jar 5 · B36/S125 · 2×2");
    assert!(toasts(&h).contains(&"Copied the six jars' rules".to_string()));
}

/// The control API (for the MCP bridge) drives the same state the GUI shows.
#[test]
fn control_api() {
    let mut h = harness(Route::Terrarium);
    let s = terr_mut(&mut h);
    s.keep(0, true);
    s.keep(5, true);
    s.set_mutation(3);
    s.breed();
    s.tick_n(4);
    s.set_running(true);
    settle(&mut h);
    assert_eq!(terr(&h).brood(), 2);
    assert!(has_label(&h, "brood 2"));
    assert_eq!(codes(&h)[..2], ["B3/S23", "B368/S245"]);
    assert!(has_label(&h, "Mutation: 3 bit flips"));
    get(&h, "Pause", Role::Button);
    terr_mut(&mut h).fresh_soup();
    terr_mut(&mut h).set_running(false);
    settle(&mut h);
    get(&h, "Run", Role::Button);
}

// ---- renders (DONE-1) -----------------------------------------------------------------------

/// At 1440 × 900 the cards sit three to a row where the reference puts them (the second row
/// 22 px lower: the jar keeps its whole footprint) and the aside matches the reference.
#[test]
fn renders_like_the_mockup_at_1440() {
    let mut h = harness(Route::Terrarium);
    let img = render(&mut h);
    save(&img, &screenshot_dir().join("terrarium-1440.png"));
    let mock = load(&mockup_png(Route::Terrarium, 1440));
    let aside = mean_diff(&img, &mock, Some((1121, 57, 319, 843)));
    let chrome = mean_diff(&img, &mock, Some((0, 0, 1440, 102)));
    println!("terrarium 1440: aside |Δ| = {aside:.2}, header + tabs |Δ| = {chrome:.2}");
    assert!(aside < 2.5, "aside drifted: {aside:.2}");
    // the header's Run vs Pause differs (the reference is running)
    assert!(chrome < 3.0, "header drifted: {chrome:.2}");
    // cards: 22 px stage padding, ≈ 346 px wide, 18 px apart (reference: 23–368, 387–732)
    let c1 = get(&h, "Jar 1", Role::Group).rect();
    let c2 = get(&h, "Jar 2", Role::Group).rect();
    let c4 = get(&h, "Jar 4", Role::Group).rect();
    assert!(
        (c1.left() - 22.0).abs() < 1.5 && (c1.top() - 124.0).abs() < 1.5,
        "{c1:?}"
    );
    assert!((c1.width() - 345.9).abs() < 1.5, "{c1:?}");
    assert!((c2.left() - c1.right() - 18.0).abs() < 0.5);
    assert!((c4.left() - c1.left()).abs() < 0.5 && (c4.top() - c1.bottom() - 18.0).abs() < 0.5);
    assert!(c4.bottom() <= 900.0, "both rows fit at 1440 × 900: {c4:?}");
}

/// At 390 the cards stack one per row inside the scrolling stage, the aside follows, and
/// everything still works.
#[test]
fn works_at_390() {
    let mut h = phone_full(Route::Terrarium);
    let c1 = get(&h, "Jar 1", Role::Group).rect();
    assert!(c1.left() >= 0.0 && c1.right() <= PHONE_W, "{c1:?}");
    let jars = get(&h, "Jars", Role::Region).rect();
    let aside = get(&h, "Breeding", Role::Region).rect();
    assert!(aside.top() >= jars.bottom() - 0.5, "stacked");
    // jar 6 is below the stage's fold: Keep scrolls it into view inside the stage
    click_button(&mut h, "Keep jar 6");
    assert_eq!(pressed(&h, "Keep jar 6"), Some(true));
    click_button(&mut h, "Breed brood 2");
    assert_eq!(codes(&h)[0], "B368/S245");

    let mock = load(&mockup_png(Route::Terrarium, 390));
    let img = render_full(Route::Terrarium, PHONE_W, mock.height() as f32);
    save(&img, &screenshot_dir().join("terrarium-390.png"));
    let d = mean_diff(&img, &mock, Some((0, 1032, 390, 460)));
    println!("terrarium 390: aside |Δ| = {d:.2}");
    assert!(d < 3.0, "390 aside drifted: {d:.2}");
}
