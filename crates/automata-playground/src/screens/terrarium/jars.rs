//! The Terrarium's stage: `.graph` with `overflow: auto; padding: 22px` holding a grid of jar
//! cards (`repeat(auto-fill, minmax(300px, 1fr)); gap: 18px`).
//!
//! A card (`.jarcard`, `padding: 14px 14px 16px; gap: 10px`, centred column) holds the jar
//! area (the bench's [`JarScene::FOOTPRINT`], 22 px taller than the mockup's 228 px box so the
//! slab clears the code), the code / name / Keep row and the liveliness row.

use super::State;
use crate::widgets::text::{css_line_height, galley, paint_line, text_top};
use crate::widgets::{Keep, set_bounds};
use ap_bench::{JarScene, jar_bob};
use ap_engine::Liveliness;
use ap_theme as t;
use eframe::egui::{
    Align, CornerRadius, Layout, Rect, ScrollArea, Sense, Stroke, StrokeKind, Ui, UiBuilder,
    WidgetInfo, accesskit::Role, pos2, vec2,
};
use lab_ui::layout::auto_fill;

/// The stage padding and the grid gap.
const PAD: f32 = 22.0;
const GAP: f32 = 18.0;
/// `minmax(300px, 1fr)`.
const MIN_CARD: f32 = 300.0;
/// Card padding (top, sides, bottom) inside its 1 px border, and the gap between its rows.
const PAD_TOP: f32 = 14.0;
const PAD_X: f32 = 14.0;
const PAD_BOTTOM: f32 = 16.0;
const ROW_GAP: f32 = 10.0;
/// The "Liveliness" label and the "N alive" columns (`width: 58px` / `74px`).
const METER_LABEL_W: f32 = 58.0;
const POP_W: f32 = 74.0;

/// The vertical sizes of a card, from the fonts.
struct CardMetrics {
    code_line: f32,
    name_line: f32,
    /// The code / name / Keep row.
    head: f32,
    /// The liveliness row.
    meter: f32,
}

impl CardMetrics {
    fn new(ui: &Ui) -> CardMetrics {
        let code_line = css_line_height(ui, &lab_theme::font::mono_medium(15.0));
        let name_line = css_line_height(ui, &lab_theme::font::body(11.5));
        let meter = css_line_height(ui, &lab_theme::font::body(11.0))
            .max(css_line_height(ui, &lab_theme::font::mono(11.0)));
        CardMetrics {
            code_line,
            name_line,
            head: (code_line + 2.0 + name_line).max(t::size::KEEP),
            meter,
        }
    }

    /// The card's border-box height.
    fn height(&self) -> f32 {
        1.0 + PAD_TOP
            + JarScene::FOOTPRINT.y
            + ROW_GAP
            + self.head
            + ROW_GAP
            + self.meter
            + PAD_BOTTOM
            + 1.0
    }
}

/// The meter colour for a liveliness value (quiet grey, frantic coral, else the accent).
pub fn meter_color(liveliness: f64) -> eframe::egui::Color32 {
    match Liveliness::of(liveliness) {
        Liveliness::Quiet => t::METER_QUIET,
        Liveliness::Lively => t::ACCENT,
        Liveliness::Frantic => t::METER_FRANTIC,
    }
}

impl State {
    /// The scrolling grid of jar cards on the stage.
    pub(super) fn jar_grid(&mut self, ui: &mut Ui) {
        let out = ScrollArea::vertical()
            .id_salt("terrarium-jars")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let origin = ui.cursor().min;
                let width = ui.available_width();
                let (cols, card_w) = auto_fill(width - 2.0 * PAD, MIN_CARD, GAP);
                let m = CardMetrics::new(ui);
                let card_h = m.height();
                let n = self.terr.jars().len();
                let rows = n.div_ceil(cols.max(1));
                let mut toggled = None;
                for i in 0..n {
                    let (c, r) = (i % cols, i / cols);
                    let rect = Rect::from_min_size(
                        origin
                            + vec2(
                                PAD + c as f32 * (card_w + GAP),
                                PAD + r as f32 * (card_h + GAP),
                            ),
                        vec2(card_w, card_h),
                    );
                    if self.card(ui, i, rect, &m) {
                        toggled = Some(i);
                    }
                }
                if let Some(i) = toggled {
                    let kept = self.terr.is_kept(i);
                    self.keep(i, !kept);
                }
                let height = 2.0 * PAD + rows as f32 * card_h + rows.saturating_sub(1) as f32 * GAP;
                ui.allocate_rect(
                    Rect::from_min_size(origin, vec2(width, height)),
                    Sense::hover(),
                );
            });
        // a Keep button scrolled into view inside the stage: the page follows (390 px)
        crate::layout::bubble_scroll(ui, out.id, out.state.clamped_offset(), out.inner_rect);
    }

    /// One jar card at `rect`; returns whether its Keep button was pressed.
    fn card(&self, ui: &mut Ui, i: usize, rect: Rect, m: &CardMetrics) -> bool {
        let Some(jar) = self.terr.jar(i) else {
            return false;
        };
        let kept = self.terr.is_kept(i);
        let code = jar.rule().code();
        let name = jar.name();
        let population = jar.population();
        let liveliness = jar.liveliness();
        let mut card = ui.new_child(
            UiBuilder::new()
                .id_salt(("jar-card", i))
                .max_rect(rect)
                .layout(Layout::top_down(Align::Min))
                .accessibility_role(Role::Group)
                .accessibility_label(format!("Jar {}", i + 1)),
        );
        set_bounds(&card, rect);
        let (fill, border) = if kept {
            (t::JAR_CARD_KEPT, t::EDGE)
        } else {
            (t::JAR_CARD, t::JAR_CARD_BORDER)
        };
        card.painter().rect(
            rect,
            CornerRadius::same(t::radius::JAR_CARD),
            fill,
            Stroke::new(1.0, border),
            StrokeKind::Inside,
        );

        // the jar, centred (shrunk only if a card is ever narrower than the jar)
        let jar_w = JarScene::FOOTPRINT.x.min(rect.width() - 2.0);
        let jar_h = JarScene::FOOTPRINT.y * jar_w / JarScene::FOOTPRINT.x;
        let area = Rect::from_min_size(
            pos2(rect.center().x - jar_w / 2.0, rect.top() + 1.0 + PAD_TOP),
            vec2(jar_w, jar_h),
        );
        if card.is_rect_visible(area) {
            let grid = jar.grid();
            let cell = |x: usize, y: usize| grid.is_alive(x, y);
            let mut scene = JarScene::new(&cell, t::JAR_INKS[i % t::JAR_INKS.len()]);
            scene.size = grid.width();
            scene.bob = jar_bob(self.bob_secs, i);
            scene.paint(card.painter(), area);
        }
        let description = JarScene::description(&code, population);
        card.interact(area, card.scope_id().with("jar"), Sense::hover())
            .widget_info(|| WidgetInfo::labeled(Role::Image, true, &description));

        // code / name · Keep
        let left = rect.left() + 1.0 + PAD_X;
        let right = rect.right() - 1.0 - PAD_X;
        let head = Rect::from_min_max(
            pos2(left, area.top() + JarScene::FOOTPRINT.y + ROW_GAP),
            pos2(right, area.top() + JarScene::FOOTPRINT.y + ROW_GAP + m.head),
        );
        let keep_label = if kept { "Kept" } else { "Keep" };
        let keep_w = 1.0
            + 10.0
            + 13.0
            + 6.0
            + galley(
                &card,
                keep_label,
                lab_theme::font::medium(12.0),
                t::TEXT_MUTED,
            )
            .size()
            .x
            + 10.0
            + 1.0;
        let keep_rect = Rect::from_center_size(
            pos2(right - keep_w / 2.0, head.center().y),
            vec2(keep_w, t::size::KEEP),
        );
        let code_color = if kept { t::INK } else { t::TEXT };
        let text_w = (keep_rect.left() - 10.0 - left).max(10.0);
        let clip = card.painter().with_clip_rect(
            Rect::from_min_size(pos2(left, head.top() - 4.0), vec2(text_w, m.head + 8.0))
                .intersect(card.clip_rect()),
        );
        let text_top_y = head.center().y - (m.code_line + 2.0 + m.name_line) / 2.0;
        let code_g = galley(&card, &code, lab_theme::font::mono_medium(15.0), code_color);
        let name_g = galley(&card, name, lab_theme::font::body(11.5), t::TEXT_MUTED);
        paint_line(&clip, &code_g, left, text_top_y, m.code_line, code_color);
        let name_y = text_top_y + m.code_line + 2.0;
        paint_line(&clip, &name_g, left, name_y, m.name_line, t::TEXT_MUTED);
        label(
            &card,
            Rect::from_min_size(pos2(left, text_top_y), vec2(text_w, m.code_line)),
            ("code", i),
            &code,
        );
        label(
            &card,
            Rect::from_min_size(pos2(left, name_y), vec2(text_w, m.name_line)),
            ("name", i),
            name,
        );
        let keep_name = format!("Keep jar {}", i + 1);
        let mut keep_ui = card.new_child(
            UiBuilder::new()
                .id_salt(("keep", i))
                .max_rect(keep_rect)
                .layout(Layout::left_to_right(Align::Center)),
        );
        let pressed = Keep::new(kept)
            .accessible_name(&keep_name)
            .show(&mut keep_ui)
            .clicked();

        // Liveliness ▬▬▬▭▭ N alive
        let row = Rect::from_min_max(
            pos2(left, head.bottom() + ROW_GAP),
            pos2(right, head.bottom() + ROW_GAP + m.meter),
        );
        let cy = row.center().y;
        let painter = card.painter();
        let lg = galley(
            &card,
            "Liveliness",
            lab_theme::font::body(11.0),
            t::TEXT_MUTED,
        );
        painter.galley(pos2(left, text_top(cy, &lg)), lg, t::TEXT_MUTED);
        let track = Rect::from_min_max(
            pos2(left + METER_LABEL_W + 8.0, cy - 3.0),
            pos2(right - POP_W - 8.0, cy + 3.0),
        );
        painter.rect_filled(track, CornerRadius::same(3), t::METER_TRACK);
        let live = liveliness.clamp(0.0, 1.0) as f32;
        if live > 0.0 {
            let fill = Rect::from_min_size(track.min, vec2(track.width() * live, track.height()));
            painter.rect_filled(fill, CornerRadius::same(3), meter_color(liveliness));
        }
        let pop_text = format!("{population} alive");
        let pg = galley(&card, &pop_text, lab_theme::font::mono(11.0), t::TEXT_DIM);
        painter.galley(
            pos2(right - pg.size().x, text_top(cy, &pg)),
            pg,
            t::TEXT_DIM,
        );
        let percent = (live * 100.0).round();
        let meter = card.interact(track, card.scope_id().with("meter"), Sense::hover());
        let meter_name = format!("Liveliness {percent}%");
        meter.widget_info(|| WidgetInfo::labeled(Role::Meter, true, &meter_name));
        card.ctx().accesskit_node_builder(meter.id, |b| {
            b.set_numeric_value(f64::from(percent));
            b.set_min_numeric_value(0.0);
            b.set_max_numeric_value(100.0);
        });
        label(
            &card,
            Rect::from_min_max(pos2(right - POP_W, row.top()), row.max),
            ("pop", i),
            &pop_text,
        );
        pressed
    }
}

/// An AccessKit label over painted text.
fn label(ui: &Ui, rect: Rect, salt: (&str, usize), text: &str) {
    ui.interact(rect, ui.scope_id().with(salt), Sense::hover())
        .widget_info(|| WidgetInfo::labeled(Role::Label, true, text));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn meter_colours_follow_the_mockup() {
        assert_eq!(meter_color(0.05), t::METER_QUIET);
        assert_eq!(meter_color(0.5), t::ACCENT);
        assert_eq!(meter_color(0.9), t::METER_FRANTIC);
    }
}
