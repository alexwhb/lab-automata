//! Snapshot renders of every scene with egui_kittest (wgpu), at the mockup's 1440 px stage
//! sizes: `tests/snapshots/*.png` (`UPDATE_SNAPSHOTS=1 cargo test -p ap-bench --test
//! snapshots` to refresh). Compare them side by side with
//! `docs/design/screenshots/<Screen>-1440.png` (the bench areas).

mod common;

use ap_bench::loom::{KeyVisual, LoomScene, paint_key};
use ap_bench::{JarScene, TopDownGrid, TrayScene, jar_bob};
use egui::{Rect, Ui, pos2, vec2};
use egui_kittest::Harness;

/// The Tray's / Loom's `.graph` stage at 1440 × 900 (main column minus the tab row).
const TRAY_STAGE: egui::Vec2 = egui::Vec2::new(872.0, 798.0);
const LOOM_STAGE: egui::Vec2 = egui::Vec2::new(1120.0, 798.0);

fn harness(size: egui::Vec2, paint: impl FnMut(&mut Ui) + 'static) -> Harness<'static> {
    let mut paint = paint;
    let mut h = Harness::builder()
        .with_size(size)
        .with_pixels_per_point(1.0)
        .wgpu()
        .with_context_setup(ap_theme::install)
        .build_ui(move |ui| {
            // paint over the whole window, not inside kittest's test frame
            let screen = ui.ctx().content_rect();
            let mut full = ui.new_child(egui::UiBuilder::new().max_rect(screen));
            full.set_clip_rect(screen);
            paint(&mut full);
        });
    h.run_steps(2);
    h
}

fn tray_snapshot(
    name: &str,
    tilt: f32,
    hover: Option<(usize, usize)>,
    cursor: Option<(usize, usize)>,
) {
    let garden = common::Garden::mockup().run(30).clone();
    assert_eq!((garden.alive(), garden.generation), (90, 30)); // SPEC REF-2's gen 30
    let mut h = harness(TRAY_STAGE, move |ui| {
        let rect = ui.max_rect();
        ap_bench::stage(ui.painter(), rect);
        let cell = |c, r| garden.visual(c, r);
        let mut scene = TrayScene::new(40, 26, &cell);
        scene.tilt = tilt;
        scene.hover = hover;
        scene.cursor = cursor;
        scene.label_left = "PETRI TRAY · 40 × 26 · TORUS";
        scene.label_right = "B3/S23";
        scene.paint(ui.painter(), rect);
    });
    h.snapshot(name);
}

#[test]
fn tray_tilted() {
    tray_snapshot("tray_tilted", 1.0, None, None);
}

#[test]
fn tray_flat_with_hover_and_cursor() {
    tray_snapshot("tray_flat", 0.0, Some((12, 6)), Some((30, 20)));
}

#[test]
fn tray_mid_transition() {
    tray_snapshot("tray_half", ap_bench::ease(0.5), None, None);
}

#[test]
fn tray_narrow() {
    let garden = common::Garden::mockup().run(30).clone();
    let mut h = harness(vec2(390.0, 420.0), move |ui| {
        let rect = ui.max_rect();
        ap_bench::stage(ui.painter(), rect);
        let cell = |c, r| garden.visual(c, r);
        let mut scene = TrayScene::new(40, 26, &cell);
        scene.zoom = scene.fit_zoom(rect.size());
        scene.label_left = "PETRI TRAY · 40 × 26 · TORUS";
        scene.label_right = "B3/S23";
        scene.paint(ui.painter(), rect);
    });
    h.snapshot("tray_390");
}

#[test]
fn loom() {
    let rows_owned = common::weave(30, 47);
    let mut h = harness(LOOM_STAGE, move |ui| {
        let rect = ui.max_rect();
        ap_bench::stage(ui.painter(), rect);
        let rows: Vec<&[bool]> = rows_owned.iter().map(|r| r.as_slice()).collect();
        let mut scene = LoomScene::new(&rows);
        scene.head_label = "PRINT HEAD · 121 CELLS · WRAPS";
        scene.rule_label = "Rule 30";
        scene.progress = 47.0 / 72.0;
        scene.spool_angle = 0.6;
        let geo = scene.paint(ui.painter(), rect);
        for (i, r) in geo.keys.iter().enumerate() {
            let mut key = KeyVisual::for_rule(30, 7 - i as u8);
            key.hovered = i == 1;
            key.focused = i == 6;
            paint_key(ui.painter(), *r, key);
        }
        ui.painter().text(
            geo.caption,
            egui::Align2::LEFT_TOP,
            "These 8 keys are the rule. Read the outputs as binary to get its number.",
            ap_theme::font::body(12.0),
            ap_theme::TEXT_MUTED,
        );
    });
    h.snapshot("loom");
}

#[test]
fn terrarium_jars() {
    let jars: Vec<common::Garden> = (0..6).map(|i| common::jar(i as u64 + 1, 40)).collect();
    // the mockup's card grid: 3 columns of 345 px cards, 18 px gaps, 22 px padding; the jar
    // area sits 15 px inside each card
    let mut h = harness(vec2(1120.0, 720.0), move |ui| {
        let rect = ui.max_rect();
        ap_bench::stage(ui.painter(), rect);
        for (i, g) in jars.iter().enumerate() {
            let (col, row) = (i % 3, i / 3);
            let card = Rect::from_min_size(
                rect.min
                    + vec2(
                        22.0 + col as f32 * (345.0 + 18.0),
                        22.0 + row as f32 * (330.0 + 18.0),
                    ),
                vec2(345.0, 330.0),
            );
            ui.painter().rect(
                card,
                14,
                ap_theme::JAR_CARD,
                egui::Stroke::new(1.0, ap_theme::JAR_CARD_BORDER),
                egui::StrokeKind::Inside,
            );
            let area = Rect::from_min_size(
                pos2(card.center().x - 136.0, card.top() + 15.0),
                JarScene::FOOTPRINT,
            );
            let cell = |x: usize, y: usize| g.age[y * 32 + x] > 0;
            let mut jar = JarScene::new(&cell, ap_theme::JAR_INKS[i]);
            jar.bob = jar_bob(0.9, i);
            jar.paint(ui.painter(), area);
        }
    });
    h.snapshot("terrarium");
}

#[test]
fn rule_lab_top_down() {
    let (prev, now) = common::rule_lab(120);
    let mut h = harness(vec2(800.0, 512.0), move |ui| {
        let rect = ui.max_rect();
        lab_ui_paper(ui, rect);
        let cell = |x, y| now.top_cell(&prev, x, y);
        let grid = TopDownGrid::new(64, 40, &cell);
        grid.paint(
            ui.painter(),
            rect.min + vec2(16.0, 16.0),
            rect.width() - 32.0,
        );
    });
    h.snapshot("rule_lab");
}

#[test]
fn rule_lab_top_down_narrow() {
    let (prev, now) = common::rule_lab(120);
    let mut h = harness(vec2(390.0, 280.0), move |ui| {
        let rect = ui.max_rect();
        lab_ui_paper(ui, rect);
        let cell = |x, y| now.top_cell(&prev, x, y);
        let grid = TopDownGrid::new(64, 40, &cell);
        grid.paint(
            ui.painter(),
            rect.min + vec2(16.0, 16.0),
            rect.width() - 32.0,
        );
    });
    h.snapshot("rule_lab_390");
}

/// The window's `.paper` ground behind the Rule lab's well.
fn lab_ui_paper(ui: &Ui, rect: Rect) {
    ui.painter().rect_filled(rect, 0, ap_theme::PAPER.base);
}
