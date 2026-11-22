//! Navigation (NAV-1/2/3/4/5) driven through AccessKit: the brand, both modes and every bench
//! tab land on the right route from every screen, at 1440 and at 390 px; the header has no app
//! switcher; ⌘1–⌘4, Space and `S`; the Playground's last-bench memory; carry-over delivery.

mod common;

use automata_playground::{Carry, Mode, Route};
use common::*;
use eframe::egui::{Key, accesskit::Role};
use egui_kittest::kittest::{NodeT, Queryable};

fn check_header_paths(w: f32, hgt: f32) {
    let mut h = app().size(w, hgt).build();
    for from in Route::ALL {
        goto(&mut h, from);
        click_link(&mut h, "Automata Playground");
        assert_eq!(route(&h), Route::Tray, "brand from {from:?} at {w}");

        goto(&mut h, from);
        click_link(&mut h, "Advanced");
        assert_eq!(route(&h), Route::RuleLab, "Advanced from {from:?} at {w}");

        goto(&mut h, from);
        let last = h.state().last_bench();
        click_link(&mut h, "Playground");
        assert_eq!(route(&h), last, "Playground from {from:?} at {w}");

        if from.is_bench() {
            for tab in Route::BENCHES {
                goto(&mut h, from);
                click_tab(&mut h, tab.tab_label().unwrap());
                assert_eq!(route(&h), tab, "tab {tab:?} from {from:?} at {w}");
            }
        }
    }
}

#[test]
fn header_paths_from_every_screen_desktop() {
    check_header_paths(W, H);
}

#[test]
fn header_paths_from_every_screen_phone() {
    check_header_paths(PHONE_W, PHONE_H);
}

/// The tab row is a `TabList` named "Benches" on the three benches only, with the current
/// tab selected.
#[test]
fn bench_tab_row_follows_the_route() {
    let mut h = harness(Route::Tray);
    for r in Route::ALL {
        goto(&mut h, r);
        let has_tabs = h
            .query_by_role_and_label(Role::TabList, "Benches")
            .is_some();
        assert_eq!(has_tabs, r.is_bench(), "{r:?}");
        if r.is_bench() {
            let tabs = child_labels(&h, "Benches", Role::TabList, Role::Tab);
            assert_eq!(tabs, ["Tray · 2D", "Loom · 1D", "Terrarium · breed"]);
            for b in Route::BENCHES {
                let label = b.tab_label().unwrap();
                assert_eq!(selected(&h, label, Role::Tab), Some(b == r), "{r:?} {b:?}");
            }
        }
    }
}

/// The mode switcher is a `Navigation` "Mode" with two links; the current mode is selected
/// (none on the Series screen). There is no app switcher (NAV-3: the apps don't switch yet).
#[test]
fn mode_switcher_reports_the_current_one_and_no_app_switcher() {
    let mut h = harness(Route::Tray);
    for r in Route::ALL {
        goto(&mut h, r);
        assert_eq!(
            child_labels(&h, "Mode", Role::Navigation, Role::Link),
            ["Playground", "Advanced"]
        );
        for m in Mode::ALL {
            assert_eq!(
                selected(&h, m.label(), Role::Link),
                Some(r.mode() == Some(m)),
                "{r:?} {m:?}"
            );
        }
        assert!(h.query_by_label("Switch app").is_none(), "{r:?}");
        for label in ["Breadbox circuit app", "Automata Playground (current)"] {
            assert!(h.query_by_label(label).is_none(), "{r:?}: {label}");
        }
        // the sibling names only appear on the Series screen's cards, never as header links
        assert!(
            h.query_all_by_label("Chem Playground")
                .all(|n| n.accesskit_node().role() != Role::Link),
            "{r:?}"
        );
    }
}

/// NAV-1: the header shows the route's document name.
#[test]
fn header_shows_the_document_name() {
    let mut h = harness(Route::Tray);
    for r in Route::ALL {
        goto(&mut h, r);
        assert!(has_label(&h, "Automata Playground"));
        let doc = get(&h, r.document(), Role::Label).rect();
        let brand = get(&h, "Automata Playground", Role::Link).rect();
        assert!(
            brand.contains(doc.center()),
            "{r:?}: document name inside the brand"
        );
    }
}

/// "Playground" opens the last bench used (Tray at first); Advanced doesn't change it.
#[test]
fn playground_remembers_the_last_bench() {
    let mut h = harness(Route::Tray);
    assert_eq!(h.state().last_bench(), Route::Tray);
    click_tab(&mut h, "Terrarium · breed");
    click_link(&mut h, "Advanced");
    assert_eq!(route(&h), Route::RuleLab);
    click_link(&mut h, "Playground");
    assert_eq!(route(&h), Route::Terrarium);
    // via a shortcut and the Series screen too
    press_cmd(&mut h, Key::Num2);
    settle(&mut h);
    h.state_mut().navigate(Route::Series);
    settle(&mut h);
    assert_eq!(route(&h), Route::Series);
    click_link(&mut h, "Playground");
    assert_eq!(route(&h), Route::Loom);
}

/// ⌘1–⌘3 switch benches and ⌘4 opens the Rule lab, from every screen.
#[test]
fn command_digit_shortcuts() {
    let mut h = harness(Route::Tray);
    for from in Route::ALL {
        for (key, to) in [
            (Key::Num1, Route::Tray),
            (Key::Num2, Route::Loom),
            (Key::Num3, Route::Terrarium),
            (Key::Num4, Route::RuleLab),
        ] {
            goto(&mut h, from);
            press_cmd(&mut h, key);
            settle(&mut h);
            assert_eq!(route(&h), to, "{key:?} from {from:?}");
        }
    }
}

/// Space toggles Run / Pause and `S` steps when nothing has focus; a focused control gets
/// Space for itself (it's activated instead).
#[test]
fn space_and_s_reach_the_active_screen() {
    let mut h = harness(Route::Tray);
    assert!(!h.state().screens().tray.running, "tests start paused");
    press(&mut h, Key::Space);
    assert!(h.state().screens().tray.running, "Space runs");
    press(&mut h, Key::Space);
    assert!(!h.state().screens().tray.running, "Space pauses");
    // (it advanced while running; S adds exactly one generation each and stays paused)
    let before = h.state().screens().tray.generation;
    press(&mut h, Key::S);
    press(&mut h, Key::S);
    assert_eq!(h.state().screens().tray.generation, before + 2, "S steps");
    assert!(!h.state().screens().tray.running);
    // the header shows it
    settle(&mut h);
    assert!(has_label(&h, &format!("gen {:04}", before + 2)));

    // with a tab focused, Space activates the tab and doesn't toggle the tray
    h.get_by_role_and_label(Role::Tab, "Loom · 1D").focus();
    settle(&mut h);
    press(&mut h, Key::Space);
    settle(&mut h);
    assert_eq!(route(&h), Route::Loom);
    assert!(!h.state().screens().tray.running);

    // the Rule lab maps Space to Play / Pause
    goto(&mut h, Route::RuleLab);
    h.step();
    let before = h.state().screens().rule_lab.playing;
    press(&mut h, Key::Space);
    assert_ne!(h.state().screens().rule_lab.playing, before);
}

/// Real-app options start the benches running (TRAY-7 "Runs on start").
#[test]
fn autoplay_starts_running() {
    let h = app().autoplay().build();
    assert!(h.state().screens().tray.running);
    assert!(has_label(&h, "Pause"));
}

/// NAV-4: a carry-over is delivered to the target screen after navigation.
#[test]
fn carry_over_reaches_the_target() {
    let mut h = harness(Route::Terrarium);
    h.state_mut()
        .navigate_with(Route::RuleLab, Carry::rule("B36/S23", Route::Terrarium));
    settle(&mut h);
    assert_eq!(route(&h), Route::RuleLab);
    let got = h.state().screens().rule_lab.received.clone();
    assert_eq!(got.map(|c| c.rule), Some("B36/S23".to_string()));
    assert!(toasts(&h).iter().any(|t| t.contains("B36/S23")));

    h.state_mut().navigate_with(
        Route::Tray,
        Carry::rule("B3/S23", Route::RuleLab).with_rle("x = 3, y = 1, rule = B3/S23\n3o!"),
    );
    settle(&mut h);
    let got = h.state().screens().tray.received.clone().unwrap();
    assert_eq!(got.from, Route::RuleLab);
    assert!(got.rle.unwrap().ends_with("3o!"));
}

/// At 390 px the header wraps like the mockup (Tray-390) minus its app-switcher row: brand,
/// mode switcher (indented 12 px more) and the actions each on their own rows, 171 px in all
/// (the mockup's 227 less the 40 px switcher and its 16 px gap); the tab row follows it in the
/// bench column.
#[test]
fn phone_header_wraps_like_the_mockup() {
    let h = app().at(Route::Tray).phone().build();
    let brand = get(&h, "Automata Playground", Role::Link).rect();
    let playground = get(&h, "Playground", Role::Link).rect();
    let advanced = get(&h, "Advanced", Role::Link).rect();
    let run = get(&h, "Run", Role::Button).rect();
    let share = get(&h, "Share", Role::Button).rect();
    assert!(brand.center().y < playground.center().y);
    assert!((playground.center().y - advanced.center().y).abs() < 0.5);
    assert!(playground.center().y < run.center().y);
    assert!(run.center().y < share.center().y, "Share wraps under Run");
    for r in [brand, playground, advanced, run, share] {
        assert!(r.left() >= -0.5 && r.right() <= PHONE_W + 0.5, "{r:?}");
    }
    // the mockup's pixel rows: modes well 42–82 (pills 46–78), then (switcher row removed)
    // actions 98–130 and 138–170 (hit areas extend 6 px around 32 px controls)
    assert!(
        (playground.top() + 6.0 - 46.0).abs() <= 1.0,
        "{playground:?}"
    );
    assert!((run.top() + 6.0 - 98.0).abs() <= 1.0, "{run:?}");
    assert!((share.top() + 6.0 - 138.0).abs() <= 1.0, "{share:?}");
    assert_eq!(h.state().header_height(), 171.0);
}

/// At 390 px the columns stack at the mockup's panel height and the page grows to the
/// reference's full height less the 56 px app-switcher row the header leaves out.
#[test]
fn phone_columns_stack_to_the_reference_height() {
    for (r, height) in [
        (Route::Tray, 2759.0 - 56.0),
        (Route::Loom, 1915.0 - 56.0),
        (Route::Terrarium, 1875.0 - 56.0),
        (Route::RuleLab, 2899.0 - 56.0),
    ] {
        let h = app().at(r).phone().build();
        assert_eq!(h.state().content_height(), height, "{r:?}");
    }
}
