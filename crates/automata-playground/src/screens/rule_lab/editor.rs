//! The rule-script editor (LAB-4): the mockup's `.code` block made editable — JetBrains Mono
//! 12 px / 1.65 `#D9D9D9` on the `#060606` card, `white-space: pre` (no wrapping; the card
//! scrolls sideways like `overflow-x: auto`). Lines with errors get the series' warning red: a
//! tinted band behind the line, the text underlined and a 2 px bar in the card's left padding;
//! the messages are listed under the card ("Line 2: …") with a note that the last valid script
//! is still in effect.
//!
//! AccessKit: a multi-line text input named "Rule script" whose description lists the errors.

use crate::widgets::Card;
use crate::widgets::text::{css_rows, wrapped};
use ap_engine::ScriptError;
use ap_theme as t;
use eframe::egui::{
    Align, CornerRadius, Frame, Galley, Margin, Rect, ScrollArea, Sense, Shape, Stroke, TextBuffer,
    TextEdit, Ui, WidgetInfo, accesskit::Role, epaint::text::LayoutJob, epaint::text::TextFormat,
    pos2, vec2,
};
use std::sync::Arc;

/// The editor's accessible name.
pub const NAME: &str = "Rule script";
/// The note under the errors.
pub const LAST_VALID: &str = "The last valid script is still in effect.";
/// `.code` line height: 12 px × 1.65.
pub const LINE_H: f32 = 19.8;

/// What the editor did this frame.
#[derive(Debug)]
pub struct EditorOutput {
    /// The new text, when the user changed it.
    pub changed: Option<String>,
    /// The editor has keyboard focus (the user is typing).
    pub focused: bool,
}

/// The layout job for `text`: the `.code` look, error lines underlined in red.
fn job(text: &str, error_lines: &[usize]) -> LayoutJob {
    let mut job = LayoutJob::default();
    let normal = TextFormat {
        font_id: lab_theme::font::mono(12.0),
        color: t::TEXT_DIM,
        line_height: Some(LINE_H),
        valign: Align::Center,
        ..Default::default()
    };
    let bad = TextFormat {
        underline: Stroke::new(1.0, t::ERROR),
        ..normal.clone()
    };
    for (i, line) in text.split_inclusive('\n').enumerate() {
        let fmt = if error_lines.contains(&(i + 1)) {
            &bad
        } else {
            &normal
        };
        job.append(line, 0.0, fmt.clone());
    }
    if text.is_empty() {
        job.append("", 0.0, normal);
    }
    job.wrap.max_width = f32::INFINITY;
    job
}

/// Draws the editor (card + errors) for `text` with `errors`, full width at the cursor.
pub fn script_editor(ui: &mut Ui, text: &str, errors: &[ScriptError]) -> EditorOutput {
    let mut buf = text.to_string();
    let mut error_lines: Vec<usize> = errors.iter().map(|e| e.line).collect();
    error_lines.dedup();
    let mut focused = false;
    let mut changed = false;
    Card::panel().fill(t::INSET).show(ui, |ui| {
        // a slot under the text for the error bands
        let bands = ui.painter().add(Shape::Noop);
        let mut layouter = |ui: &Ui, buf: &dyn TextBuffer, _wrap: f32| -> Arc<Galley> {
            css_rows(
                ui.fonts_mut(|f| f.layout_job(job(buf.as_str(), &error_lines))),
                LINE_H,
            )
        };
        let width = ui.available_width();
        let card_left = ui.max_rect().left() - 14.0;
        let out = ScrollArea::horizontal()
            .id_salt("rule-script-scroll")
            .auto_shrink([false, true])
            .show(ui, |ui| {
                TextEdit::multiline(&mut buf)
                    .id_salt("rule-script")
                    .frame(Frame::NONE)
                    .margin(Margin::ZERO)
                    .desired_width(width)
                    .desired_rows(1)
                    .layouter(&mut layouter)
                    .show(ui)
            });
        let edit = out.inner;
        focused = edit.response.has_focus();
        changed = edit.response.changed();
        // red bands behind the lines with errors, and a bar in the card's left padding
        let view = out.inner_rect;
        let mut shapes = Vec::new();
        for &line in &error_lines {
            let Some(row) = row_rect(&edit.galley, line) else {
                continue;
            };
            let y = edit.galley_pos.y + row.top();
            let band = Rect::from_min_max(
                pos2(view.left() - 6.0, y),
                pos2(view.right() + 6.0, y + row.height()),
            );
            shapes.push(Shape::rect_filled(band, CornerRadius::same(3), t::ERROR_BG));
            shapes.push(Shape::rect_filled(
                Rect::from_min_max(
                    pos2(card_left + 5.0, y + 2.0),
                    pos2(card_left + 7.0, y + row.height() - 2.0),
                ),
                CornerRadius::same(1),
                t::ERROR,
            ));
        }
        ui.painter().set(bands, Shape::Vec(shapes));
        // the accessible name and the errors as its description
        let summary = summary(errors);
        let id = edit.response.id;
        ui.ctx().accesskit_node_builder(id, |b| {
            b.set_label(NAME);
            match &summary {
                Some(s) => b.set_description(s.clone()),
                None => b.clear_description(),
            }
        });
    });
    if !errors.is_empty() {
        error_list(ui, errors);
    }
    EditorOutput {
        changed: changed.then_some(buf),
        focused,
    }
}

/// The rect (galley-relative) of 1-based `line`'s row (no wrapping: one row per line).
fn row_rect(galley: &Galley, line: usize) -> Option<Rect> {
    let row = galley.rows.get(line.checked_sub(1)?)?;
    Some(row.rect())
}

/// "2 errors. Line 2: …; Line 5: …" for the accessible description.
pub fn summary(errors: &[ScriptError]) -> Option<String> {
    if errors.is_empty() {
        return None;
    }
    let n = errors.len();
    let list: Vec<String> = errors.iter().map(message).collect();
    Some(format!(
        "{n} error{}. {}",
        if n == 1 { "" } else { "s" },
        list.join("; ")
    ))
}

/// "Line 2: unknown boundary `tor` (use torus or dead)".
pub fn message(e: &ScriptError) -> String {
    format!("Line {}: {}", e.line, e.message)
}

/// The messages under the card, then the note that the last valid script still runs.
fn error_list(ui: &mut Ui, errors: &[ScriptError]) {
    let width = ui.available_width();
    let font = lab_theme::font::body(12.0);
    let galleys: Vec<_> = errors
        .iter()
        .map(|e| {
            wrapped(
                ui,
                &message(e),
                font.clone(),
                t::ERROR,
                width - 12.0,
                Some(18.0),
            )
        })
        .collect();
    let note = wrapped(
        ui,
        LAST_VALID,
        lab_theme::font::body(11.0),
        t::TEXT_MUTED,
        width - 12.0,
        Some(16.0),
    );
    let h = galleys.iter().map(|g| g.size().y + 4.0).sum::<f32>() + note.size().y;
    let (rect, _) = ui.allocate_exact_size(vec2(width, h), Sense::hover());
    let painter = ui.painter();
    let mut y = rect.top();
    for (g, e) in galleys.into_iter().zip(errors) {
        let gh = g.size().y;
        painter.rect_filled(
            Rect::from_min_size(pos2(rect.left() + 1.0, y + 4.0), vec2(2.0, gh - 8.0)),
            CornerRadius::same(1),
            t::ERROR,
        );
        painter.galley(pos2(rect.left() + 12.0, y), g, t::ERROR);
        let text = message(e);
        ui.interact(
            Rect::from_min_size(pos2(rect.left(), y), vec2(width, gh)),
            ui.scope_id().with(("script-error", e.line, &e.message)),
            Sense::hover(),
        )
        .widget_info(|| WidgetInfo::labeled(Role::Label, true, &text));
        y += gh + 4.0;
    }
    let note_rect = Rect::from_min_size(pos2(rect.left() + 12.0, y), note.size());
    painter.galley(note_rect.min, note, t::TEXT_MUTED);
    ui.interact(note_rect, ui.scope_id().with("script-note"), Sense::hover())
        .widget_info(|| WidgetInfo::labeled(Role::Label, true, LAST_VALID));
}
