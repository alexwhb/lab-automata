//! Every bench scene, interactive: `cargo run -p ap-bench --example scenes`.
//!
//! - Tray: hover cells (through the perspective), click / drag to draw, arrow keys move the
//!   cell cursor and Enter toggles, Space runs / pauses, T toggles the top-down view.
//! - Loom: weaves rule 30; click a key to flip that bit of the rule.
//! - Terrarium: six bobbing jars.
//! - Rule lab: scrub the R-pentomino's 400 generations.
//!
//! The Life stepper here is demo code from the tests; the app uses `ap-engine`.

#[path = "../tests/common/mod.rs"]
mod common;

use ap_bench::anim::{Transition, ease, jar_bob, spool_angle};
use ap_bench::loom::{KeyVisual, LoomScene, paint_key};
use ap_bench::{JarScene, TopDownGrid, TrayScene};
use eframe::egui;
use egui::{Key, Rect, Sense, pos2, vec2};

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1180.0, 860.0])
            .with_title("ap-bench scenes"),
        ..Default::default()
    };
    eframe::run_native(
        "ap-bench scenes",
        options,
        Box::new(|cc| {
            ap_theme::install(&cc.egui_ctx);
            Ok(Box::new(Scenes::default()))
        }),
    )
}

#[derive(PartialEq, Clone, Copy)]
enum Tab {
    Tray,
    Loom,
    Terrarium,
    RuleLab,
}

struct Scenes {
    tab: Tab,
    time: f64,
    // tray
    garden: common::Garden,
    running: bool,
    acc: f32,
    tilt: Transition,
    cursor: (usize, usize),
    paint_value: Option<bool>,
    // loom
    rule: u8,
    rows: Vec<Vec<bool>>,
    weave_acc: f32,
    weave_time: f64,
    // terrarium
    jars: Vec<common::Garden>,
    jar_acc: f32,
    // rule lab
    scrub: usize,
    frames: (common::Garden, common::Garden),
}

impl Default for Scenes {
    fn default() -> Self {
        Scenes {
            tab: Tab::Tray,
            time: 0.0,
            garden: common::Garden::mockup(),
            running: true,
            acc: 0.0,
            tilt: Transition::new(1.0, 0.4, ease),
            cursor: (20, 13),
            paint_value: None,
            rule: 30,
            rows: common::weave(30, 1),
            weave_acc: 0.0,
            weave_time: 0.0,
            jars: (0..6).map(|i| common::jar(i + 1, 0)).collect(),
            jar_acc: 0.0,
            scrub: 120,
            frames: common::rule_lab(120),
        }
    }
}

impl eframe::App for Scenes {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let dt = ctx.input(|i| i.stable_dt).min(0.1);
        self.time += dt as f64;
        egui::Panel::top("tabs").show(ui, |ui| {
            ui.horizontal(|ui| {
                for (t, name) in [
                    (Tab::Tray, "Tray · 2D"),
                    (Tab::Loom, "Loom · 1D"),
                    (Tab::Terrarium, "Terrarium"),
                    (Tab::RuleLab, "Rule lab"),
                ] {
                    ui.selectable_value(&mut self.tab, t, name);
                }
                ui.separator();
                match self.tab {
                    Tab::Tray => {
                        if ui
                            .button(if self.running { "Pause" } else { "Run" })
                            .clicked()
                        {
                            self.running = !self.running;
                        }
                        let flat = self.tilt.target() < 0.5;
                        if ui.selectable_label(flat, "Top-down view (T)").clicked() {
                            self.tilt.set_target(if flat { 1.0 } else { 0.0 });
                        }
                        ui.label(format!(
                            "gen {:04} · {} alive",
                            self.garden.generation,
                            self.garden.alive()
                        ));
                    }
                    Tab::RuleLab => {
                        let old = self.scrub;
                        ui.add(egui::Slider::new(&mut self.scrub, 0..=400).text("gen"));
                        if old != self.scrub {
                            self.frames = common::rule_lab(self.scrub);
                        }
                    }
                    _ => {}
                }
            });
        });
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ui, |ui| {
                let rect = ui.max_rect();
                ap_bench::stage(ui.painter(), rect);
                match self.tab {
                    Tab::Tray => self.tray(ui, rect, dt),
                    Tab::Loom => self.loom(ui, rect, dt),
                    Tab::Terrarium => self.terrarium(ui, rect, dt),
                    Tab::RuleLab => self.rule_lab(ui, rect),
                }
            });
        ctx.request_repaint();
    }
}

impl Scenes {
    fn tray(&mut self, ui: &mut egui::Ui, rect: Rect, dt: f32) {
        if self.running {
            self.acc += dt;
            while self.acc >= 0.1 {
                self.acc -= 0.1;
                self.garden.step();
            }
        }
        self.tilt.step(dt);
        let (w, h) = (self.garden.w, self.garden.h);
        ui.input(|i| {
            let (c, r) = &mut self.cursor;
            if i.key_pressed(Key::ArrowLeft) {
                *c = (*c + w - 1) % w;
            }
            if i.key_pressed(Key::ArrowRight) {
                *c = (*c + 1) % w;
            }
            if i.key_pressed(Key::ArrowUp) {
                *r = (*r + h - 1) % h;
            }
            if i.key_pressed(Key::ArrowDown) {
                *r = (*r + 1) % h;
            }
        });
        if ui.input(|i| i.key_pressed(Key::Enter)) {
            let (c, r) = self.cursor;
            self.set_cell(c, r, self.garden.age[r * w + c] == 0);
        }
        if ui.input(|i| i.key_pressed(Key::Space)) {
            self.running = !self.running;
        }
        if ui.input(|i| i.key_pressed(Key::T)) {
            let t = if self.tilt.target() < 0.5 { 1.0 } else { 0.0 };
            self.tilt.set_target(t);
        }

        let resp = ui.allocate_rect(rect, Sense::click_and_drag());
        let garden = self.garden.clone();
        let cell = |c, r| garden.visual(c, r);
        let mut scene = TrayScene::new(w, h, &cell);
        scene.tilt = self.tilt.value();
        scene.zoom = scene.fit_zoom(rect.size());
        scene.label_left = "PETRI TRAY · 40 × 26 · TORUS";
        scene.label_right = "B3/S23";
        scene.cursor = Some(self.cursor);
        let geo = scene.geometry(rect);
        let pointer = resp.interact_pointer_pos().or(resp.hover_pos());
        scene.hover = pointer.and_then(|p| geo.cell_at(p));
        if resp.drag_started() || resp.clicked() {
            if let Some((c, r)) = pointer.and_then(|p| geo.cell_at(p)) {
                let v = garden.age[r * w + c] == 0;
                self.paint_value = Some(v);
                self.set_cell(c, r, v);
            }
        } else if resp.dragged()
            && let (Some(v), Some((c, r))) =
                (self.paint_value, pointer.and_then(|p| geo.cell_near(p)))
        {
            self.set_cell(c, r, v);
        }
        if resp.drag_stopped() {
            self.paint_value = None;
        }
        scene.paint(ui.painter(), rect);
    }

    fn set_cell(&mut self, c: usize, r: usize, alive: bool) {
        let i = r * self.garden.w + c;
        self.garden.age[i] = if alive { 1 } else { 0 };
    }

    fn loom(&mut self, ui: &mut egui::Ui, rect: Rect, dt: f32) {
        self.weave_acc += dt;
        self.weave_time += dt as f64;
        while self.weave_acc >= 1.0 / 14.0 {
            self.weave_acc -= 1.0 / 14.0;
            let last = self.rows.last().cloned().unwrap_or_default();
            let n = last.len();
            let row = (0..n)
                .map(|i| {
                    let p = (last[(i + n - 1) % n] as u8) << 2
                        | (last[i] as u8) << 1
                        | last[(i + 1) % n] as u8;
                    (self.rule >> p) & 1 == 1
                })
                .collect();
            self.rows.push(row);
            if self.rows.len() > 72 {
                self.rows.remove(0);
            }
        }
        let rows: Vec<&[bool]> = self.rows.iter().map(|r| r.as_slice()).collect();
        let mut scene = LoomScene::new(&rows);
        scene.zoom = LoomScene::fit_zoom(rect.size());
        scene.head_label = "PRINT HEAD · 121 CELLS · WRAPS";
        let label = format!("Rule {}", self.rule);
        scene.rule_label = &label;
        scene.progress = rows.len() as f32 / 72.0;
        scene.spool_angle = spool_angle(self.weave_time);
        let geo = scene.paint(ui.painter(), rect);
        let mut flip = None;
        for (i, r) in geo.keys.iter().enumerate() {
            let p = 7 - i as u8;
            let resp = ui.interact(
                geo.key_target(i),
                ui.scope_id().with(("key", i)),
                Sense::click(),
            );
            let mut key = KeyVisual::for_rule(self.rule, p);
            key.hovered = resp.hovered();
            key.pressed = resp.is_pointer_button_down_on();
            key.focused = resp.has_focus();
            paint_key(ui.painter(), *r, key);
            if resp.clicked() {
                flip = Some(p);
            }
        }
        if let Some(p) = flip {
            self.rule ^= 1 << p;
            self.rows = common::weave(self.rule, 1);
        }
    }

    fn terrarium(&mut self, ui: &mut egui::Ui, rect: Rect, dt: f32) {
        self.jar_acc += dt;
        while self.jar_acc >= 0.14 {
            self.jar_acc -= 0.14;
            for j in &mut self.jars {
                j.step();
            }
        }
        let cols = ((rect.width() - 44.0) / 300.0).floor().clamp(1.0, 3.0) as usize;
        let card_w = (rect.width() - 44.0 - 18.0 * (cols - 1) as f32) / cols as f32;
        for (i, g) in self.jars.iter().enumerate() {
            let (col, row) = (i % cols, i / cols);
            let card = Rect::from_min_size(
                rect.min
                    + vec2(
                        22.0 + col as f32 * (card_w + 18.0),
                        22.0 + row as f32 * 300.0,
                    ),
                vec2(card_w, 282.0),
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
            jar.bob = jar_bob(self.time, i);
            jar.paint(ui.painter(), area);
        }
    }

    fn rule_lab(&mut self, ui: &mut egui::Ui, rect: Rect) {
        ui.painter().rect_filled(rect, 0, ap_theme::PAPER.base);
        let (prev, now) = &self.frames;
        let cell = |x, y| now.top_cell(prev, x, y);
        let grid = TopDownGrid::new(64, 40, &cell);
        let geo = grid.paint(
            ui.painter(),
            rect.min + vec2(16.0, 16.0),
            rect.width() - 32.0,
        );
        if let Some((x, y)) = ui.ctx().pointer_hover_pos().and_then(|p| geo.cell_at(p)) {
            ui.painter().rect_stroke(
                geo.cell_rect(x, y).expand(1.0),
                2,
                egui::Stroke::new(1.0, ap_theme::LINK),
                egui::StrokeKind::Outside,
            );
        }
    }
}
