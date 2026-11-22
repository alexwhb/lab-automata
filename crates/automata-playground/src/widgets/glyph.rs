//! 3 × 3 cell-glyph icons: the Series "icon family" (grey empty, magenta live) and the Tray's
//! "8 neighbours · edges wrap" neighbourhood glyph.

use ap_theme as t;
use eframe::egui::{
    Color32, Painter, Rect, Response, Sense, Ui, WidgetInfo, accesskit::Role, pos2, vec2,
};

/// The two glyph looks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlyphStyle {
    /// Series icon family: `viewBox 0 0 27 27`, 7 px squares on a 9 px pitch, empty
    /// `#3A3A3A`, live accent.
    Icon,
    /// Tray neighbourhood glyph: `viewBox 0 0 40 40`, 11 px squares (rx 2) at 1 / 14.5 / 28,
    /// neighbours `#2A1A26`, live accent.
    Neighbourhood,
}

/// A 3 × 3 glyph: bit `y * 3 + x` of `mask` is cell `(x, y)` live.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellGlyph {
    /// Live cells, row-major.
    pub mask: u16,
    /// The look.
    pub style: GlyphStyle,
}

impl CellGlyph {
    /// The Series icon family, in the mockup's order: glider, blinker, block, ring, corners.
    pub const FAMILY: [CellGlyph; 5] = [
        CellGlyph::icon(&[(1, 0), (2, 1), (0, 2), (1, 2), (2, 2)]),
        CellGlyph::icon(&[(0, 1), (1, 1), (2, 1)]),
        CellGlyph::icon(&[(0, 0), (1, 0), (0, 1), (1, 1)]),
        CellGlyph::icon(&[
            (0, 0),
            (1, 0),
            (2, 0),
            (0, 1),
            (2, 1),
            (0, 2),
            (1, 2),
            (2, 2),
        ]),
        CellGlyph::icon(&[(0, 0), (2, 0), (0, 2), (2, 2)]),
    ];

    /// The Tray's Moore neighbourhood glyph (centre live, eight neighbours around it).
    pub const MOORE: CellGlyph = CellGlyph {
        mask: 1 << 4,
        style: GlyphStyle::Neighbourhood,
    };

    /// An icon-family glyph with these live cells.
    pub const fn icon(live: &[(u8, u8)]) -> Self {
        let mut mask = 0u16;
        let mut i = 0;
        while i < live.len() {
            mask |= 1 << (live[i].1 * 3 + live[i].0);
            i += 1;
        }
        Self {
            mask,
            style: GlyphStyle::Icon,
        }
    }

    /// Is cell `(x, y)` live?
    pub fn live(&self, x: u8, y: u8) -> bool {
        x < 3 && y < 3 && self.mask & (1 << (y * 3 + x)) != 0
    }

    /// Paints the glyph scaled into `rect` (square, centred).
    pub fn paint(&self, painter: &Painter, rect: Rect) {
        let (view, cell, pitch, origin, radius, off): (f32, f32, f32, f32, f32, Color32) =
            match self.style {
                GlyphStyle::Icon => (27.0, 7.0, 9.0, 1.0, 0.0, t::GLYPH_OFF),
                GlyphStyle::Neighbourhood => (40.0, 11.0, 13.5, 1.0, 2.0, t::NB_GLYPH_OFF),
            };
        let s = rect.width().min(rect.height()) / view;
        let o = rect.center() - vec2(view, view) * s / 2.0;
        for y in 0..3u8 {
            for x in 0..3u8 {
                let r = Rect::from_min_size(
                    o + vec2(origin + x as f32 * pitch, origin + y as f32 * pitch) * s,
                    vec2(cell, cell) * s,
                );
                let c = if self.live(x, y) { t::ACCENT } else { off };
                painter.rect_filled(r, radius * s, c);
            }
        }
    }

    /// Allocates `size` × `size` and paints the glyph; AccessKit: an `Image` named `label`.
    pub fn show(&self, ui: &mut Ui, size: f32, label: &str) -> Response {
        let (rect, resp) = ui.allocate_exact_size(vec2(size, size), Sense::hover());
        self.paint(ui.painter(), rect);
        let l = label.to_string();
        resp.widget_info(|| WidgetInfo::labeled(Role::Image, true, &l));
        let _ = pos2(0.0, 0.0);
        resp
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn family_matches_the_mockup() {
        let glider = CellGlyph::FAMILY[0];
        assert!(glider.live(1, 0) && glider.live(2, 1) && glider.live(0, 2));
        assert!(!glider.live(0, 0));
        assert_eq!(
            CellGlyph::FAMILY.map(|g| g.mask.count_ones()),
            [5, 3, 4, 8, 4]
        );
        assert!(CellGlyph::MOORE.live(1, 1) && !CellGlyph::MOORE.live(0, 0));
    }
}
