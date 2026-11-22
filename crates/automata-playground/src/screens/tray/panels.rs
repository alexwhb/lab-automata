//! The two asides: the rules list and stamps (TRAY-4/5, TRAY-9's custom stamp), and the rule
//! panel and readouts (TRAY-6/7/8).

use super::{SPEED_RANGE, StampChoice, State, Tool};
use crate::widgets::text::{
    css_line_height, galley, galley_spaced, lbl_galley, lbl_row, paint_line,
};
use crate::widgets::{
    BitRow, Callout, Cartridge, CellGlyph, Plot, Slider, StampTile, StatTile, Toggle, lbl,
};
use ap_engine::pattern::library;
use ap_engine::presets::{CARTRIDGES, matching_cartridge};
use ap_engine::{LifeRule, mood::HISTORY_LEN};
use ap_theme as t;
use eframe::egui::{
    Color32, FontId, Rect, Response, Sense, Stroke, Ui, UiBuilder, WidgetInfo, accesskit::Role,
    pos2, vec2,
};

/// A stamp tile to draw.
struct Tile<'a> {
    name: &'a str,
    cells: &'a [(i32, i32)],
    choice: StampChoice,
}

/// The neighbourhood's count limit on the bit rows (Moore: 0–8).
const BITS: usize = 9;

impl State {
    /// The left aside: "Rules" + six cartridges, "Stamps · click to place" + the stamp grid.
    pub(super) fn rules_aside(&mut self, ui: &mut Ui) {
        lbl(ui, "Rules");
        // `flex-direction: column; gap: 6px`
        let lit = matching_cartridge(self.model.rule()).map(|c| c.id);
        let mut picked = None;
        group(ui, "Rules", 6.0, |ui| {
            for (i, c) in CARTRIDGES.iter().enumerate() {
                let code = c.rule.code();
                let body = t::CART_BODIES[i % t::CART_BODIES.len()];
                if Cartridge::new(c.name, c.note, &code, body)
                    .on(lit == Some(c.id))
                    .show(ui)
                    .clicked()
                {
                    picked = Some(c.rule);
                }
            }
        });
        if let Some(rule) = picked {
            self.set_rule(rule);
        }

        // border-top + padding-top 14, "STAMPS" … "click to place"
        rule_line(ui);
        ui.add_space(14.0 - ui.spacing().item_spacing.y);
        caption_row(ui, "Stamps", "click to place", 11.0);

        // `grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 8px`
        let stamp_on = |s: StampChoice| self.current_tool() == Tool::Stamp && self.stamp == s;
        let mut tiles: Vec<Tile> = library::STAMPS
            .iter()
            .enumerate()
            .map(|(i, s)| Tile {
                name: s.name,
                cells: &self.stamp_cells[i],
                choice: StampChoice::Library(i),
            })
            .collect();
        if let Some(c) = &self.custom {
            tiles.push(Tile {
                name: &c.name,
                cells: &c.cells,
                choice: StampChoice::Custom,
            });
        }
        let mut chosen = None;
        group(ui, "Stamps", 8.0, |ui| {
            grid2(ui, 8.0, tiles.len(), |ui, i| {
                let tile = &tiles[i];
                if StampTile::new(tile.name, tile.cells)
                    .on(stamp_on(tile.choice))
                    .show(ui)
                    .clicked()
                {
                    chosen = Some(tile.choice);
                }
            });
        });
        match chosen {
            Some(StampChoice::Library(i)) => self.pick_stamp(i),
            Some(StampChoice::Custom) => {
                self.stamp = StampChoice::Custom;
                self.set_tool(Tool::Stamp);
            }
            None => {}
        }
    }

    /// The right aside: rule, speed and look, population, what is happening.
    pub(super) fn readouts(&mut self, ui: &mut Ui) {
        let rule = *self.model.rule();
        let name = matching_cartridge(&rule).map_or("Custom rule", |c| c.name);
        let mut new_rule: Option<LifeRule> = None;

        // ---- Rule (gap 10)
        group(ui, "Rule", 10.0, |ui| {
            lbl_row(ui, "Rule", name, lab_theme::font::body(12.0), t::TEXT_MUTED);
            big_code(ui, &rule.code());
            // `flex-direction: column; gap: 6px`
            ui.scope(|ui| {
                ui.spacing_mut().item_spacing.y = 6.0;
                css_text(ui, "Born with", lab_theme::font::body(12.0), t::TEXT_DIM);
                if let Some(n) = bit_row(ui, "Born with", rule.birth().bits()) {
                    let mut r = rule;
                    r.toggle_birth(n);
                    new_rule = Some(r);
                }
                ui.add_space(4.0);
                css_text(
                    ui,
                    "Survives with",
                    lab_theme::font::body(12.0),
                    t::TEXT_DIM,
                );
                if let Some(n) = bit_row(ui, "Survives with", rule.survive().bits()) {
                    let mut r = rule;
                    r.toggle_survive(n);
                    new_rule = Some(r);
                }
            });
            neighbourhood_row(ui, rule.neighbourhood().len());
        });
        if let Some(r) = new_rule {
            self.set_rule(r);
        }

        // ---- Speed and look
        section(ui, "Speed and look", 10.0, |ui| {
            let mut speed = self.speed;
            let value = format!("{speed:.0} gen/s");
            Slider::new("Speed", &mut speed, SPEED_RANGE)
                .step(1.0)
                .value_text(value)
                .show(ui);
            self.set_speed(speed);
            let mut flip = (false, false);
            ui.scope(|ui| {
                // rows as tall as the 28 px pills (egui's default row is taller)
                ui.spacing_mut().interact_size.y = t::size::MINI;
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
                    flip.0 = Toggle::new("Ghost trails", self.trails).show(ui).clicked();
                    flip.1 = Toggle::new("Top-down view", self.flat).show(ui).clicked();
                });
            });
            if flip.0 {
                self.trails = !self.trails;
            }
            if flip.1 {
                let f = !self.flat;
                self.set_flat(f);
            }
        });

        // ---- Population
        let stats = self.model.stats();
        section(ui, "Population", 10.0, |ui| {
            lbl_row(
                ui,
                "Population",
                &format!("last {HISTORY_LEN} gens"),
                lab_theme::font::body(12.0),
                t::TEXT_MUTED,
            );
            let hist = self.model.history();
            let values: Vec<f32> = hist.values().iter().map(|&v| v as f32).collect();
            let label = format!("Population over the last {HISTORY_LEN} generations");
            let resp = Plot::sparkline(&values, HISTORY_LEN).label(&label).show(ui);
            let summary = format!(
                "{} generations, latest {}, peak {}",
                values.len(),
                hist.latest(),
                hist.values().iter().copied().max().unwrap_or(0)
            );
            ui.ctx()
                .accesskit_node_builder(resp.id, |b| b.set_value(summary));
            let tiles = [
                ("Generation", stats.generation.to_string(), t::TEXT),
                ("Alive", stats.population.to_string(), t::INK),
                ("Born", stats.born.to_string(), t::NEWBORN.face),
                ("Died", stats.died.to_string(), t::TEXT_MUTED),
            ];
            grid2(ui, 8.0, tiles.len(), |ui, i| {
                let (k, v, c) = &tiles[i];
                StatTile::new(k, v).color(*c).show(ui);
            });
        });

        // ---- What is happening
        Callout::new(stats.mood_title, stats.mood_text).show(ui);
    }
}

/// A named AccessKit group laid out top-down with `gap` (`<section aria-label>` /
/// a `display: flex; flex-direction: column` box).
fn group(ui: &mut Ui, label: &str, gap: f32, add: impl FnOnce(&mut Ui)) -> Rect {
    let origin = ui.cursor().min;
    let width = ui.available_width();
    let mut child = ui.new_child(
        UiBuilder::new()
            .max_rect(Rect::from_min_size(origin, vec2(width, f32::INFINITY)))
            .layout(eframe::egui::Layout::top_down(eframe::egui::Align::Min))
            .accessibility_role(Role::Group)
            .accessibility_label(label),
    );
    child.spacing_mut().item_spacing = vec2(gap, gap);
    add(&mut child);
    let used = Rect::from_min_size(origin, vec2(width, child.min_rect().height()));
    crate::widgets::set_bounds(&child, used);
    ui.advance_cursor_after_rect(used);
    used
}

/// A readout section: `border-top: 1px solid #242424; padding-top: 16px` over a [`group`].
fn section(ui: &mut Ui, label: &str, gap: f32, add: impl FnOnce(&mut Ui)) {
    let origin = ui.cursor().min;
    let width = ui.available_width();
    ui.painter().hline(
        origin.x..=origin.x + width,
        origin.y + 0.5,
        Stroke::new(1.0, t::BORDER),
    );
    let mut child = ui.new_child(
        UiBuilder::new()
            .max_rect(Rect::from_min_size(
                pos2(origin.x, origin.y + 17.0),
                vec2(width, f32::INFINITY),
            ))
            .layout(eframe::egui::Layout::top_down(eframe::egui::Align::Min)),
    );
    let used = group(&mut child, label, gap, add);
    ui.advance_cursor_after_rect(Rect::from_min_max(
        origin,
        pos2(origin.x + width, used.bottom()),
    ));
}

/// The aside's separator line (`border-top: 1px solid #242424`) across the full width.
fn rule_line(ui: &mut Ui) {
    let origin = ui.cursor().min;
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(vec2(width, 1.0), Sense::hover());
    ui.painter().hline(
        origin.x..=origin.x + width,
        rect.top() + 0.5,
        Stroke::new(1.0, t::BORDER),
    );
}

/// `.lbl` on the left, a caption (`font-size: <size>`, muted) on the right, centred
/// (`align-items: center; justify-content: space-between`).
fn caption_row(ui: &mut Ui, label: &str, caption: &str, size: f32) {
    let width = ui.available_width();
    let lg = lbl_galley(ui, label, t::TEXT_MUTED);
    let font = lab_theme::font::body(size);
    let cg = galley(ui, caption, font.clone(), t::TEXT_MUTED);
    let h = css_line_height(ui, &lab_theme::font::semibold(11.0)).max(css_line_height(ui, &font));
    let (rect, resp) = ui.allocate_exact_size(vec2(width, h), Sense::hover());
    let painter = ui.painter();
    paint_line(painter, &lg, rect.left(), rect.top(), h, t::TEXT_MUTED);
    paint_line(
        painter,
        &cg,
        rect.right() - cg.size().x,
        rect.top(),
        h,
        t::TEXT_MUTED,
    );
    let s = format!("{label}: {caption}");
    resp.widget_info(|| WidgetInfo::labeled(Role::Label, true, &s));
}

/// One line of text in a CSS `line-height: normal` box, as a `Label`.
fn css_text(ui: &mut Ui, text: &str, font: FontId, color: Color32) -> Response {
    let g = galley(ui, text, font.clone(), color);
    let h = css_line_height(ui, &font);
    let (rect, resp) = ui.allocate_exact_size(vec2(g.size().x, h), Sense::hover());
    paint_line(ui.painter(), &g, rect.left(), rect.top(), h, color);
    let s = text.to_string();
    resp.widget_info(|| WidgetInfo::labeled(Role::Label, true, &s));
    resp
}

/// The big rule code: JetBrains Mono 500 30 px, ink, `letter-spacing: -0.02em`.
fn big_code(ui: &mut Ui, code: &str) {
    let font = lab_theme::font::mono_medium(30.0);
    let g = galley_spaced(ui, code, font.clone(), t::INK, -0.6);
    let h = css_line_height(ui, &font);
    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), h), Sense::hover());
    paint_line(ui.painter(), &g, rect.left(), rect.top(), h, t::INK);
    let s = code.to_string();
    resp.widget_info(|| WidgetInfo::labeled(Role::Label, true, &s));
}

/// A Tray bit row (`BitRow`), laid out as CSS does: the on-bits' 3 px drop is a box-shadow,
/// so the row takes 30 px.
fn bit_row(ui: &mut Ui, prefix: &str, mask: u32) -> Option<usize> {
    let origin = ui.cursor().min;
    let width = ui.available_width();
    let mut child = ui.new_child(
        UiBuilder::new()
            .max_rect(Rect::from_min_size(
                origin,
                vec2(width, t::size::BIT_H + 3.0),
            ))
            .layout(eframe::egui::Layout::top_down(eframe::egui::Align::Min)),
    );
    let clicked = BitRow::new(prefix, BITS, mask).show(&mut child);
    ui.advance_cursor_after_rect(Rect::from_min_size(origin, vec2(width, t::size::BIT_H)));
    clicked
}

/// The 40 px Moore glyph and "8 neighbours · edges wrap" (`gap: 6px`, 12 px muted).
fn neighbourhood_row(ui: &mut Ui, neighbours: usize) {
    let text = format!("{neighbours} neighbours · edges wrap");
    let width = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(vec2(width, 40.0), Sense::hover());
    CellGlyph::MOORE.paint(
        ui.painter(),
        Rect::from_min_size(rect.min, vec2(40.0, 40.0)),
    );
    let font = lab_theme::font::body(12.0);
    let g = galley(ui, &text, font.clone(), t::TEXT_MUTED);
    let h = css_line_height(ui, &font);
    paint_line(
        ui.painter(),
        &g,
        rect.left() + 46.0,
        rect.center().y - h / 2.0,
        h,
        t::TEXT_MUTED,
    );
    resp.widget_info(|| WidgetInfo::labeled(Role::Label, true, &text));
}

/// Two equal columns `gap` apart (`repeat(2, minmax(0, 1fr))`): calls `add(ui, i)` for each
/// of `n` cells in a full-width child `Ui`; rows are as tall as their tallest cell.
fn grid2(ui: &mut Ui, gap: f32, n: usize, mut add: impl FnMut(&mut Ui, usize)) {
    let origin = ui.cursor().min;
    let width = ui.available_width();
    let cw = (width - gap) / 2.0;
    let mut y = origin.y;
    for row in 0..n.div_ceil(2) {
        let mut row_h: f32 = 0.0;
        for col in 0..2 {
            let i = row * 2 + col;
            if i >= n {
                break;
            }
            let rect = Rect::from_min_size(
                pos2(origin.x + col as f32 * (cw + gap), y),
                vec2(cw, f32::INFINITY),
            );
            let mut cell = ui.new_child(
                UiBuilder::new()
                    .id_salt(("grid2", i))
                    .max_rect(rect)
                    .layout(eframe::egui::Layout::top_down(eframe::egui::Align::Min)),
            );
            add(&mut cell, i);
            row_h = row_h.max(cell.min_rect().height());
        }
        y += row_h + gap;
    }
    let used = Rect::from_min_max(origin, pos2(origin.x + width, (y - gap).max(origin.y)));
    ui.advance_cursor_after_rect(used);
}
