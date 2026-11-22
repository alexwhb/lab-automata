//! The Loom's aside ("Rule and presets"): the rule number with its binary, blurb and −1 / +1 /
//! Surprise me (LOOM-4); the famous threads (LOOM-5); First row and Speed (LOOM-6).

use super::{FirstRow, SPEED_RANGE, State};
use crate::widgets::text::{css_line_height, galley_spaced, text_block};
use crate::widgets::{Preset, SegGroup, SegItem, SegStyle, Slider, ghost, lbl};
use ap_engine::elementary::FAMOUS;
use ap_theme as t;
use eframe::egui::{Galley, Sense, Stroke, Ui, WidgetInfo, accesskit::Role, pos2, vec2};

/// The First row segments.
const FIRST_ROW: [SegItem<'static>; 2] = [SegItem::new("Single cell"), SegItem::new("Random")];

impl State {
    /// The aside's three sections (the column gives `padding: 16px; gap: 16px`).
    pub(super) fn aside(&mut self, ui: &mut Ui) {
        section(ui, 6.0, false, |ui| self.rule_section(ui));
        section(ui, 6.0, true, |ui| self.famous_section(ui));
        section(ui, 10.0, true, |ui| self.start_section(ui));
    }

    /// "Rule": the number (Space Grotesk 44, ink) and binary (mono 14, `.18em`) on one
    /// baseline, the blurb, then −1 / +1 / Surprise me.
    fn rule_section(&mut self, ui: &mut Ui) {
        lbl(ui, "Rule");
        let rule = self.elementary();
        number_row(ui, &rule.number().to_string(), &rule.binary());
        text_block(
            ui,
            &rule.blurb(),
            lab_theme::font::body(12.5),
            t::TEXT_DIM,
            Some(12.5 * 1.55),
        );
        // the theme's 40 px `interact_size` would make the row taller than its 30 px buttons
        ui.spacing_mut().interact_size.y = 0.0;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = vec2(6.0, 0.0);
            if ghost("−1")
                .accessible_name("Previous rule")
                .height(30.0)
                .show(ui)
                .clicked()
            {
                self.previous_rule();
            }
            if ghost("+1")
                .accessible_name("Next rule")
                .height(30.0)
                .show(ui)
                .clicked()
            {
                self.next_rule();
            }
            if ghost("Surprise me").height(30.0).show(ui).clicked() {
                self.surprise();
            }
        });
    }

    /// "Famous threads": the six presets, the current rule's highlighted.
    fn famous_section(&mut self, ui: &mut Ui) {
        lbl(ui, "Famous threads");
        let rule = self.rule();
        for f in FAMOUS {
            let number = f.rule.to_string();
            if Preset::new(&number, f.name, f.note)
                .on(f.rule == rule)
                .show(ui)
                .clicked()
            {
                self.set_rule(f.rule);
            }
        }
    }

    /// "First row" (Single cell / Random) and the Speed slider.
    fn start_section(&mut self, ui: &mut Ui) {
        lbl(ui, "First row");
        let current = FirstRow::of(self.start());
        let selected = FirstRow::ALL.iter().position(|f| *f == current);
        if let Some(i) = SegGroup::new("First row", &FIRST_ROW, selected)
            .style(SegStyle::Mini { fill: true })
            .show(ui)
            .clicked
        {
            self.pick_first_row(FirstRow::ALL[i]);
        }
        let mut speed = self.speed;
        let text = format!("{} rows/s", speed.round() as u32);
        if Slider::new("Speed", &mut speed, SPEED_RANGE)
            .step(1.0)
            .value_text(text)
            .show(ui)
            .changed()
        {
            self.set_speed(speed);
        }
    }
}

/// A `<section>` of an aside: `gap` between its children and, after the first,
/// `border-top: 1px solid #242424; padding-top: 14px` (the Terrarium's aside uses it too).
pub(in crate::screens) fn section(
    ui: &mut Ui,
    gap: f32,
    border_top: bool,
    add: impl FnOnce(&mut Ui),
) {
    ui.scope(|ui| {
        ui.spacing_mut().item_spacing = vec2(gap, gap);
        if border_top {
            let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 1.0), Sense::hover());
            ui.painter()
                .hline(rect.x_range(), rect.center().y, Stroke::new(1.0, t::BORDER));
            ui.add_space((14.0 - gap).max(0.0));
        }
        add(ui);
    });
}

/// The big rule number and its binary on one baseline (`display: flex; align-items:
/// baseline; gap: 12px`), each in its CSS line box.
fn number_row(ui: &mut Ui, number: &str, binary: &str) {
    let num_font = lab_theme::font::heading(44.0);
    let bin_font = lab_theme::font::mono(14.0);
    let num = galley_spaced(ui, number, num_font.clone(), t::INK, -0.88);
    let bin = galley_spaced(ui, binary, bin_font.clone(), t::TEXT_DIM, 2.52);
    let num_line = css_line_height(ui, &num_font);
    let bin_line = css_line_height(ui, &bin_font);
    // each line box puts its baseline at half-leading + ascent; align those
    let num_base = num_line / 2.0 + baseline_offset(&num) - num.size().y / 2.0;
    let bin_base = bin_line / 2.0 + baseline_offset(&bin) - bin.size().y / 2.0;
    let base = num_base.max(bin_base);
    let height = (base - num_base + num_line).max(base - bin_base + bin_line);
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
    let baseline = rect.top() + base;
    let painter = ui.painter();
    painter.galley(
        pos2(rect.left(), baseline - baseline_offset(&num)),
        num.clone(),
        t::INK,
    );
    let bin_x = rect.left() + num.size().x + 12.0;
    painter.galley(
        pos2(bin_x, baseline - baseline_offset(&bin)),
        bin.clone(),
        t::TEXT_DIM,
    );
    let label = format!("Rule {number}, binary {binary}");
    ui.interact(rect, ui.scope_id().with("rule-number"), Sense::hover())
        .widget_info(|| WidgetInfo::labeled(Role::Label, true, &label));
}

/// The baseline's distance from a one-line galley's top.
fn baseline_offset(g: &Galley) -> f32 {
    g.rows.first().map_or(g.size().y, |r| {
        r.row.glyphs.first().map_or(r.size.y, |gl| gl.pos.y)
    })
}
