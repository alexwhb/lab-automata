//! The ground every bench scene stands on: the `.graph` stage (flat graph paper), the tilted
//! `.floor` plane with its projected grid, the dark band over the floor's far edge and the
//! blurred contact shadows.

use crate::draw::Batch;
use crate::projection::{Projection, Tilt, View};
use crate::tokens;
use ap_theme::Grid;
use egui::Color32;
use egui::{Painter, Pos2, Rect, pos2, vec2};

/// The floor's transform: `perspective(900px) rotateX(62deg)`, `transform-origin: 50% 0`.
pub const FLOOR_TILT: Tilt = Tilt::new(900.0, 62.0, 1.0, egui::Vec2::new(0.5, 0.0));

/// Paints the `.graph` stage — `#0B0C0E` with 120 px / 24 px lines — into `rect`, anchored at
/// its top-left corner. Screens call this first; the scenes paint over it.
pub fn stage(painter: &Painter, rect: Rect) {
    lab_ui::paint_grid(painter, rect, rect.min, &ap_theme::GRAPH);
}

/// Paints the tilted `.floor` (`floor_rect` in stage CSS px, e.g. the Tray's
/// `left: -360px; top: 300px; 1560 × 760` inside its container) seen through `view`, clipped
/// to the screen rect `clip` (the stage, `overflow: hidden`).
pub fn floor(batch: &mut Batch, floor_rect: Rect, view: View, clip: Rect) {
    let p = Projection::new(FLOOR_TILT, floor_rect, view);
    paint_plane_grid(batch, &p, &tokens::FLOOR, clip);
}

/// Paints a graph-paper plane (base colour + CSS 1 px grid lines anchored at the element's
/// top-left) on a projection, down to the screen row `clip.bottom()`.
pub fn paint_plane_grid(batch: &mut Batch, p: &Projection, grid: &Grid, clip: Rect) {
    let size = p.size();
    let v_max = p
        .local_y_at_screen_y(clip.bottom() + 2.0)
        .map_or(size.y, |v| v.clamp(0.0, size.y));
    if v_max <= 0.0 {
        return;
    }
    batch.fill_rect(
        p,
        Rect::from_min_size(Pos2::ZERO, vec2(size.x, v_max)),
        grid.base,
    );
    // horizontal extent worth drawing: unproject the clip's bottom corners
    let (u0, u1) = {
        let bl = p.unproject(pos2(clip.left() - 2.0, clip.bottom()));
        let br = p.unproject(pos2(clip.right() + 2.0, clip.bottom()));
        let tl = p.unproject(pos2(clip.left() - 2.0, p.project(Pos2::ZERO).y));
        let tr = p.unproject(pos2(clip.right() + 2.0, p.project(Pos2::ZERO).y));
        let xs = [bl, br, tl, tr].into_iter().flatten().map(|q| q.x);
        let (lo, hi) = xs.fold((f32::MAX, f32::MIN), |(a, b), x| (a.min(x), b.max(x)));
        if lo > hi {
            (0.0, size.x)
        } else {
            (lo.max(0.0), hi.min(size.x))
        }
    };
    // CSS lists major-h, major-v, minor-h, minor-v top to bottom: paint minor first
    for (step, color) in [(grid.minor_px, grid.minor), (grid.major_px, grid.major)] {
        if step <= 0.0 || color.a() == 0 {
            continue;
        }
        // vertical lines: [x, x + 1] × [0, v_max]
        let mut x = (u0 / step).floor().max(0.0) * step;
        while x < u1.min(size.x) {
            batch.line(p, pos2(x + 0.5, 0.0), pos2(x + 0.5, v_max), 1.0, color);
            x += step;
        }
        // horizontal lines: [u0, u1] × [y, y + 1]
        let mut y = 0.0;
        while y < v_max {
            batch.line(p, pos2(u0, y + 0.5), pos2(u1, y + 0.5), 1.0, color);
            y += step;
        }
    }
}

/// The band over the floor's far edge: `linear-gradient(rgba(11,12,14,0), rgba(11,12,14,.8))`
/// filling `band` (stage CSS px).
pub fn floor_band(batch: &mut Batch, band: Rect, view: View) {
    let p = Projection::flat(band, view);
    let local = Rect::from_min_size(Pos2::ZERO, band.size());
    batch.fill_rect_vgradient(&p, local, Color32::TRANSPARENT, tokens::FLOOR_BAND);
}

/// A contact shadow: an ellipse filling `r` (stage CSS px) with `color`, blurred by
/// `filter: blur(<blur>px)` (a Gaussian with σ = blur).
pub fn contact_shadow(batch: &mut Batch, r: Rect, blur: f32, color: Color32, view: View) {
    let p = Projection::flat(r, view);
    batch.blurred_ellipse(&p, Rect::from_min_size(Pos2::ZERO, r.size()), blur, color);
}
