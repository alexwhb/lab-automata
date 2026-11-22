//! The bench stage: the tilted tray (TRAY-1/2), pointer drawing through the perspective
//! (TRAY-3), the keyboard cursor (TRAY-10), the pinned chips and the cell-age legend.

use super::{State, Stroke, Tool, cell_visual};
use crate::ctx::Ctx;
use crate::widgets::Chip;
use crate::widgets::text::{css_line_height, galley, text_top};
use ap_bench::TrayScene;
use ap_bench::tray::CONTAINER;
use ap_engine::presets::{TRAY_BOUNDARY, TRAY_H, TRAY_W};
use ap_theme as t;
use eframe::egui::{
    Align2, CornerRadius, EventFilter, Key, PointerButton, Rect, Sense, Stroke as EStroke,
    StrokeKind, Ui, WidgetInfo, accesskit::Role, epaint::Shadow, pos2, vec2,
};

/// Chips sit 16 px from the stage's sides and 14 px from its top / bottom.
const CHIP_X: f32 = 16.0;
const CHIP_Y: f32 = 14.0;

impl State {
    /// Draws and drives the stage (`ui` is the `.graph` stage, already painted).
    pub(super) fn stage(&mut self, ctx: &mut Ctx, ui: &mut Ui) {
        let stage = ui.max_rect();
        let label_left = format!(
            "PETRI TRAY · {TRAY_W} × {TRAY_H} · {}",
            TRAY_BOUNDARY.label().to_uppercase()
        );
        let code = self.model.rule().code();
        let tilt = self.tilt.value();

        // the scene's placement: the mockup's 840 × 800 box at the top of the stage, zoomed
        // to fit narrow stages and centred vertically when there's room to spare
        let probe_cell = |_: usize, _: usize| ap_bench::CellVisual::Empty;
        let zoom = TrayScene::new(TRAY_W, TRAY_H, &probe_cell).fit_zoom(stage.size());
        let spare = (stage.height() - CONTAINER.y * zoom) / 2.0;
        let scene_rect =
            Rect::from_min_max(pos2(stage.left(), stage.top() + spare.max(0.0)), stage.max);

        // ---- input, hit-tested against this frame's geometry (before any edit)
        let id = ui.make_persistent_id("tray-grid");
        let resp = ui.interact(stage, id, Sense::click_and_drag());
        let geo = {
            let grid = self.model.grid();
            let trails = self.trails;
            let cell = |c: usize, r: usize| cell_visual(grid, trails, c, r);
            let mut scene = TrayScene::new(TRAY_W, TRAY_H, &cell);
            scene.tilt = tilt;
            scene.zoom = zoom;
            scene.geometry(scene_rect)
        };
        let pointer = ui.input(|i| i.pointer.interact_pos());
        if resp.is_pointer_button_down_on() {
            match self.stroke {
                Stroke::Idle => {
                    self.stroke = Stroke::Inert;
                    if let Some((c, r)) = pointer.and_then(|p| geo.cell_at(p)) {
                        match self.current_tool() {
                            Tool::Stamp => self.apply_tool(c, r),
                            tool => {
                                let value = tool == Tool::Draw && !self.model.grid().is_alive(c, r);
                                self.model.set_cell(c, r, value);
                                self.stroke = Stroke::Paint {
                                    value,
                                    last: (c, r),
                                };
                            }
                        }
                    }
                }
                Stroke::Paint { value, last } => {
                    if let Some(cell) = pointer.and_then(|p| geo.cell_near(p))
                        && cell != last
                    {
                        self.model.paint_line(last, cell, value);
                        self.stroke = Stroke::Paint { value, last: cell };
                    }
                }
                Stroke::Inert => {}
            }
        } else {
            self.stroke = Stroke::Idle;
        }

        // keyboard (TRAY-10): arrows move the cursor, Space / Enter apply the tool
        self.has_focus = resp.has_focus();
        if self.has_focus {
            ui.memory_mut(|m| {
                m.set_focus_lock_filter(
                    id,
                    EventFilter {
                        horizontal_arrows: true,
                        vertical_arrows: true,
                        ..Default::default()
                    },
                )
            });
            let (dx, dy) = ui.input(|i| {
                let k = |key| i.num_presses(key) as i64;
                (
                    k(Key::ArrowRight) - k(Key::ArrowLeft),
                    k(Key::ArrowDown) - k(Key::ArrowUp),
                )
            });
            let (c, r) = self.cursor;
            self.cursor = (
                (c as i64 + dx).rem_euclid(TRAY_W as i64) as usize,
                (r as i64 + dy).rem_euclid(TRAY_H as i64) as usize,
            );
        }
        // a click that isn't the pointer's (Space / Enter while focused, AccessKit) applies
        // the tool at the cursor; pointer presses were handled above
        if resp.clicked() && !resp.clicked_by(PointerButton::Primary) {
            let (c, r) = self.cursor;
            self.apply_tool(c, r);
        }
        if resp.gained_focus() {
            resp.scroll_to_me(None);
        }

        // ---- paint with the edited grid
        let hover = resp.hover_pos();
        let geo = {
            let grid = self.model.grid();
            let trails = self.trails;
            let cell = |c: usize, r: usize| cell_visual(grid, trails, c, r);
            let mut scene = TrayScene::new(TRAY_W, TRAY_H, &cell);
            scene.tilt = tilt;
            scene.zoom = zoom;
            scene.label_left = &label_left;
            scene.label_right = &code;
            scene.cursor = self.has_focus.then_some(self.cursor);
            let probe = scene.geometry(scene_rect);
            scene.hover = hover.and_then(|p| probe.cell_at(p));
            scene.paint(ui.painter(), scene_rect)
        };
        self.geometry = Some(geo);

        // AccessKit: the tray is a focusable grid with the A11Y-4 description; the engraved
        // labels and the tool are repeated in its description (A11Y-2)
        let desc = self.description();
        resp.widget_info(|| WidgetInfo::labeled(Role::Grid, true, &desc));
        let extra = format!(
            "{label_left} · rule {code}. {}. Arrow keys move the cell cursor, Space or Enter applies the tool.",
            self.tool_hint()
        );
        ui.ctx()
            .accesskit_node_builder(id, |b| b.set_description(extra));

        // ---- chips and legend
        let pop = self.model.grid().population();
        Chip::new(format!("gen {:04} · ", self.model.generation()))
            .part(format!("{pop} alive"), t::INK)
            .show_at(
                ui,
                pos2(stage.left() + CHIP_X, stage.top() + CHIP_Y),
                Align2::LEFT_TOP,
            );
        Chip::default().part(self.tool_hint(), t::TEXT_DIM).show_at(
            ui,
            pos2(stage.right() - CHIP_X, stage.top() + CHIP_Y),
            Align2::RIGHT_TOP,
        );
        legend(ui, stage);
        let _ = ctx;
    }
}

/// The cell-age legend pinned bottom-left: `padding: 7px 12px; gap: 14px; radius 8`, 10 px
/// swatches (radius 3, 6 px before the word; the newborn one glows), 12 px `#D9D9D9`.
fn legend(ui: &Ui, stage: Rect) {
    let items = [
        ("newborn", t::NEWBORN.face, true),
        ("alive", t::ALIVE.face, false),
        ("elder", t::ELDER.face, false),
        ("ghost trail", t::TRAIL[2], false),
    ];
    let font = lab_theme::font::body(12.0);
    let line_h = css_line_height(ui, &font);
    let galleys: Vec<_> = items
        .iter()
        .map(|(s, _, _)| galley(ui, *s, font.clone(), t::TEXT_DIM))
        .collect();
    // flex-wrap within the stage (16 px margins)
    let max_w = (stage.width() - 2.0 * CHIP_X - 26.0).max(40.0);
    let item_w = |i: usize| 10.0 + 6.0 + galleys[i].size().x;
    let mut lines: Vec<Vec<usize>> = vec![Vec::new()];
    let mut x = 0.0;
    for i in 0..items.len() {
        let w = item_w(i);
        let line = lines.last_mut().expect("a line");
        if !line.is_empty() && x + 14.0 + w > max_w {
            lines.push(vec![i]);
            x = w;
        } else {
            x += if line.is_empty() { w } else { 14.0 + w };
            line.push(i);
        }
    }
    let row_h = line_h.max(10.0);
    let content_w = lines
        .iter()
        .map(|l| l.iter().map(|&i| item_w(i)).sum::<f32>() + 14.0 * (l.len() - 1) as f32)
        .fold(0.0, f32::max);
    let content_h = row_h * lines.len() as f32 + 14.0 * (lines.len() - 1) as f32;
    let size = vec2(content_w + 24.0 + 2.0, content_h + 14.0 + 2.0);
    let rect =
        Align2::LEFT_BOTTOM.anchor_size(pos2(stage.left() + CHIP_X, stage.bottom() - CHIP_Y), size);
    let painter = ui.painter();
    painter.rect(
        rect,
        CornerRadius::same(t::radius::WELL),
        t::CHIP,
        EStroke::new(1.0, t::CHIP_BORDER),
        StrokeKind::Inside,
    );
    let mut y = rect.top() + 8.0;
    for line in &lines {
        let mut x = rect.left() + 13.0;
        let cy = y + row_h / 2.0;
        for &i in line {
            let (label, color, glow) = items[i];
            let sw = Rect::from_min_size(pos2(x, cy - 5.0), vec2(10.0, 10.0));
            if glow {
                // box-shadow: 0 0 8px rgba(255,170,232,.8)
                let shadow = Shadow {
                    offset: [0, 0],
                    blur: 8,
                    spread: 0,
                    color: t::NEWBORN.glow.unwrap_or(color),
                };
                painter.add(shadow.as_shape(sw, CornerRadius::same(3)));
            }
            painter.rect_filled(sw, CornerRadius::same(3), color);
            let g = galleys[i].clone();
            let w = g.size().x;
            painter.galley(pos2(sw.right() + 6.0, text_top(cy, &g)), g, t::TEXT_DIM);
            let r = ui.interact(
                Rect::from_min_size(sw.min, vec2(16.0 + w, 10.0)),
                ui.scope_id().with(("legend", i)),
                Sense::hover(),
            );
            r.widget_info(|| WidgetInfo::labeled(Role::Label, true, label));
            x += item_w(i) + 14.0;
        }
        y += row_h + 14.0;
    }
}
