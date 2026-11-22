//! CSS `transform: perspective(P) rotateX(θ) scale(s)` with a `transform-origin`, as an exact
//! forward map (element-local px → screen) and its inverse (screen → element-local px, by
//! intersecting the view ray with the tilted plane).
//!
//! # The maths (CSS semantics)
//!
//! An element whose untransformed border box is `rect` (stage CSS px) has its transform applied
//! about the origin `o = rect.min + origin_frac · size`. For an element-local point `(x, y)`:
//!
//! ```text
//! u  = (x − ox)·s,  v = (y − oy)·s          scale(s) about the origin
//! y' = v·cos θ,     z' = v·sin θ             rotateX(θ): CSS y points down, z toward the viewer,
//!                                            so a positive θ tips the top edge away
//! w  = 1 − z'/P                              perspective(P): homogeneous divide
//! stage = o + (u, y') / w
//! screen = view.origin + view.zoom · stage
//! ```
//!
//! The inverse, for a screen point with stage offset `(X, Y)` from `o`:
//! `Y = v·cos θ / (1 − v·sin θ/P)` ⇒ `v = Y / (cos θ + Y·sin θ/P)`, then `w = 1 − v·sin θ/P`,
//! `u = X·w`, and `x = ox + u/s`, `y = oy + v/s`. Points on or above the horizon
//! (`cos θ + Y·sin θ/P ≤ 0`) or behind the eye (`w ≤ 0`) have no preimage.
//!
//! Validated against Chrome's own layout (`tests/chrome_geometry.rs`, data recorded by
//! `scripts/chrome-geometry.mjs`).

use egui::{Pos2, Rect, Vec2, pos2};

/// The parameters of a CSS `perspective(P) rotateX(θ) scale(s)` transform and its
/// `transform-origin` (as fractions of the element's size, CSS `50% 100%` = `(0.5, 1.0)`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tilt {
    /// `perspective(P)` in CSS px.
    pub perspective: f32,
    /// `rotateX(θ)` in degrees (positive tips the top edge away from the viewer).
    pub angle_deg: f32,
    /// `scale(s)`.
    pub scale: f32,
    /// `transform-origin` as fractions of the element's width and height.
    pub origin: Vec2,
}

impl Tilt {
    /// The identity transform (origin at the centre).
    pub const IDENTITY: Tilt = Tilt {
        perspective: 1.0e6,
        angle_deg: 0.0,
        scale: 1.0,
        origin: Vec2::new(0.5, 0.5),
    };

    /// `perspective(P) rotateX(θ) scale(s)` about `origin` (fractions of the size).
    pub const fn new(perspective: f32, angle_deg: f32, scale: f32, origin: Vec2) -> Tilt {
        Tilt {
            perspective,
            angle_deg,
            scale,
            origin,
        }
    }

    /// CSS transition interpolation between two transforms with the same function list:
    /// each function's arguments interpolate linearly with the (eased) progress `t`.
    pub fn lerp(a: Tilt, b: Tilt, t: f32) -> Tilt {
        let l = |p: f32, q: f32| p + (q - p) * t;
        Tilt {
            perspective: l(a.perspective, b.perspective),
            angle_deg: l(a.angle_deg, b.angle_deg),
            scale: l(a.scale, b.scale),
            origin: a.origin + (b.origin - a.origin) * t,
        }
    }
}

/// Maps stage CSS px to screen points: `screen = origin + zoom · stage`.
///
/// `origin` is where the stage's top-left corner lands on screen; `zoom` shrinks a whole scene
/// on narrow windows (1 = one CSS px per point, as at 1440 × 900).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct View {
    /// Screen position of stage CSS (0, 0).
    pub origin: Pos2,
    /// Screen points per stage CSS px.
    pub zoom: f32,
}

impl View {
    /// A view of a stage whose top-left corner is at `origin`, at `zoom`.
    pub const fn new(origin: Pos2, zoom: f32) -> View {
        View { origin, zoom }
    }

    /// Stage CSS px → screen.
    #[inline]
    pub fn to_screen(&self, p: Pos2) -> Pos2 {
        self.origin + p.to_vec2() * self.zoom
    }

    /// Screen → stage CSS px.
    #[inline]
    pub fn to_stage(&self, p: Pos2) -> Pos2 {
        let z = if self.zoom.abs() > 1e-6 {
            self.zoom
        } else {
            1.0
        };
        pos2((p.x - self.origin.x) / z, (p.y - self.origin.y) / z)
    }

    /// A stage-CSS rect → screen rect.
    pub fn rect_to_screen(&self, r: Rect) -> Rect {
        Rect::from_min_max(self.to_screen(r.min), self.to_screen(r.max))
    }

    /// The view of an element-local coordinate system whose origin sits at stage `offset`
    /// (an untransformed child box).
    pub fn offset(&self, offset: Vec2) -> View {
        View {
            origin: self.to_screen(offset.to_pos2()),
            zoom: self.zoom,
        }
    }
}

/// A transformed element: its [`Tilt`], its untransformed border box in stage CSS px, and the
/// [`View`] that puts the stage on screen. Maps element-local CSS px (from the box's top-left)
/// to screen points and back.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Projection {
    tilt: Tilt,
    rect: Rect,
    view: View,
    // derived
    sin: f32,
    cos: f32,
    /// sin θ / P
    k: f32,
    /// transform origin, element-local
    o_local: Vec2,
    /// transform origin on screen
    o_screen: Pos2,
}

impl Projection {
    /// The projection of an element with untransformed border box `rect` (stage CSS px).
    pub fn new(tilt: Tilt, rect: Rect, view: View) -> Projection {
        let (sin, cos) = tilt.angle_deg.to_radians().sin_cos();
        let p = if tilt.perspective.abs() > 1e-3 {
            tilt.perspective
        } else {
            1e-3
        };
        let o_local = rect.size() * tilt.origin;
        let o_screen = view.to_screen(rect.min + o_local);
        Projection {
            tilt,
            rect,
            view,
            sin,
            cos,
            k: sin / p,
            o_local,
            o_screen,
        }
    }

    /// An untransformed element (identity tilt) at `rect`.
    pub fn flat(rect: Rect, view: View) -> Projection {
        Projection::new(Tilt::IDENTITY, rect, view)
    }

    /// The transform.
    pub fn tilt(&self) -> Tilt {
        self.tilt
    }

    /// The element's untransformed border box in stage CSS px.
    pub fn rect(&self) -> Rect {
        self.rect
    }

    /// The element's size in CSS px.
    pub fn size(&self) -> Vec2 {
        self.rect.size()
    }

    /// The stage → screen view.
    pub fn view(&self) -> View {
        self.view
    }

    /// The element-local rect `[0, size]`.
    pub fn local_rect(&self) -> Rect {
        Rect::from_min_size(Pos2::ZERO, self.rect.size())
    }

    /// The homogeneous `w` of an element-local point (1 at the origin's row, > 1 farther away,
    /// < 1 nearer). Screen distances at that point are magnified by `zoom · s / w`.
    #[inline]
    pub fn w(&self, local: Pos2) -> f32 {
        1.0 - (local.y - self.o_local.y) * self.tilt.scale * self.k
    }

    /// Screen points per element-local px, horizontally, at `local` (the row's magnification).
    #[inline]
    pub fn magnification(&self, local: Pos2) -> f32 {
        self.view.zoom * self.tilt.scale / self.w(local).max(1e-4)
    }

    /// Element-local CSS px → screen point. Points behind the eye are clamped to a tiny
    /// positive `w` (scenes never put geometry there).
    #[inline]
    pub fn project(&self, local: Pos2) -> Pos2 {
        let s = self.tilt.scale;
        let u = (local.x - self.o_local.x) * s;
        let v = (local.y - self.o_local.y) * s;
        let w = (1.0 - v * self.k).max(1e-4);
        let f = self.view.zoom / w;
        pos2(self.o_screen.x + u * f, self.o_screen.y + v * self.cos * f)
    }

    /// Like [`Projection::project`] but `None` for points behind the eye.
    pub fn try_project(&self, local: Pos2) -> Option<Pos2> {
        (self.w(local) > 1e-4).then(|| self.project(local))
    }

    /// Screen point → element-local CSS px (the view ray meets the tilted plane), or `None`
    /// above the horizon / behind the eye. The result may lie outside the element's box; see
    /// [`Projection::unproject_inside`].
    pub fn unproject(&self, screen: Pos2) -> Option<Pos2> {
        let z = self.view.zoom;
        let s = self.tilt.scale;
        if z.abs() < 1e-6 || s.abs() < 1e-6 {
            return None;
        }
        let xx = (screen.x - self.o_screen.x) / z;
        let yy = (screen.y - self.o_screen.y) / z;
        let den = self.cos + yy * self.k;
        if den <= 1e-6 {
            return None;
        }
        let v = yy / den;
        let w = 1.0 - v * self.k;
        if w <= 1e-4 {
            return None;
        }
        let u = xx * w;
        Some(pos2(self.o_local.x + u / s, self.o_local.y + v / s))
    }

    /// [`Projection::unproject`], restricted to points inside the element's box.
    pub fn unproject_inside(&self, screen: Pos2) -> Option<Pos2> {
        self.unproject(screen)
            .filter(|p| self.local_rect().contains(*p))
    }

    /// The four projected corners of an element-local rect, clockwise from the top-left
    /// (the order of `DOM.getBoxModel`'s quads).
    pub fn quad(&self, r: Rect) -> [Pos2; 4] {
        [
            self.project(r.left_top()),
            self.project(r.right_top()),
            self.project(r.right_bottom()),
            self.project(r.left_bottom()),
        ]
    }

    /// The screen bounding box of a projected element-local rect (`getBoundingClientRect()`).
    pub fn bounds(&self, r: Rect) -> Rect {
        Rect::from_points(&self.quad(r))
    }

    /// The screen bounding box of the whole element.
    pub fn element_bounds(&self) -> Rect {
        self.bounds(self.local_rect())
    }

    /// The element-local row whose projection is screen row `y` (on the plane's vertical
    /// centre line), if it is below the horizon.
    pub fn local_y_at_screen_y(&self, y: f32) -> Option<f32> {
        self.unproject(pos2(self.o_screen.x, y)).map(|p| p.y)
    }

    /// The projection of the same transform with the element moved by `d` stage px (a parent
    /// wrapper's `translate`, such as the jars' bob).
    pub fn translated(&self, d: Vec2) -> Projection {
        Projection::new(self.tilt, self.rect.translate(d), self.view)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::vec2;

    fn tray() -> Projection {
        Projection::new(
            Tilt::new(1600.0, 44.0, 0.96, vec2(0.5, 1.0)),
            Rect::from_min_size(pos2(21.84375, 66.0), vec2(828.0, 562.0)),
            View::new(pos2(0.0, 0.0), 1.0),
        )
    }

    #[test]
    fn origin_row_is_unscaled_and_far_edge_shrinks() {
        let p = tray();
        // bottom edge (origin row): x scaled only by s, y unchanged
        let bl = p.project(pos2(0.0, 562.0));
        assert!((bl.x - 38.40375).abs() < 1e-3, "{bl:?}");
        assert!((bl.y - 628.0).abs() < 1e-3);
        // top-left corner: the numbers Chrome reports (tests/data/chrome-geometry.json)
        let tl = p.project(pos2(0.0, 0.0));
        assert!((tl.x - 113.8315).abs() < 0.01, "{tl:?}");
        assert!((tl.y - 313.5566).abs() < 0.01, "{tl:?}");
        assert!(p.w(pos2(0.0, 0.0)) > 1.0);
    }

    #[test]
    fn forward_inverse_round_trip() {
        for zoom in [1.0, 0.5, 1.7] {
            for (persp, deg, s, oy) in [
                (1600.0, 44.0, 0.96, 1.0),
                (1600.0, 0.0, 0.9, 1.0),
                (1400.0, 36.0, 1.0, 1.0),
                (900.0, 52.0, 0.86, 0.7),
                (900.0, 62.0, 1.0, 0.0),
                (1600.0, 22.0, 0.93, 1.0),
            ] {
                let p = Projection::new(
                    Tilt::new(persp, deg, s, vec2(0.5, oy)),
                    Rect::from_min_size(pos2(13.0, 40.0), vec2(800.0, 500.0)),
                    View::new(pos2(100.0, 50.0), zoom),
                );
                for x in [0.0, 1.0, 123.4, 400.0, 799.0, 800.0] {
                    for y in [0.0, 2.5, 250.0, 499.0, 500.0] {
                        let l = pos2(x, y);
                        let s = p.project(l);
                        let back = p.unproject(s).expect("on the plane");
                        assert!(
                            (back - l).length() < 2e-3,
                            "{persp} {deg} {s:?}: {l:?} → {back:?}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn above_the_horizon_has_no_preimage() {
        let p = tray();
        // the horizon is P·cot θ above the origin row, scaled by the view
        let horizon = 628.0 - 1600.0 / 44f32.to_radians().tan();
        assert!(p.unproject(pos2(400.0, horizon - 10.0)).is_none());
        assert!(p.unproject(pos2(400.0, horizon + 10.0)).is_some());
        assert!(p.unproject_inside(pos2(400.0, 300.0)).is_none()); // above the tray's far edge
        assert!(p.unproject_inside(pos2(435.0, 500.0)).is_some());
    }

    #[test]
    fn identity_is_a_translation() {
        let r = Rect::from_min_size(pos2(10.0, 20.0), vec2(100.0, 50.0));
        let p = Projection::flat(r, View::new(pos2(5.0, 5.0), 2.0));
        assert_eq!(p.project(pos2(0.0, 0.0)), pos2(25.0, 45.0));
        let q = p.project(pos2(100.0, 50.0));
        assert!((q - pos2(225.0, 145.0)).length() < 1e-2, "{q:?}");
    }

    #[test]
    fn tilt_lerp_matches_css_transition() {
        let a = Tilt::new(1600.0, 0.0, 0.9, vec2(0.5, 1.0));
        let b = Tilt::new(1600.0, 44.0, 0.96, vec2(0.5, 1.0));
        let m = Tilt::lerp(a, b, 0.5);
        assert!((m.angle_deg - 22.0).abs() < 1e-5 && (m.scale - 0.93).abs() < 1e-5);
    }
}
