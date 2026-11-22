//! A terrarium jar (Terrarium · breed, SPEC TERR-1): a 32 × 32 torus of cells in a jar tilted
//! `perspective(900px) rotateX(52deg) scale(.86)` (origin 50% 70%), gently bobbing over its
//! blurred shadow.
//!
//! Layout follows `docs/design/mockup/Terrarium.dc.html`: inside the card's 272 × 228 jar area,
//! the shadow at (26, 196) 220 × 30 blurred 10 px, then the 272 × 272 jar element (moved by the
//! bob) with its body, 16 px slab, inner ring, the cells in a 256 × 256 svg at (8, 8) clipped
//! to radius 20, and the glint at (14, 10) 120 × 46.
//!
//! The mockup's slab overflows its 228 px area and covers the rule code below it; reserve
//! [`JarScene::FOOTPRINT`] instead (SPEC TERR-1: fix the overlap).

use crate::draw::{Batch, radii, rrect_outline};
use crate::projection::{Projection, Tilt, View};
use crate::stage;
use crate::tokens;
use egui::{Color32, Painter, Pos2, Rect, Vec2, pos2, vec2};

/// The jar's transform.
pub const JAR_TILT: Tilt = Tilt::new(900.0, 52.0, 0.86, Vec2::new(0.5, 0.7));
/// The jar element's untransformed size.
pub const JAR_SIZE: Vec2 = Vec2::new(272.0, 272.0);
/// The cell svg's box inside the jar.
const CELLS: Rect = Rect {
    min: Pos2::new(8.0, 8.0),
    max: Pos2::new(264.0, 264.0),
};

/// The inputs of one jar.
pub struct JarScene<'a> {
    /// Cells per side (32).
    pub size: usize,
    /// Whether the cell at (x, y) is alive.
    pub cell: &'a dyn Fn(usize, usize) -> bool,
    /// The jar's ink (`ap_theme::JAR_INKS` by jar index).
    pub ink: Color32,
    /// The bob offset in CSS px (0 … −3, [`crate::anim::jar_bob`]).
    pub bob: f32,
}

impl<'a> JarScene<'a> {
    /// The mockup's jar area inside the card (272 × 228).
    pub const AREA: Vec2 = Vec2::new(272.0, 228.0);
    /// The area that actually contains the jar, its slab and its shadow (272 × 250): reserve
    /// this so the rule code below isn't covered.
    pub const FOOTPRINT: Vec2 = Vec2::new(272.0, 250.0);

    /// A 32 × 32 jar.
    pub fn new(cell: &'a dyn Fn(usize, usize) -> bool, ink: Color32) -> Self {
        JarScene {
            size: 32,
            cell,
            ink,
            bob: 0.0,
        }
    }

    /// The jar's projection for a jar area whose top-left is at `area.min`; the area's width
    /// sets the zoom (`area.width() / 272`).
    pub fn projection(&self, area: Rect) -> Projection {
        let view = View::new(area.min, area.width() / JAR_SIZE.x);
        Projection::new(
            JAR_TILT,
            Rect::from_min_size(pos2(0.0, self.bob), JAR_SIZE),
            view,
        )
    }

    /// Paints the shadow and the jar into `area` (the card's jar area; give it
    /// [`JarScene::FOOTPRINT`]'s aspect to keep the code below clear). Returns the projection.
    pub fn paint(&self, painter: &Painter, area: Rect) -> Projection {
        let mut b = Batch::new(painter);
        let p = self.build(&mut b, area);
        b.paint(painter);
        p
    }

    /// [`JarScene::paint`] into a batch.
    pub fn build(&self, b: &mut Batch, area: Rect) -> Projection {
        let view = View::new(area.min, area.width() / JAR_SIZE.x);
        stage::contact_shadow(
            b,
            Rect::from_min_size(pos2(26.0, 196.0), vec2(220.0, 30.0)),
            10.0,
            tokens::JAR_SHADOW,
            view,
        );
        let p = self.projection(area);
        let r = p.local_rect();
        // box-shadow: 0 16px 0 #0B0C0E, 0 16px 0 2px #2E3138, inset 0 0 0 6px #17181D
        let slab = r.translate(vec2(0.0, 16.0));
        b.fill_rrect(&p, slab.expand(2.0), radii(28.0), tokens::JAR_EDGE);
        b.fill_rrect(&p, slab, radii(26.0), tokens::JAR_SLAB);
        b.fill_rrect(&p, r, radii(26.0), tokens::JAR_BODY);
        b.border(&p, r, radii(26.0), 2.0, tokens::JAR_EDGE);
        b.ring(
            &p,
            r.shrink(2.0),
            radii(24.0),
            r.shrink(8.0),
            radii(18.0),
            tokens::JAR_RING,
        );
        // cells: 8 px pitch, 7 × 7 at +.5, clipped to the svg's 20 px corners
        let n = self.size.max(1);
        let pitch = CELLS.width() / n as f32;
        let mut clip = Vec::new();
        rrect_outline(CELLS, radii(20.0), 6, &mut clip);
        let corner = (20.0 / pitch).ceil() as usize;
        for y in 0..n {
            for x in 0..n {
                if !(self.cell)(x, y) {
                    continue;
                }
                let cell = Rect::from_min_size(
                    CELLS.min + vec2(x as f32 * pitch + 0.5, y as f32 * pitch + 0.5),
                    Vec2::splat(pitch - 1.0),
                );
                let near_corner =
                    (x < corner || x >= n - corner) && (y < corner || y >= n - corner);
                if near_corner {
                    let poly = clip_convex(&cell_corners(cell), &clip);
                    if poly.len() >= 3 {
                        b.fill_poly(&p, &poly, self.ink);
                    }
                } else {
                    b.fill_rect(&p, cell, self.ink);
                }
            }
        }
        // the glint: (14, 10) 120 × 46, radius 20,
        // linear-gradient(120deg, rgba(255,255,255,.08), rgba(255,255,255,0))
        let glint = Rect::from_min_size(pos2(14.0, 10.0), vec2(120.0, 46.0));
        let a = 120f32.to_radians();
        let dir = vec2(a.sin(), -a.cos());
        let len = glint.width() * a.sin().abs() + glint.height() * a.cos().abs();
        let c = glint.center();
        b.fill_rrect_with(&p, glint, radii(20.0), |q| {
            let t = ((q - c).dot(dir) / len + 0.5).clamp(0.0, 1.0);
            tokens::JAR_GLINT.gamma_multiply(1.0 - t)
        });
        p
    }

    /// The accessible description of a jar ("Jar running rule B3/S23, 122 cells alive").
    pub fn description(code: &str, alive: usize) -> String {
        crate::a11y::jar(code, alive)
    }
}

fn cell_corners(r: Rect) -> [Pos2; 4] {
    [
        r.left_top(),
        r.right_top(),
        r.right_bottom(),
        r.left_bottom(),
    ]
}

/// Sutherland–Hodgman: `subject` clipped to the convex, clockwise polygon `clip`.
pub(crate) fn clip_convex(subject: &[Pos2], clip: &[Pos2]) -> Vec<Pos2> {
    let mut out: Vec<Pos2> = subject.to_vec();
    let n = clip.len();
    for i in 0..n {
        if out.is_empty() {
            break;
        }
        let a = clip[i];
        let b = clip[(i + 1) % n];
        // inside = right of a→b on a y-down screen for a clockwise polygon
        let inside = |p: Pos2| (b.x - a.x) * (p.y - a.y) - (b.y - a.y) * (p.x - a.x) >= 0.0;
        let input = std::mem::take(&mut out);
        for j in 0..input.len() {
            let cur = input[j];
            let prev = input[(j + input.len() - 1) % input.len()];
            let (ci, pi) = (inside(cur), inside(prev));
            if ci {
                if !pi {
                    out.push(intersect(prev, cur, a, b));
                }
                out.push(cur);
            } else if pi {
                out.push(intersect(prev, cur, a, b));
            }
        }
    }
    out
}

fn intersect(p: Pos2, q: Pos2, a: Pos2, b: Pos2) -> Pos2 {
    let d1 = q - p;
    let d2 = b - a;
    let den = d1.x * d2.y - d1.y * d2.x;
    if den.abs() < 1e-9 {
        return q;
    }
    let t = ((a.x - p.x) * d2.y - (a.y - p.y) * d2.x) / den;
    p + d1 * t
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clipping_a_square_by_a_square() {
        let a = cell_corners(Rect::from_min_size(pos2(0.0, 0.0), vec2(10.0, 10.0)));
        let b = cell_corners(Rect::from_min_size(pos2(5.0, 5.0), vec2(10.0, 10.0)));
        let c = clip_convex(&a, &b);
        let bb = Rect::from_points(&c);
        assert!(
            (bb.min - pos2(5.0, 5.0)).length() < 1e-4
                && (bb.max - pos2(10.0, 10.0)).length() < 1e-4
        );
    }

    #[test]
    fn footprint_contains_the_jar_slab_and_shadow() {
        let none = |_: usize, _: usize| false;
        for bob in [0.0, -3.0] {
            let scene = JarScene {
                bob,
                ..JarScene::new(&none, Color32::WHITE)
            };
            let area = Rect::from_min_size(Pos2::ZERO, JarScene::FOOTPRINT);
            let p = scene.projection(area);
            let slab = p.bounds(p.local_rect().translate(vec2(0.0, 16.0)).expand(2.0));
            assert!(slab.bottom() <= JarScene::FOOTPRINT.y, "{slab:?}");
            assert!(slab.left() >= 0.0 && slab.right() <= JarScene::FOOTPRINT.x);
        }
        // the shadow's visible extent (≈ 2σ below its ellipse) also fits
        const { assert!(196.0 + 30.0 + 20.0 <= JarScene::FOOTPRINT.y) };
        // and the mockup's 228 px area does not hold the slab (the bug we fix)
        let area = Rect::from_min_size(Pos2::ZERO, JarScene::AREA);
        let scene = JarScene::new(&none, Color32::WHITE);
        let p = scene.projection(area);
        assert!(p.bounds(p.local_rect().translate(vec2(0.0, 16.0))).bottom() > JarScene::AREA.y);
    }
}
