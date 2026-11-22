//! List tiles: the Tray's rule cartridges (`.cart`) and stamps (`.stamp`), and the Loom's
//! famous threads (`.preset`).

use super::text::{ellipsized, galley, text_top, wrapped};
use super::{hit, interact};
use ap_theme as t;
use eframe::egui::{
    Color32, CornerRadius, Rect, Response, Stroke, StrokeKind, Ui, accesskit::Role, pos2, vec2,
};
use lab_ui::paint::focus_ring_if;

/// `.cart` — a rule cartridge (Tray "Rules"): the little cartridge body (34 × 26, radius
/// 5 5 3 3, a darker 3 px bottom inset, a 4 px side in [`shade`](ap_theme::shade)`(body, .45)`
/// and the label strip), name (13 px 600, wraps) over note (11.5 px muted, ellipsis), and the
/// rule code (mono 10.5) at the right. Full width; hover: edge border, `#1A1018`; on: accent
/// border, tint. AccessKit: a toggle `Button` named by `name` (pressed = current rule).
#[must_use = "call .show(ui)"]
pub struct Cartridge<'a> {
    name: &'a str,
    note: &'a str,
    code: &'a str,
    body: Color32,
    on: bool,
}

impl<'a> Cartridge<'a> {
    /// A cartridge for rule `code` called `name`, with a short `note` and a `body` colour
    /// ([`ap_theme::CART_BODIES`]).
    pub fn new(name: &'a str, note: &'a str, code: &'a str, body: Color32) -> Self {
        Self {
            name,
            note,
            code,
            body,
            on: false,
        }
    }

    /// Highlighted (`.cart-on`: this is the current rule).
    pub fn on(mut self, on: bool) -> Self {
        self.on = on;
        self
    }

    /// Draws it full width.
    pub fn show(self, ui: &mut Ui) -> Response {
        let width = ui.available_width();
        let code_g = galley(ui, self.code, lab_theme::font::mono(10.5), t::TEXT_DIM);
        // padding 8 10, gaps 10: body 34 | text (flex 1) | code
        let text_w = (width - 2.0 - 20.0 - 34.0 - 10.0 - 10.0 - code_g.size().x).max(20.0);
        let name_g = wrapped(
            ui,
            self.name,
            lab_theme::font::semibold(13.0),
            t::TEXT,
            text_w,
            None,
        );
        let note_g = ellipsized(
            ui,
            self.note,
            lab_theme::font::body(11.5),
            t::TEXT_MUTED,
            text_w,
        );
        let text_h = name_g.size().y + 2.0 + note_g.size().y;
        let inner_h = text_h.max(26.0 + 4.0);
        let size = vec2(width, inner_h + 16.0 + 2.0);
        let (id, rect) = ui.allocate_space(size);
        let resp = interact(
            ui,
            id,
            hit::target(rect),
            true,
            Role::Button,
            self.name,
            Some(self.on),
        );
        let hovered = resp.hovered();
        let (fill, border) = if self.on {
            (t::TINT, t::ACCENT)
        } else if hovered {
            (t::TILE_HOVER, t::EDGE)
        } else {
            (t::TILE, t::BORDER)
        };
        let painter = ui.painter();
        let radius = CornerRadius::same(t::radius::CARD);
        painter.rect(
            rect,
            radius,
            fill,
            Stroke::new(1.0, border),
            StrokeKind::Inside,
        );
        let cy = rect.center().y;
        // the cartridge body (align-items: center → centred on the row)
        let body = Rect::from_min_size(pos2(rect.left() + 11.0, cy - 13.0), vec2(34.0, 26.0));
        let body_r = CornerRadius {
            nw: 5,
            ne: 5,
            sw: 3,
            se: 3,
        };
        painter.rect_filled(
            body.translate(vec2(0.0, 4.0)),
            body_r,
            t::shade(self.body, 0.45),
        );
        painter.rect_filled(body, body_r, self.body);
        // `inset 0 -3px 0 rgba(0,0,0,.25)`
        let inset = Rect::from_min_max(pos2(body.left(), body.bottom() - 3.0), body.max);
        painter.rect_filled(
            inset,
            CornerRadius {
                nw: 0,
                ne: 0,
                sw: 3,
                se: 3,
            },
            Color32::from_black_alpha(64),
        );
        let strip = Rect::from_min_size(
            pos2(body.center().x - 10.0, body.top() + 4.0),
            vec2(20.0, 6.0),
        );
        painter.rect_filled(strip, 2.0, ap_theme::rgba(10, 10, 10, 0.55));
        // text
        let tx = body.right() + 10.0;
        let ty = cy - text_h / 2.0;
        let note_y = ty + name_g.size().y + 2.0;
        painter.galley(pos2(tx, ty), name_g, t::TEXT);
        painter.galley(pos2(tx, note_y), note_g, t::TEXT_MUTED);
        painter.galley(
            pos2(rect.right() - 11.0 - code_g.size().x, text_top(cy, &code_g)),
            code_g,
            t::TEXT_DIM,
        );
        focus_ring_if(ui, &resp, rect, radius);
        resp
    }
}

/// `.stamp` — a stamp tile: the pattern preview (its live cells as 4 px squares on a 5 px
/// pitch, fitted into 62 × 34 like the mockup's `viewBox` with 3 px padding) above the name
/// (11.5 px 600). `min-height: 78px`; hover: edge border; on: accent border, tint, ink name,
/// accent cells. AccessKit: a toggle `Button` named by `name`.
#[must_use = "call .show(ui)"]
pub struct StampTile<'a> {
    name: &'a str,
    cells: &'a [(i32, i32)],
    on: bool,
}

impl<'a> StampTile<'a> {
    /// A tile for the pattern whose live cells are `cells` (`(x, y)`, from 0).
    pub fn new(name: &'a str, cells: &'a [(i32, i32)]) -> Self {
        Self {
            name,
            cells,
            on: false,
        }
    }

    /// Selected (`.stamp-on`).
    pub fn on(mut self, on: bool) -> Self {
        self.on = on;
        self
    }

    /// Draws it full width.
    pub fn show(self, ui: &mut Ui) -> Response {
        let width = ui.available_width();
        let fg = if self.on { t::INK } else { t::TEXT };
        let name_g = wrapped(
            ui,
            self.name,
            lab_theme::font::semibold(11.5),
            fg,
            width - 10.0,
            Some(13.8),
        );
        // padding 8 4, gap 6, preview 34
        let h = (8.0 + 34.0 + 6.0 + name_g.size().y + 8.0 + 2.0).max(78.0);
        let (id, rect) = ui.allocate_space(vec2(width, h));
        let resp = interact(
            ui,
            id,
            hit::target(rect),
            true,
            Role::Button,
            self.name,
            Some(self.on),
        );
        let (fill, border) = if self.on {
            (t::TINT, t::ACCENT)
        } else if resp.hovered() {
            (t::TILE_HOVER, t::EDGE)
        } else {
            (t::TILE, t::BORDER)
        };
        let painter = ui.painter();
        let radius = CornerRadius::same(t::radius::CARD);
        painter.rect(
            rect,
            radius,
            fill,
            Stroke::new(1.0, border),
            StrokeKind::Inside,
        );
        let content_h = 34.0 + 6.0 + name_g.size().y;
        let top = rect.center().y - content_h / 2.0;
        let preview = Rect::from_center_size(pos2(rect.center().x, top + 17.0), vec2(62.0, 34.0));
        paint_pattern(
            painter,
            preview,
            self.cells,
            if self.on { t::ACCENT } else { t::TEXT_DIM },
        );
        painter.galley(
            pos2(rect.center().x - name_g.size().x / 2.0, top + 40.0),
            name_g,
            fg,
        );
        focus_ring_if(ui, &resp, rect, radius);
        resp
    }
}

/// Paints a pattern's cells like the mockup's stamp preview: 4 px cells on a 5 px pitch in a
/// `viewBox` of at least 18 × 12 plus 3 px padding, scaled to fit `rect` (`xMidYMid meet`).
pub fn paint_pattern(
    painter: &eframe::egui::Painter,
    rect: Rect,
    cells: &[(i32, i32)],
    color: Color32,
) {
    let w = cells.iter().map(|c| c.0 + 1).max().unwrap_or(0).max(0) as f32;
    let h = cells.iter().map(|c| c.1 + 1).max().unwrap_or(0).max(0) as f32;
    let vw = (w * 5.0).max(18.0) + 6.0;
    let vh = (h * 5.0).max(12.0) + 6.0;
    let scale = (rect.width() / vw).min(rect.height() / vh);
    // the pattern's own box (w·5 × h·5) is centred in the viewBox
    let origin = rect.center() - vec2(w * 5.0, h * 5.0) * scale / 2.0;
    for &(x, y) in cells {
        let r = Rect::from_min_size(
            origin + vec2(x as f32 * 5.0, y as f32 * 5.0) * scale,
            vec2(4.0, 4.0) * scale,
        );
        painter.rect_filled(r, 0.0, color);
    }
}

/// `.preset` — a Loom famous thread: the rule number (mono 500 14, ink, 34 px column), name
/// (13 px 600) over note (11.5 px muted). Full width, `padding: 7px 10px`, radius 8, `#111`;
/// hover: edge border; on: accent border, tint. AccessKit: a toggle `Button` named
/// `"<number> <name>"` ("30 Chaos").
#[must_use = "call .show(ui)"]
pub struct Preset<'a> {
    number: &'a str,
    name: &'a str,
    note: &'a str,
    on: bool,
}

impl<'a> Preset<'a> {
    /// A preset row.
    pub fn new(number: &'a str, name: &'a str, note: &'a str) -> Self {
        Self {
            number,
            name,
            note,
            on: false,
        }
    }

    /// Highlighted (the current rule).
    pub fn on(mut self, on: bool) -> Self {
        self.on = on;
        self
    }

    /// Draws it full width.
    pub fn show(self, ui: &mut Ui) -> Response {
        let width = ui.available_width();
        let num_g = galley(ui, self.number, lab_theme::font::mono_medium(14.0), t::INK);
        let text_w = (width - 2.0 - 20.0 - 34.0 - 10.0).max(20.0);
        let name_g = wrapped(
            ui,
            self.name,
            lab_theme::font::semibold(13.0),
            t::TEXT,
            text_w,
            None,
        );
        let note_g = wrapped(
            ui,
            self.note,
            lab_theme::font::body(11.5),
            t::TEXT_MUTED,
            text_w,
            None,
        );
        let text_h = name_g.size().y + 1.0 + note_g.size().y;
        let size = vec2(width, text_h.max(num_g.size().y) + 14.0 + 2.0);
        let (id, rect) = ui.allocate_space(size);
        let label = format!("{} {}", self.number, self.name);
        let resp = interact(
            ui,
            id,
            hit::target(rect),
            true,
            Role::Button,
            &label,
            Some(self.on),
        );
        let (fill, border) = if self.on {
            (t::TINT, t::ACCENT)
        } else if resp.hovered() {
            (t::CARD, t::EDGE)
        } else {
            (t::CARD, t::BORDER)
        };
        let painter = ui.painter();
        let radius = CornerRadius::same(t::radius::WELL);
        painter.rect(
            rect,
            radius,
            fill,
            Stroke::new(1.0, border),
            StrokeKind::Inside,
        );
        let cy = rect.center().y;
        painter.galley(
            pos2(rect.left() + 11.0, text_top(cy, &num_g)),
            num_g,
            t::INK,
        );
        let tx = rect.left() + 11.0 + 34.0 + 10.0;
        let ty = cy - text_h / 2.0;
        let ny = ty + name_g.size().y + 1.0;
        painter.galley(pos2(tx, ty), name_g, t::TEXT);
        painter.galley(pos2(tx, ny), note_g, t::TEXT_MUTED);
        focus_ring_if(ui, &resp, rect, radius);
        resp
    }
}
