//! The Terrarium's aside ("Breeding"): Mutation, "Breed brood N+1" and the keep note
//! (TERR-2); the lineage (TERR-3); "Use a favourite" (NAV-4).

use super::{State, mutation_range};
use crate::ctx::Ctx;
use crate::router::Route;
use crate::screens::loom::section;
use crate::widgets::text::{css_line_height, galley, text_block, text_top};
use crate::widgets::{Icon, Slider, ghost, lbl, prim, set_bounds};
use ap_engine::LineageEntry;
use ap_theme as t;
use eframe::egui::{
    CornerRadius, Pos2, Rect, Sense, Shape, Stroke, StrokeKind, Ui, UiBuilder, WidgetInfo,
    accesskit::Role, pos2, vec2,
};
use lab_ui::paint::advance_block;

/// The breeding sentence.
pub const BREEDING_TEXT: &str = "Children mix their parents’ rules, plus a few random flips.";
/// The lineage's placeholder before the first brood.
pub const NOTHING_BRED: &str = "Nothing bred yet.";

impl State {
    /// The aside's three sections (the column gives `padding: 16px; gap: 16px`).
    pub(super) fn aside(&mut self, ctx: &mut Ctx, ui: &mut Ui) {
        section(ui, 10.0, false, |ui| self.breeding(ui));
        section(ui, 8.0, true, |ui| self.lineage_section(ui));
        section(ui, 8.0, true, |ui| self.favourite(ctx, ui));
    }

    /// "Breeding": the sentence, Mutation, the breed button and which jars are parents.
    fn breeding(&mut self, ui: &mut Ui) {
        lbl(ui, "Breeding");
        text_block(
            ui,
            BREEDING_TEXT,
            lab_theme::font::body(13.0),
            t::TEXT_DIM,
            Some(13.0 * 1.55),
        );
        let mutation = self.mutation();
        let mut value = f32::from(mutation);
        let text = if mutation == 1 {
            "1 bit flip".to_string()
        } else {
            format!("{mutation} bit flips")
        };
        if Slider::new("Mutation", &mut value, mutation_range())
            .step(1.0)
            .value_text(text)
            .show(ui)
            .changed()
        {
            self.set_mutation(value.round() as u8);
        }
        let label = self.terr.breed_label();
        if prim(&label).icon(Icon::Breed).big().show(ui).clicked() {
            self.breed();
        }
        text_block(
            ui,
            &self.keep_note(),
            lab_theme::font::body(12.0),
            t::TEXT_MUTED,
            None,
        );
    }

    /// "Lineage": newest first, or the dashed "Nothing bred yet." box.
    fn lineage_section(&self, ui: &mut Ui) {
        lbl(ui, "Lineage");
        if self.terr.lineage().is_empty() {
            dashed_note(ui, NOTHING_BRED);
        } else {
            for entry in self.terr.lineage().newest_first() {
                lineage_row(ui, entry);
            }
        }
    }

    /// "Use a favourite": the two hand-offs (NAV-4).
    fn favourite(&self, ctx: &mut Ctx, ui: &mut Ui) {
        lbl(ui, "Use a favourite");
        if ghost("Plant in the Tray")
            .full_width()
            .link()
            .show(ui)
            .clicked()
        {
            self.send_to(ctx, Route::Tray);
        }
        if ghost("Analyse in Rule lab")
            .full_width()
            .link()
            .show(ui)
            .clicked()
        {
            self.send_to(ctx, Route::RuleLab);
        }
    }
}

/// The dashed note box: 12 px faint text, `border: 1px dashed #2E2E2E; border-radius: 8px;
/// padding: 12px`.
fn dashed_note(ui: &mut Ui, text: &str) {
    let width = ui.available_width();
    let font = lab_theme::font::body(12.0);
    let line = css_line_height(ui, &font);
    let (rect, resp) = ui.allocate_exact_size(vec2(width, line + 24.0 + 2.0), Sense::hover());
    let painter = ui.painter();
    let outline = rounded_outline(rect.shrink(0.5), 8.0);
    painter.extend(Shape::dashed_line(
        &outline,
        Stroke::new(1.0, t::DASHED),
        3.0,
        3.0,
    ));
    let g = galley(ui, text, font, t::TEXT_FAINT);
    painter.galley(
        pos2(rect.left() + 13.0, text_top(rect.center().y, &g)),
        g,
        t::TEXT_FAINT,
    );
    let s = text.to_string();
    resp.widget_info(|| WidgetInfo::labeled(Role::Label, true, &s));
}

/// A closed rounded-rectangle outline (clockwise from the top-left corner's end).
fn rounded_outline(r: Rect, radius: f32) -> Vec<Pos2> {
    let radius = radius.min(r.width() / 2.0).min(r.height() / 2.0);
    let corners = [
        (pos2(r.right() - radius, r.top() + radius), -90.0f32),
        (pos2(r.right() - radius, r.bottom() - radius), 0.0),
        (pos2(r.left() + radius, r.bottom() - radius), 90.0),
        (pos2(r.left() + radius, r.top() + radius), 180.0),
    ];
    let mut pts = vec![pos2(r.left() + radius, r.top())];
    for (c, start) in corners {
        for k in 0..=6 {
            let a = (start + 15.0 * k as f32).to_radians();
            pts.push(c + vec2(a.cos(), a.sin()) * radius);
        }
    }
    pts.push(pos2(r.left() + radius, r.top()));
    pts
}

/// One lineage row: "brood N" (mono 11, faint, 54 px, `padding-top: 3px`) and the parents'
/// codes as wrapping chips (mono 11 ink on the callout tint, edge border, `padding: 2px 6px`,
/// radius 4, gap 4).
fn lineage_row(ui: &mut Ui, entry: &LineageEntry) {
    let width = ui.available_width();
    let origin = ui.cursor().min;
    let font = lab_theme::font::mono(11.0);
    let line = css_line_height(ui, &font);
    let chip_h = line + 4.0 + 2.0;
    let chips_left = origin.x + 54.0 + 10.0;
    let chips_w = (width - 64.0).max(40.0);
    // wrap the chips
    let galleys: Vec<_> = entry
        .parents
        .iter()
        .map(|c| galley(ui, c, font.clone(), t::INK))
        .collect();
    let mut rects = Vec::with_capacity(galleys.len());
    let (mut x, mut y) = (0.0f32, 0.0f32);
    for g in &galleys {
        let w = g.size().x + 12.0 + 2.0;
        if x > 0.0 && x + w > chips_w {
            x = 0.0;
            y += chip_h + 4.0;
        }
        rects.push(Rect::from_min_size(
            pos2(chips_left + x, origin.y + y),
            vec2(w, chip_h),
        ));
        x += w + 4.0;
    }
    let height = (y + chip_h).max(3.0 + line);
    let used = Rect::from_min_size(origin, vec2(width, height));
    let name = format!("brood {}", entry.brood);
    let group = ui.new_child(
        UiBuilder::new()
            .id_salt(("lineage", entry.brood))
            .max_rect(used)
            .accessibility_role(Role::Group)
            .accessibility_label(format!("{name} parents")),
    );
    set_bounds(&group, used);
    let painter = group.painter();
    let ng = galley(&group, &name, font.clone(), t::TEXT_FAINT);
    let name_rect = Rect::from_min_size(pos2(origin.x, origin.y + 3.0), vec2(54.0, line));
    painter.galley(
        pos2(name_rect.left(), text_top(name_rect.center().y, &ng)),
        ng,
        t::TEXT_FAINT,
    );
    group
        .interact(name_rect, group.scope_id().with("name"), Sense::hover())
        .widget_info(|| WidgetInfo::labeled(Role::Label, true, &name));
    for (k, (rect, g)) in rects.iter().zip(galleys).enumerate() {
        painter.rect(
            *rect,
            CornerRadius::same(4),
            t::CALLOUT,
            Stroke::new(1.0, t::EDGE),
            StrokeKind::Inside,
        );
        painter.galley(
            pos2(rect.left() + 7.0, text_top(rect.center().y, &g)),
            g,
            t::INK,
        );
        let code = entry.parents[k].clone();
        group
            .interact(*rect, group.scope_id().with(("chip", k)), Sense::hover())
            .widget_info(|| WidgetInfo::labeled(Role::Label, true, &code));
    }
    advance_block(ui, used);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outline_is_closed_and_inside() {
        let r = Rect::from_min_size(pos2(0.0, 0.0), vec2(100.0, 40.0));
        let o = rounded_outline(r, 8.0);
        assert_eq!(o.first(), o.last());
        assert!(o.iter().all(|p| r.expand(0.01).contains(*p)));
    }
}
