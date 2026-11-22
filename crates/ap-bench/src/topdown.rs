//! The Rule lab's top-down view (SPEC LAB-2): the grid at the scrubbed generation on the
//! `.well` (`#060606` with 12 px lines at 6 %, 1 px `#242424` border, radius 10), one 10 × 10
//! square per cell on a 12 px pitch — born this generation in the newborn colour, alive in the
//! accent, died this generation as ghosts — scaled down to fit narrow columns
//! (`max-width: 100%; height: auto`).
//!
//! Unlike the mockup (whose background grid stays 12 px while the svg shrinks), the grid lines
//! scale with the cells so they stay aligned.

use crate::draw::{Batch, radii, rrect_outline};
use crate::projection::{Projection, View};
use crate::tokens;
use egui::{Painter, Pos2, Rect, Vec2, pos2, vec2};

/// What a cell of the top-down view shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TopCell {
    /// Dead now and in the previous generation.
    #[default]
    Empty,
    /// Born this generation (`#FFE6F7`).
    Born,
    /// Alive, and alive before (accent).
    Alive,
    /// Died this generation (`#2C1428`).
    Ghost,
}

/// Cell pitch in the well (CSS px at scale 1).
pub const PITCH: f32 = 12.0;

/// The inputs of the top-down view.
pub struct TopDownGrid<'a> {
    /// Grid size (the Rule lab's 64 × 40).
    pub cols: usize,
    pub rows: usize,
    /// What the cell at (x, y) shows.
    pub cell: &'a dyn Fn(usize, usize) -> TopCell,
}

impl<'a> TopDownGrid<'a> {
    /// A grid of `cols × rows`.
    pub fn new(cols: usize, rows: usize, cell: &'a dyn Fn(usize, usize) -> TopCell) -> Self {
        TopDownGrid { cols, rows, cell }
    }

    /// The scale (≤ 1) at which the well fits `avail_width` points.
    pub fn scale_for(&self, avail_width: f32) -> f32 {
        let natural = self.cols as f32 * PITCH;
        if natural <= 0.0 {
            return 1.0;
        }
        ((avail_width - 2.0) / natural).clamp(0.05, 1.0)
    }

    /// The well's size (border included) when fitted to `avail_width` (770 × 482 for 64 × 40
    /// at full size).
    pub fn size_for(&self, avail_width: f32) -> Vec2 {
        let s = self.scale_for(avail_width);
        vec2(self.cols as f32, self.rows as f32) * PITCH * s + Vec2::splat(2.0)
    }

    /// Geometry for a well whose top-left is at `min`, fitted to `avail_width`.
    pub fn geometry(&self, min: Pos2, avail_width: f32) -> TopDownGeometry {
        let scale = self.scale_for(avail_width);
        TopDownGeometry {
            well: Rect::from_min_size(min, self.size_for(avail_width)),
            scale,
            cols: self.cols,
            rows: self.rows,
        }
    }

    /// Paints the well at `min`, fitted to `avail_width`, and returns its geometry (advance the
    /// layout by `geometry.well`).
    pub fn paint(&self, painter: &Painter, min: Pos2, avail_width: f32) -> TopDownGeometry {
        let geo = self.geometry(min, avail_width);
        let mut b = Batch::new(painter);
        self.build(&mut b, &geo);
        b.paint(painter);
        geo
    }

    /// [`TopDownGrid::paint`] into a batch.
    pub fn build(&self, b: &mut Batch, geo: &TopDownGeometry) {
        let well = geo.well;
        let outer = Projection::flat(
            Rect::from_min_size(Pos2::ZERO, well.size()),
            View::new(well.min, 1.0),
        );
        let r = outer.local_rect();
        b.fill_rrect(&outer, r, radii(10.0), tokens::WELL.base);
        // the svg (and the background grid) at the padding box, scaled
        let s = geo.scale;
        let inner_size = vec2(self.cols as f32, self.rows as f32) * PITCH;
        let p = Projection::flat(
            Rect::from_min_size(Pos2::ZERO, inner_size),
            View::new(well.min + vec2(1.0, 1.0), s),
        );
        let line = tokens::WELL.major;
        for i in 0..self.cols {
            let x = i as f32 * PITCH + 0.5;
            b.line(&p, pos2(x, 0.0), pos2(x, inner_size.y), 1.0, line);
        }
        for j in 0..self.rows {
            let y = j as f32 * PITCH + 0.5;
            b.line(&p, pos2(0.0, y), pos2(inner_size.x, y), 1.0, line);
        }
        // overflow: hidden + border-radius 10 clips the corner cells
        let mut clip = Vec::new();
        rrect_outline(
            Rect::from_min_size(Pos2::ZERO, inner_size),
            radii(9.0 / s),
            4,
            &mut clip,
        );
        let corner = ((9.0 / s) / PITCH).ceil() as usize;
        for kind in [TopCell::Ghost, TopCell::Alive, TopCell::Born] {
            let color = match kind {
                TopCell::Ghost => tokens::GHOST_CELL,
                TopCell::Alive => tokens::ACCENT,
                TopCell::Born => tokens::NEWBORN.face,
                TopCell::Empty => continue,
            };
            for y in 0..self.rows {
                for x in 0..self.cols {
                    if (self.cell)(x, y) != kind {
                        continue;
                    }
                    let cell = Rect::from_min_size(
                        pos2(x as f32 * PITCH + 1.0, y as f32 * PITCH + 1.0),
                        vec2(10.0, 10.0),
                    );
                    let near = (x < corner || x + corner >= self.cols)
                        && (y < corner || y + corner >= self.rows);
                    if near {
                        let q = [
                            cell.left_top(),
                            cell.right_top(),
                            cell.right_bottom(),
                            cell.left_bottom(),
                        ];
                        let poly = crate::jar::clip_convex(&q, &clip);
                        if poly.len() >= 3 {
                            b.fill_poly(&p, &poly, color);
                        }
                    } else {
                        b.fill_rect(&p, cell, color);
                    }
                }
            }
        }
        b.border(&outer, r, radii(10.0), 1.0, tokens::WELL_BORDER);
    }

    /// The accessible description (SPEC A11Y-4).
    pub fn description(&self, generation: usize, alive: usize) -> String {
        crate::a11y::top_down(self.cols, self.rows, generation, alive)
    }
}

/// Screen geometry of the top-down view.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TopDownGeometry {
    /// The well's screen rect (border included).
    pub well: Rect,
    /// Screen points per CSS px of the grid (≤ 1).
    pub scale: f32,
    cols: usize,
    rows: usize,
}

impl TopDownGeometry {
    /// The cell under `screen` (the whole 12 px pitch counts), if any.
    pub fn cell_at(&self, screen: Pos2) -> Option<(usize, usize)> {
        let l = (screen - self.well.min - vec2(1.0, 1.0)) / (PITCH * self.scale);
        if l.x < 0.0 || l.y < 0.0 {
            return None;
        }
        let (x, y) = (l.x as usize, l.y as usize);
        (x < self.cols && y < self.rows).then_some((x, y))
    }

    /// The screen rect of cell (x, y)'s 10 × 10 square.
    pub fn cell_rect(&self, x: usize, y: usize) -> Rect {
        let s = self.scale;
        Rect::from_min_size(
            self.well.min
                + vec2(1.0, 1.0)
                + vec2(x as f32 * PITCH + 1.0, y as f32 * PITCH + 1.0) * s,
            Vec2::splat(10.0 * s),
        )
    }
}
