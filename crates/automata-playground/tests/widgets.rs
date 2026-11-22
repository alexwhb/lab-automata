//! The widget kit, driven through the gallery (`widgets::gallery`) via AccessKit: each
//! widget's states (pressed, selected, disabled), sizes and hit areas, keyboard and AccessKit
//! actions, and a snapshot of the whole gallery (`tests/snapshots/widget-gallery.png`;
//! `UPDATE_SNAPSHOTS=1` refreshes it).

mod common;

use automata_playground::widgets::gallery::Gallery;
use eframe::egui::{self, accesskit::Role, accesskit::Toggled};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};

type G = Harness<'static, Gallery>;

fn gallery() -> G {
    let mut h = Harness::builder()
        .with_size(egui::vec2(1440.0, 900.0))
        .with_pixels_per_point(1.0)
        .wgpu()
        .with_context_setup(ap_theme::install)
        .build_ui_state(
            |ui, g: &mut Gallery| {
                // the whole window, not kittest's padded test frame
                let screen = ui.ctx().content_rect();
                let mut full = ui.new_child(egui::UiBuilder::new().max_rect(screen));
                full.set_clip_rect(screen);
                g.ui(&mut full);
            },
            Gallery::default(),
        );
    h.run_steps(4);
    h
}

fn click(h: &mut G, label: &str, role: Role) {
    h.get_by_role_and_label(role, label).click();
    h.run_steps(3);
}

fn pressed(h: &G, label: &str) -> Option<bool> {
    h.get_by_role_and_label(Role::Button, label)
        .accesskit_node()
        .toggled()
        .map(|t| t == Toggled::True)
}

fn rect(h: &G, label: &str) -> egui::Rect {
    h.get_by_label(label).rect()
}

#[test]
fn buttons_click_and_report_disabled() {
    let mut h = gallery();
    for l in ["Pause", "Step", "Restart", "Random soup", "Breed brood 2"] {
        click(&mut h, l, Role::Button);
        assert_eq!(h.state().clicks.get(l), Some(&1), "{l}");
    }
    click(&mut h, "Plant in the Tray", Role::Link);
    assert_eq!(h.state().clicks.get("Plant in the Tray"), Some(&1));
    let dis = h.get_by_label("Disabled");
    assert!(dis.accesskit_node().is_disabled());
    dis.click();
    h.run_steps(2);
    assert_eq!(h.state().clicks.get("Disabled"), None);
    // .big is 44 px tall and spans the column; ghosts are drawn 32 px with a 44 px target
    assert_eq!(rect(&h, "Breed brood 2").height(), 44.0);
    assert_eq!(rect(&h, "Step").height(), 44.0);
    // the counter is text
    h.get_by_role_and_label(Role::Label, "gen 0030");
}

#[test]
fn segmented_groups_select_and_skip_disabled() {
    let mut h = gallery();
    assert_eq!(pressed(&h, "Draw"), Some(true));
    click(&mut h, "Stamp", Role::Button);
    assert_eq!(h.state().tool, 2);
    assert_eq!(pressed(&h, "Stamp"), Some(true));
    assert_eq!(pressed(&h, "Draw"), Some(false));
    h.get_by_role_and_label(Role::Group, "Tool");

    // `flex: 1` minis share the width
    let (a, b) = (rect(&h, "Single cell"), rect(&h, "Random"));
    assert!((a.width() - b.width()).abs() < 0.5);
    click(&mut h, "Random", Role::Button);
    assert_eq!(h.state().start, 1);

    // disabled family entries can't be picked and say so
    let generations = h.get_by_label("Generations");
    assert!(generations.accesskit_node().is_disabled());
    generations.click();
    h.run_steps(2);
    assert_eq!(h.state().family, 0);
    // they wrap onto a second row in a narrow well (Rule lab "Family")
    assert!(rect(&h, "Larger than Life").top() > rect(&h, "Life-like").bottom() - 10.0);
}

#[test]
fn toggles_and_keep_flip() {
    let mut h = gallery();
    assert_eq!(pressed(&h, "Ghost trails"), Some(true));
    click(&mut h, "Ghost trails", Role::Button);
    assert_eq!(pressed(&h, "Ghost trails"), Some(false));
    assert_eq!(pressed(&h, "Keep jar 1"), Some(false));
    click(&mut h, "Keep jar 1", Role::Button);
    assert!(h.state().kept);
    assert_eq!(pressed(&h, "Keep jar 1"), Some(true));
    // the pill is 30 px drawn, 44 px target
    assert_eq!(rect(&h, "Keep jar 1").height(), 44.0);
}

#[test]
fn bits_toggle_and_tile_their_targets() {
    let mut h = gallery();
    assert_eq!(pressed(&h, "Born with 3"), Some(true));
    assert_eq!(pressed(&h, "Born with 2"), Some(false));
    click(&mut h, "Born with 2", Role::Button);
    assert_eq!(h.state().born, (1 << 2) | (1 << 3));
    assert_eq!(pressed(&h, "Born with 2"), Some(true));
    // Tray bits: 28 × 30 drawn 4 px apart → inner targets span the 32 px pitch, 44 tall
    let r4 = rect(&h, "Born with 4");
    assert_eq!((r4.width(), r4.height()), (32.0, 44.0));
    let (r3, r5) = (rect(&h, "Born with 3"), rect(&h, "Born with 5"));
    assert!(r3.right() <= r4.left() + 0.01 && r4.right() <= r5.left() + 0.01);
    // Rule lab bits wrap in a narrow card
    assert!(
        rect(&h, "Survives with 12").center().y > rect(&h, "Survives with 0").center().y + 20.0
    );
    click(&mut h, "Survives with 12", Role::Button);
    assert_eq!(h.state().lab_bits & (1 << 12), 1 << 12);
}

#[test]
fn neighbourhood_grid() {
    let mut h = gallery();
    assert_eq!(pressed(&h, "Neighbour -1, -1"), Some(true));
    assert_eq!(pressed(&h, "Neighbour 2, 2"), Some(false));
    click(&mut h, "Neighbour 2, 2", Role::Button);
    assert_eq!(pressed(&h, "Neighbour 2, 2"), Some(true));
    let me = h.get_by_label("This cell");
    assert!(me.accesskit_node().is_disabled());
    h.get_by_role_and_label(Role::Group, "Neighbourhood cells");
    // neighbours tile: no target covers another's centre
    let cells: Vec<egui::Rect> = h
        .get_by_role_and_label(Role::Group, "Neighbourhood cells")
        .children_recursive()
        .filter(|n| n.accesskit_node().role() == Role::Button)
        .map(|n| n.rect())
        .collect();
    assert_eq!(cells.len(), 25);
    for (i, a) in cells.iter().enumerate() {
        for (j, b) in cells.iter().enumerate() {
            if i != j {
                assert!(!a.intersect(*b).is_positive(), "cells {i} and {j} overlap");
            }
        }
    }
}

#[test]
fn list_tiles_select() {
    let mut h = gallery();
    assert_eq!(pressed(&h, "Life"), Some(true));
    click(&mut h, "HighLife", Role::Button);
    assert_eq!(h.state().cart, 1);
    assert_eq!(pressed(&h, "HighLife"), Some(true));
    assert_eq!(pressed(&h, "Life"), Some(false));
    // the long name wraps: Day & Night's cartridge is taller (the mockup's 66 vs 50 px)
    assert!(rect(&h, "Day & Night").height() > rect(&h, "Life").height() + 10.0);
    click(&mut h, "Spaceship", Role::Button);
    assert_eq!(h.state().stamp, 1);
    assert!(rect(&h, "Glider").height() >= 78.0, "stamp min-height");
    click(&mut h, "90 Sierpiński", Role::Button);
    assert_eq!(h.state().preset, 1);
}

#[test]
fn slider_keys_and_accesskit() {
    let mut h = gallery();
    let s = h.get_by_role_and_label(Role::Slider, "Speed");
    assert_eq!(s.accesskit_node().numeric_value(), Some(10.0));
    assert_eq!(s.accesskit_node().min_numeric_value(), Some(1.0));
    assert_eq!(s.accesskit_node().max_numeric_value(), Some(30.0));
    s.focus();
    h.run_steps(2);
    h.key_press(egui::Key::ArrowRight);
    h.run_steps(2);
    assert_eq!(h.state().speed, 11.0);
    h.get_by_role_and_label(Role::Slider, "Speed")
        .increment_accesskit();
    h.run_steps(2);
    assert_eq!(h.state().speed, 12.0);
    h.get_by_role_and_label(Role::Slider, "Speed")
        .set_value_accesskit(99.0);
    h.run_steps(2);
    assert_eq!(h.state().speed, 30.0, "clamped");
    // the value row follows
    assert!(h.query_by_label("Speed: 30 gen/s").is_some());
    // 28 px drawn, 44 px target
    assert_eq!(
        h.get_by_role_and_label(Role::Slider, "Speed")
            .rect()
            .height(),
        44.0
    );
}

#[test]
fn readouts_have_text_alternatives() {
    let h = gallery();
    for l in ["Generation: 30", "Alive: 90", "Born: 27", "Died: 57"] {
        h.get_by_role_and_label(Role::Label, l);
    }
    h.get_by_role_and_label(Role::Group, "What is happening");
    h.get_by_role_and_label(Role::Label, "Churning");
    h.get_by_role_and_label(Role::Label, "Settles into a period-2 cycle at gen 388.");
    h.get_by_role_and_label(Role::Label, "gen 0030 · 90 alive");
    assert!(
        h.query_all_by_role(Role::Image).count() >= 3,
        "plots and glyphs are images"
    );
}

/// Every control in the gallery: named; targets at least 44 px or tiling their group; no
/// target covers another control's centre.
#[test]
fn every_control_is_named_and_reachable() {
    let h = gallery();
    let nodes: Vec<(String, egui::Rect, bool)> = h
        .root()
        .children_recursive()
        .filter(|n| {
            matches!(
                n.accesskit_node().role(),
                Role::Button | Role::Link | Role::Slider
            )
        })
        .map(|n| {
            let ak = n.accesskit_node();
            (ak.label().unwrap_or_default(), n.rect(), ak.is_disabled())
        })
        .collect();
    assert!(nodes.len() > 60, "{}", nodes.len());
    for (l, r, disabled) in &nodes {
        assert!(!l.is_empty(), "unnamed control at {r:?}");
        if *disabled {
            continue;
        }
        let min = if common::dense(l) { 28.0 } else { 43.99 };
        assert!(r.width() >= min && r.height() >= min, "{l:?} {r:?}");
    }
    for (i, (a, ra, _)) in nodes.iter().enumerate() {
        for (j, (b, rb, _)) in nodes.iter().enumerate() {
            if i != j {
                assert!(!ra.contains(rb.center()), "{a:?} covers {b:?}'s centre");
            }
        }
    }
}

#[test]
fn widget_gallery_snapshot() {
    let mut h = gallery();
    h.remove_cursor();
    h.run_steps(2);
    let img = h.render().expect("render");
    let dir = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR"));
    let _ = img.save(dir.join("widget-gallery.png"));
    h.snapshot("widget-gallery");
}

/// A column's `gap` works like CSS `gap` after every block widget: the next item starts
/// exactly one gap below the widget's layout box (none swallows it, none adds its own), and
/// box-shadow-like extras (the Tray bits' 3 px drop) stay out of the layout box.
#[test]
fn block_widgets_keep_the_column_gap() {
    use automata_playground::widgets::{
        BitRow, Callout, Card, NbGrid, SegGroup, SegItem, SegStyle,
    };
    const GAP: f32 = 14.0;
    let items = [SegItem::new("Torus"), SegItem::new("Dead edge")];
    let ctx = egui::Context::default();
    ap_theme::install(&ctx);
    let input = || egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(320.0, 1200.0),
        )),
        ..Default::default()
    };
    let mut measured: Vec<(&str, f32, f32)> = Vec::new();
    for _ in 0..2 {
        measured.clear();
        let mut out = ctx.run_ui(input(), |ui| {
            ui.spacing_mut().item_spacing = egui::vec2(GAP, GAP);
            let mut block =
                |name: &'static str, ui: &mut egui::Ui, add: &mut dyn FnMut(&mut egui::Ui)| {
                    let top = ui.cursor().top();
                    add(ui);
                    let bottom = ui.min_rect().bottom();
                    measured.push((name, bottom - top, ui.cursor().top() - bottom));
                };
            block("BitRow", ui, &mut |ui| {
                BitRow::new("Born with", 9, 0b1000).show(ui);
            });
            block("BitRow::lab", ui, &mut |ui| {
                BitRow::new("Survives with", 9, 0b1100).lab().show(ui);
            });
            block("NbGrid", ui, &mut |ui| {
                NbGrid::new(0).show(ui);
            });
            block("SegGroup", ui, &mut |ui| {
                SegGroup::new("Boundary", &items, Some(0))
                    .style(SegStyle::Mini { fill: true })
                    .show(ui);
            });
            block("Card::label", ui, &mut |ui| {
                Card::panel().label("Card").show(ui, |ui| ui.label("x"));
            });
            block("Callout", ui, &mut |ui| {
                Callout::new("Churning", "Births and deaths are balanced.").show(ui);
            });
        });
        out.textures_delta.clear();
    }
    for (name, height, gap) in &measured {
        assert_eq!(*gap, GAP, "{name}: gap after it");
        assert!(*height > 0.0, "{name}: empty box");
    }
    let height = |n: &str| measured.iter().find(|m| m.0 == n).map(|m| m.1).unwrap();
    assert_eq!(
        height("BitRow"),
        30.0,
        "Tray bits: 30 px, the drop is a box-shadow"
    );
    assert_eq!(height("BitRow::lab"), 26.0);
    assert_eq!(height("NbGrid"), 5.0 * 30.0 + 4.0 * 3.0);
}
