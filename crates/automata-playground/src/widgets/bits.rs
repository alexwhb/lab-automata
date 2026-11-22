//! Rule bits: the `.bit` toggles of the Born-with / Survives-with rows (Tray 28 × 30 with the
//! 3 px drop when on; Rule lab 26 × 26) and the Rule lab's 5 × 5 neighbourhood editor
//! (`.nb`, `.nb-on`, `.nb-me`).

use super::text::{galley, text_top};
use super::{hit, interact};
use ap_theme as t;
use eframe::egui::{
    CornerRadius, Id, Rect, Response, Stroke, StrokeKind, Ui, UiBuilder, accesskit::Role, pos2,
    vec2,
};
use lab_ui::paint::{advance_block, focus_ring_if};

/// One `.bit`: a mono number that toggles. Tray look by default; [`Bit::lab`] for the Rule
/// lab's 26 × 26 variant. AccessKit: a toggle `Button` named e.g. "Born with 3".
#[derive(Debug, Clone, Copy)]
pub struct Bit<'a> {
    n: usize,
    on: bool,
    lab: bool,
    name: &'a str,
}

impl<'a> Bit<'a> {
    /// Bit `n` (the number shown), currently `on`, named `name`.
    pub fn new(n: usize, on: bool, name: &'a str) -> Self {
        Self {
            n,
            on,
            lab: false,
            name,
        }
    }

    /// The Rule lab variant: `min-width: 26px; height: 26px`, radius 5, 11.5 px, no drop.
    pub fn lab(mut self) -> Self {
        self.lab = true;
        self
    }

    /// The drawn size.
    pub fn size(&self, ui: &Ui) -> eframe::egui::Vec2 {
        if self.lab {
            let w = galley(ui, &self.n.to_string(), self.font(), t::TEXT)
                .size()
                .x
                + 10.0;
            vec2(w.max(t::size::LAB_BIT), t::size::LAB_BIT)
        } else {
            vec2(t::size::BIT_W, t::size::BIT_H)
        }
    }

    fn font(&self) -> eframe::egui::FontId {
        lab_theme::font::mono_medium(if self.lab { 11.5 } else { 12.0 })
    }

    /// Draws the bit at the cursor.
    pub fn show(self, ui: &mut Ui) -> Response {
        let size = self.size(ui);
        let (id, rect) = ui.allocate_space(size);
        self.paint(ui, id, rect, hit::target(rect))
    }

    /// Draws the bit into `rect` with hit area `target`.
    pub fn paint(self, ui: &Ui, id: Id, rect: Rect, target: Rect) -> Response {
        let resp = interact(ui, id, target, true, Role::Button, self.name, Some(self.on));
        let radius = CornerRadius::same(if self.lab {
            t::radius::MINI
        } else {
            t::radius::BUTTON
        });
        let hovered = resp.hovered();
        let painter = ui.painter();
        let (fill, border, fg) = if self.on {
            (t::ACCENT, t::ACCENT, t::ON_ACCENT)
        } else if hovered {
            (t::CONTROL, t::EDGE, t::TEXT)
        } else {
            (t::CONTROL, t::BORDER_STRONG, t::TEXT_MUTED)
        };
        if self.on && !self.lab {
            // `box-shadow: 0 3px 0 #8A2F76`
            painter.rect_filled(rect.translate(vec2(0.0, 3.0)), radius, t::BIT_ON_DROP);
        }
        painter.rect(
            rect,
            radius,
            fill,
            Stroke::new(1.0, border),
            StrokeKind::Inside,
        );
        let g = galley(ui, &self.n.to_string(), self.font(), fg);
        painter.galley(
            pos2(
                rect.center().x - g.size().x / 2.0,
                text_top(rect.center().y, &g),
            ),
            g,
            fg,
        );
        focus_ring_if(ui, &resp, rect, radius);
        resp
    }
}

/// A row of bits `0..count` (Born with / Survives with). Tray: 28 × 30, 4 px apart, one row;
/// Rule lab ([`BitRow::lab`]): 26 px, 3 px apart, wrapping. Bit `n` is on when bit `n` of
/// `mask` is set; names are `"<prefix> <n>"`. Targets tile the row (no overlaps).
///
/// ```ignore
/// if let Some(n) = BitRow::new("Born with", 9, rule.birth_mask()).show(ui) { rule.toggle_birth(n) }
/// ```
#[derive(Debug, Clone, Copy)]
pub struct BitRow<'a> {
    prefix: &'a str,
    count: usize,
    mask: u32,
    lab: bool,
}

impl<'a> BitRow<'a> {
    /// Bits `0..count` (at most 32) named `"<prefix> <n>"`.
    pub fn new(prefix: &'a str, count: usize, mask: u32) -> Self {
        Self {
            prefix,
            count: count.min(32),
            mask,
            lab: false,
        }
    }

    /// The Rule lab look (26 px, 3 px gaps, wraps).
    pub fn lab(mut self) -> Self {
        self.lab = true;
        self
    }

    /// Draws the row; returns the bit clicked this frame.
    pub fn show(self, ui: &mut Ui) -> Option<usize> {
        let gap = if self.lab { 3.0 } else { 4.0 };
        let width = ui.available_width();
        let origin = ui.cursor().min;
        let names: Vec<String> = (0..self.count)
            .map(|n| format!("{} {n}", self.prefix))
            .collect();
        let bits: Vec<Bit> = (0..self.count)
            .map(|n| {
                let b = Bit::new(n, self.mask & (1 << n) != 0, &names[n]);
                if self.lab { b.lab() } else { b }
            })
            .collect();
        let mut rects = Vec::with_capacity(self.count);
        let (mut x, mut y, mut row_h) = (0.0f32, 0.0f32, 0.0f32);
        for b in &bits {
            let s = b.size(ui);
            if self.lab && x > 0.0 && x + s.x > width + 0.01 {
                x = 0.0;
                y += row_h + gap;
                row_h = 0.0;
            }
            rects.push(Rect::from_min_size(origin + vec2(x, y), s));
            x += s.x + gap;
            row_h = row_h.max(s.y);
        }
        let used = Rect::from_min_size(origin, vec2(width, y + row_h));
        let targets = hit::tile(&rects);
        let group = ui.new_child(
            UiBuilder::new()
                .id_salt(("bits", self.prefix))
                .max_rect(used)
                .accessibility_role(Role::Group)
                .accessibility_label(self.prefix),
        );
        let mut clicked = None;
        for (n, b) in bits.into_iter().enumerate() {
            let id = group.scope_id().with(n);
            if b.paint(&group, id, rects[n], targets[n]).clicked() {
                clicked = Some(n);
            }
        }
        // the Tray's 3 px drop hangs below the row
        let extra = if self.lab { 0.0 } else { 3.0 };
        advance_block(ui, used.with_max_y(used.bottom() + extra));
        clicked
    }
}

/// The Rule lab's 5 × 5 neighbourhood editor: 30 px cells 3 px apart; a neighbour on is
/// `#6B2A5C` with an accent border, the centre ("This cell") is solid accent and can't be
/// toggled. `mask` bit `(y + 2) * 5 + (x + 2)` is neighbour `(x, y)`. Names:
/// "Neighbour x, y" / "This cell".
#[derive(Debug, Clone, Copy)]
pub struct NbGrid {
    mask: u32,
}

impl NbGrid {
    /// The editor showing `mask` (25 bits, row-major from (−2, −2); the centre bit is ignored).
    pub fn new(mask: u32) -> Self {
        Self { mask }
    }

    /// The bit of neighbour `(x, y)` (`-2..=2` each).
    pub fn bit(x: i32, y: i32) -> u32 {
        1 << ((y + 2) * 5 + (x + 2))
    }

    /// Draws the grid; returns the neighbour clicked this frame.
    pub fn show(self, ui: &mut Ui) -> Option<(i32, i32)> {
        let cell = t::size::NB_CELL;
        let gap = 3.0;
        let origin = ui.cursor().min;
        let side = 5.0 * cell + 4.0 * gap;
        let used = Rect::from_min_size(origin, vec2(side, side));
        let rects: Vec<Rect> = (0..25)
            .map(|i| {
                Rect::from_min_size(
                    origin + vec2((i % 5) as f32 * (cell + gap), (i / 5) as f32 * (cell + gap)),
                    vec2(cell, cell),
                )
            })
            .collect();
        let targets = hit::tile(&rects);
        let group = ui.new_child(
            UiBuilder::new()
                .id_salt("nb-grid")
                .max_rect(used)
                .accessibility_role(Role::Group)
                .accessibility_label("Neighbourhood cells"),
        );
        let mut clicked = None;
        for i in 0..25usize {
            let (x, y) = ((i % 5) as i32 - 2, (i / 5) as i32 - 2);
            let me = x == 0 && y == 0;
            let on = self.mask & Self::bit(x, y) != 0;
            let name = if me {
                "This cell".to_string()
            } else {
                format!("Neighbour {x}, {y}")
            };
            let r = rects[i];
            let resp = interact(
                &group,
                group.scope_id().with(i),
                targets[i],
                !me,
                Role::Button,
                &name,
                Some(on && !me),
            );
            let radius = CornerRadius::same(t::radius::MINI);
            let (fill, border) = if me {
                (t::ACCENT, t::ACCENT)
            } else if on {
                (t::NB_ON, t::ACCENT)
            } else if resp.hovered() {
                (t::CONTROL, t::EDGE)
            } else {
                (t::CONTROL, t::BORDER_STRONG)
            };
            group.painter().rect(
                r,
                radius,
                fill,
                Stroke::new(1.0, border),
                StrokeKind::Inside,
            );
            focus_ring_if(&group, &resp, r, radius);
            if resp.clicked() {
                clicked = Some((x, y));
            }
        }
        advance_block(ui, used);
        clicked
    }
}
