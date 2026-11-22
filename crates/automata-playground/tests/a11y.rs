//! Accessibility (A11Y-1/3) on every route at 1440 and 390 px: every interactive node has a
//! role and a name; toggles report pressed, tabs / current mode / current app report
//! selected, disabled controls report disabled; click targets reach 44 × 44 where the
//! mockup's padding allows (and tile their group where it doesn't) without ever covering
//! another control's centre; Tab walks the header in reading order and draws the focus ring.

mod common;

use automata_playground::Route;
use common::*;
use eframe::egui::{self, accesskit::Role};
use egui_kittest::kittest::{NodeT, Queryable};

/// Roles a user acts on.
const INTERACTIVE: &[Role] = &[
    Role::Button,
    Role::Link,
    Role::Tab,
    Role::CheckBox,
    Role::RadioButton,
    Role::ComboBox,
    Role::Slider,
    Role::TextInput,
    Role::MenuItem,
    Role::ListBoxOption,
];

fn check_route(h: &mut App, r: Route, w: f32) {
    goto(h, r);
    let nodes: Vec<_> = h
        .root()
        .children_recursive()
        .filter(|n| INTERACTIVE.contains(&n.accesskit_node().role()))
        .map(|n| {
            let ak = n.accesskit_node();
            (
                ak.role(),
                ak.label().unwrap_or_default(),
                n.rect(),
                ak.is_disabled(),
            )
        })
        .collect();
    assert!(
        nodes.len() >= 9,
        "{r:?} at {w}: only {} interactive nodes",
        nodes.len()
    );
    for (role, label, rect, disabled) in &nodes {
        assert!(
            !label.trim().is_empty(),
            "{r:?} at {w}: a {role:?} at {rect:?} has no accessible name"
        );
        if *disabled {
            continue;
        }
        let min = if dense(label) { 28.0 } else { 43.99 };
        assert!(
            rect.width() >= min && rect.height() >= min,
            "{r:?} at {w}: {role:?} {label:?} is {:.1} × {:.1}",
            rect.width(),
            rect.height()
        );
    }
    // a target never covers another control's centre (clicks land where you aim)
    for (i, (_, a, ra, _)) in nodes.iter().enumerate() {
        for (j, (_, b, rb, _)) in nodes.iter().enumerate() {
            if i != j {
                assert!(
                    !ra.contains(rb.center()) || ra == rb,
                    "{r:?} at {w}: {a:?}'s target covers {b:?}'s centre"
                );
            }
        }
    }
}

#[test]
fn interactive_nodes_are_named_and_big_enough_desktop() {
    let mut h = app().build();
    for r in Route::ALL {
        check_route(&mut h, r, W);
    }
}

#[test]
fn interactive_nodes_are_named_and_big_enough_phone() {
    let mut h = app().phone().build();
    for r in Route::ALL {
        check_route(&mut h, r, PHONE_W);
    }
}

/// The header's brand, modes and switcher icons are links (`<a>` in the mockup); the actions
/// are buttons; the regions are named like the mockup's `aria-label`s.
#[test]
fn roles_match_the_mockup() {
    let h = harness(Route::Tray);
    for label in [
        "Automata Playground",
        "Playground",
        "Advanced",
        "Breadbox circuit app",
        "Chem Playground",
        "Bio Playground",
        "Automata Playground (current)",
    ] {
        get(&h, label, Role::Link);
    }
    for label in [
        "Run",
        "Step",
        "Restart",
        "Share",
        "Draw",
        "Erase",
        "Stamp",
        "Random soup",
        "Clear",
    ] {
        get(&h, label, Role::Button);
    }
    get(&h, "Mode", Role::Navigation);
    get(&h, "Switch app", Role::Group);
    get(&h, "Tool", Role::Group);
    get(&h, "Benches", Role::TabList);
    get(&h, "Cartridges and stamps", Role::Region);
    get(&h, "Rule and readouts", Role::Region);
    get(&h, "gen 0000", Role::Label);
}

/// Toggle groups report `aria-pressed`, and it follows clicks.
#[test]
fn pressed_states_follow_clicks() {
    let mut h = harness(Route::Tray);
    assert_eq!(pressed(&h, "Draw"), Some(true));
    assert_eq!(pressed(&h, "Erase"), Some(false));
    click_button(&mut h, "Erase");
    assert_eq!(pressed(&h, "Draw"), Some(false));
    assert_eq!(pressed(&h, "Erase"), Some(true));
    assert_eq!(h.state().screens().tray.tool, 1);
    // plain buttons aren't toggles
    assert_eq!(pressed(&h, "Step"), None);
}

/// Disabled controls say so (Bio Playground is planned: its Open button is disabled).
#[test]
fn disabled_controls_report_disabled() {
    let h = harness(Route::Series);
    assert!(disabled(&h, "Open Bio Playground (planned, not built yet)"));
    assert!(!disabled(&h, "Open Chem Playground"));
}

/// Tab walks the header in reading order: brand, modes, the app switcher, the screen's
/// actions, then the bench tabs and the tab row's items. Enter on a focused link follows it.
#[test]
fn tab_key_walks_in_reading_order() {
    let mut h = harness(Route::Tray);
    let mut order = Vec::new();
    for _ in 0..19 {
        press(&mut h, egui::Key::Tab);
        settle(&mut h);
        order.push(focused_label(&h).unwrap_or_default());
    }
    let expected = [
        "Automata Playground",
        "Playground",
        "Advanced",
        "Breadbox circuit app",
        "Chem Playground",
        "Bio Playground",
        "Automata Playground (current)",
        "Run",
        "Step",
        "Restart",
        "Share",
        "Tray · 2D",
        "Loom · 1D",
        "Terrarium · breed",
        "Draw",
        "Erase",
        "Stamp",
        "Random soup",
        "Clear",
    ];
    assert_eq!(order, expected);

    let mut h = harness(Route::Tray);
    for _ in 0..3 {
        press(&mut h, egui::Key::Tab);
    }
    settle(&mut h);
    assert_eq!(focused_label(&h).as_deref(), Some("Advanced"));
    press(&mut h, egui::Key::Enter);
    settle(&mut h);
    assert_eq!(route(&h), Route::RuleLab, "Enter on Advanced");
}

/// The accent focus ring appears around the focused control (and only then).
#[test]
fn focus_ring_is_visible() {
    let mut h = harness(Route::Loom);
    let step = get(&h, "New thread", Role::Button).rect();
    let area = step.shrink2(egui::vec2(0.0, 4.0)).expand(4.0);
    let accent = ap_theme::ACCENT.to_srgba_unmultiplied();
    let count = |img: &image::RgbaImage| {
        let mut n = 0;
        for y in area.top().max(0.0) as u32..area.bottom() as u32 {
            for x in area.left().max(0.0) as u32..area.right() as u32 {
                let p = img.get_pixel(x, y).0;
                if (0..3).all(|i| (p[i] as i32 - accent[i] as i32).abs() <= 30) {
                    n += 1;
                }
            }
        }
        n
    };
    let before = count(&render(&mut h));
    h.get_by_role_and_label(Role::Button, "New thread").focus();
    settle(&mut h);
    let after = count(&render(&mut h));
    assert_eq!(before, 0, "no ring before focus");
    assert!(after > 150, "focus ring not drawn: {after} accent px");
}

/// Toasts are polite status messages.
#[test]
fn toasts_are_status_nodes() {
    let mut h = harness(Route::Tray);
    click_button(&mut h, "Share");
    let t = toasts(&h);
    assert_eq!(t.len(), 1);
    get(&h, &t[0], Role::Status);
}
