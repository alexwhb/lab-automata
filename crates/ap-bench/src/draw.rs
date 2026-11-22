//! Drawing on a [`Projection`]: shapes are given in element-local CSS px, projected vertex by
//! vertex and appended, in paint order, to one [`Batch`] (a single egui [`Mesh`] that shares
//! the font atlas, so text and fills interleave freely). A whole scene becomes one
//! `Shape::Mesh`.
//!
//! Edges are anti-aliased in screen space with a one-physical-pixel feather ring, like egui's
//! own tessellator; strokes thinner than a pixel fade by coverage instead of vanishing.
//! CSS box-shadow blurs and `filter: blur()` are Gaussian (σ = blur/2 for box-shadows,
//! σ = radius for `filter: blur()`), evaluated analytically on a vertex grid.

use crate::projection::Projection;
use egui::{Color32, Galley, Mesh, Painter, Pos2, Rect, Shape, Vec2, pos2, vec2};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

/// Per-corner radii in CSS px, clockwise from the top-left: `[nw, ne, se, sw]`.
pub type Radii = [f32; 4];

/// The same radius on every corner.
pub const fn radii(r: f32) -> Radii {
    [r, r, r, r]
}

/// Shapes in paint order, flushed as one mesh. See the module docs.
pub struct Batch {
    mesh: Mesh,
    /// One physical pixel in points (the anti-aliasing feather).
    feather: f32,
    /// 1 / font atlas size, for glyph UVs (texels → normalised).
    uv_norm: Vec2,
    outline: Vec<Pos2>,
    inner: Vec<Pos2>,
    screen: Vec<Pos2>,
    colors: Vec<Color32>,
}

impl Batch {
    /// An empty batch for a painter (reads its pixels-per-point and font atlas size).
    pub fn new(painter: &Painter) -> Batch {
        let [w, h] = painter.fonts(|f| f.font_image_size());
        Batch::with_params(
            painter.pixels_per_point(),
            vec2(1.0 / w.max(1) as f32, 1.0 / h.max(1) as f32),
        )
    }

    /// An empty batch for `pixels_per_point` and a font atlas UV normaliser (tests, benches).
    pub fn with_params(pixels_per_point: f32, uv_norm: Vec2) -> Batch {
        Batch {
            mesh: Mesh::default(),
            feather: 1.0 / pixels_per_point.max(0.1),
            uv_norm,
            outline: Vec::with_capacity(64),
            inner: Vec::with_capacity(64),
            screen: Vec::with_capacity(64),
            colors: Vec::with_capacity(64),
        }
    }

    /// The mesh built so far.
    pub fn mesh(&self) -> &Mesh {
        &self.mesh
    }

    /// Takes the mesh out of the batch.
    pub fn into_mesh(self) -> Mesh {
        self.mesh
    }

    /// Adds the mesh to `painter` as one shape.
    pub fn paint(self, painter: &Painter) {
        if !self.mesh.is_empty() {
            painter.add(Shape::mesh(self.mesh));
        }
    }

    /// Reserves room for about `shapes` small shapes (≈ 32 vertices each).
    pub fn reserve(&mut self, shapes: usize) {
        self.mesh.reserve_vertices(shapes * 32);
        self.mesh.reserve_triangles(shapes * 46);
    }

    // ---- screen-space primitives ---------------------------------------------------------

    /// Fills a convex screen-space polygon (either winding) with an anti-aliased edge.
    pub fn fill_convex_screen(&mut self, pts: &[Pos2], color: Color32) {
        self.colors.clear();
        self.colors.resize(pts.len(), color);
        let colors = std::mem::take(&mut self.colors);
        self.fill_convex_screen_colored(pts, &colors);
        self.colors = colors;
    }

    /// Fills a convex screen-space polygon whose vertices carry their own colours (linear
    /// gradients are reproduced exactly by any triangulation).
    pub fn fill_convex_screen_colored(&mut self, pts: &[Pos2], colors: &[Color32]) {
        let n = pts.len();
        if n < 3 || colors.len() != n {
            return;
        }
        let area = signed_area(pts);
        if area.abs() < 1e-6 {
            return;
        }
        // outward direction: for a clockwise (on screen, y down: area > 0) polygon the
        // outward normal of edge d is (d.y, −d.x)
        let cw = area > 0.0;
        let h = self.feather / 2.0;
        let base = self.mesh.vertices.len() as u32;
        for i in 0..n {
            let a = pts[(i + n - 1) % n];
            let b = pts[i];
            let c = pts[(i + 1) % n];
            let m = miter(a, b, c, cw);
            self.mesh.colored_vertex(b - m * h, colors[i]);
            self.mesh.colored_vertex(b + m * h, Color32::TRANSPARENT);
        }
        let n32 = n as u32;
        for i in 0..n32 {
            let j = (i + 1) % n32;
            let (ii, io, ji, jo) = (
                base + 2 * i,
                base + 2 * i + 1,
                base + 2 * j,
                base + 2 * j + 1,
            );
            self.mesh.add_triangle(ii, io, jo);
            self.mesh.add_triangle(ii, jo, ji);
        }
        for i in 1..n32 - 1 {
            self.mesh
                .add_triangle(base, base + 2 * i, base + 2 * (i + 1));
        }
    }

    /// An anti-aliased band between paired screen points (`outer[i]`, `inner[i]`), closed
    /// (a ring) or open (a strip). Where the band is thinner than a pixel it fades by coverage.
    fn band_screen(&mut self, outer: &[Pos2], inner: &[Pos2], closed: bool, color: Color32) {
        let n = outer.len().min(inner.len());
        if n < 2 {
            return;
        }
        let f = self.feather;
        let base = self.mesh.vertices.len() as u32;
        for i in 0..n {
            let (o, q) = (outer[i], inner[i]);
            let d = o - q;
            let len = d.length();
            let dir = if len > 1e-6 { d / len } else { Vec2::ZERO };
            let mid = q + d * 0.5;
            let (solid, fade, c) = if len >= f {
                ((len - f) / 2.0, (len + f) / 2.0, color)
            } else {
                (0.0, f, color.gamma_multiply(len / f))
            };
            self.mesh
                .colored_vertex(mid + dir * fade, Color32::TRANSPARENT);
            self.mesh.colored_vertex(mid + dir * solid, c);
            self.mesh.colored_vertex(mid - dir * solid, c);
            self.mesh
                .colored_vertex(mid - dir * fade, Color32::TRANSPARENT);
        }
        let segs = if closed { n } else { n - 1 } as u32;
        let n32 = n as u32;
        for i in 0..segs {
            let j = (i + 1) % n32;
            let (a, b) = (base + 4 * i, base + 4 * j);
            for k in 0..3 {
                self.mesh.add_triangle(a + k, a + k + 1, b + k + 1);
                self.mesh.add_triangle(a + k, b + k + 1, b + k);
            }
        }
    }

    /// An open strip along element-local points, each with its thickness (CSS px) measured
    /// downwards from the point (at most 8 points).
    pub fn band_local(&mut self, p: &Projection, pts: &[(Pos2, f32)], color: Color32) {
        let n = pts.len().min(8);
        let mut outer = [Pos2::ZERO; 8];
        let mut inner = [Pos2::ZERO; 8];
        for (i, (q, t)) in pts.iter().take(n).enumerate() {
            outer[i] = p.project(*q);
            inner[i] = p.project(*q + vec2(0.0, *t));
        }
        self.band_screen(&outer[..n], &inner[..n], false, color);
    }

    /// A scratch vector for screen points (return it with [`Batch::put_screen`]).
    pub(crate) fn take_screen(&mut self) -> Vec<Pos2> {
        let mut v = std::mem::take(&mut self.screen);
        v.clear();
        v
    }

    pub(crate) fn put_screen(&mut self, v: Vec<Pos2>) {
        self.screen = v;
    }

    // ---- projected primitives ------------------------------------------------------------

    /// Fills an element-local convex polygon.
    pub fn fill_poly(&mut self, p: &Projection, local: &[Pos2], color: Color32) {
        let mut screen = std::mem::take(&mut self.screen);
        screen.clear();
        screen.extend(local.iter().map(|q| p.project(*q)));
        self.fill_convex_screen(&screen, color);
        self.screen = screen;
    }

    /// Fills an element-local rect (a projected quad).
    pub fn fill_rect(&mut self, p: &Projection, r: Rect, color: Color32) {
        let q = p.quad(r);
        self.fill_convex_screen(&q, color);
    }

    /// Fills an element-local rect with a vertical gradient from `top` to `bottom`.
    pub fn fill_rect_vgradient(&mut self, p: &Projection, r: Rect, top: Color32, bottom: Color32) {
        let q = p.quad(r);
        self.fill_convex_screen_colored(&q, &[top, top, bottom, bottom]);
    }

    /// Fills an element-local rounded rect (CSS `border-radius`).
    pub fn fill_rrect(&mut self, p: &Projection, r: Rect, radii: Radii, color: Color32) {
        self.fill_rrect_with(p, r, radii, |_| color);
    }

    /// Fills an element-local rounded rect whose colour varies with the local position
    /// (`color_at` must be linear for an exact gradient: CSS `linear-gradient`).
    pub fn fill_rrect_with(
        &mut self,
        p: &Projection,
        r: Rect,
        radii: Radii,
        color_at: impl Fn(Pos2) -> Color32,
    ) {
        let mut outline = std::mem::take(&mut self.outline);
        outline.clear();
        let seg = segments_for(p, r, &radii);
        rrect_outline(r, radii, seg, &mut outline);
        let mut screen = std::mem::take(&mut self.screen);
        screen.clear();
        screen.extend(outline.iter().map(|q| p.project(*q)));
        let mut colors = std::mem::take(&mut self.colors);
        colors.clear();
        colors.extend(outline.iter().map(|q| color_at(*q)));
        self.fill_convex_screen_colored(&screen, &colors);
        self.outline = outline;
        self.screen = screen;
        self.colors = colors;
    }

    /// Fills an element-local circle.
    pub fn fill_circle(&mut self, p: &Projection, center: Pos2, radius: f32, color: Color32) {
        let r = Rect::from_center_size(center, Vec2::splat(2.0 * radius));
        self.fill_rrect(p, r, radii(radius), color);
    }

    /// Fills the region between two element-local rounded rects (`inner` inside `outer`).
    pub fn ring(
        &mut self,
        p: &Projection,
        outer: Rect,
        outer_radii: Radii,
        inner: Rect,
        inner_radii: Radii,
        color: Color32,
    ) {
        let seg = segments_for(p, outer, &outer_radii);
        let mut a = std::mem::take(&mut self.outline);
        let mut b = std::mem::take(&mut self.inner);
        a.clear();
        b.clear();
        rrect_outline_fixed(outer, outer_radii, seg, &mut a);
        rrect_outline_fixed(inner, inner_radii, seg, &mut b);
        let mut sa = std::mem::take(&mut self.screen);
        sa.clear();
        sa.extend(a.iter().map(|q| p.project(*q)));
        let sb: Vec<Pos2> = b.iter().map(|q| p.project(*q)).collect();
        self.band_screen(&sa, &sb, true, color);
        self.outline = a;
        self.inner = b;
        self.screen = sa;
    }

    /// A CSS border: a band of `width` CSS px inside the border box `r`.
    pub fn border(&mut self, p: &Projection, r: Rect, radii: Radii, width: f32, color: Color32) {
        let inner = r.shrink(width);
        let ir = radii.map(|v| (v - width).max(0.0));
        self.ring(p, r, radii, inner, ir, color);
    }

    /// A stroke of `width` centred on an element-local rounded rect's outline (SVG `stroke`).
    pub fn stroke_rrect(
        &mut self,
        p: &Projection,
        r: Rect,
        radii: Radii,
        width: f32,
        color: Color32,
    ) {
        let h = width / 2.0;
        self.ring(
            p,
            r.expand(h),
            radii.map(|v| v + h),
            r.shrink(h),
            radii.map(|v| (v - h).max(0.0)),
            color,
        );
    }

    /// A stroke of `width` centred on an element-local circle.
    pub fn stroke_circle(
        &mut self,
        p: &Projection,
        center: Pos2,
        radius: f32,
        width: f32,
        color: Color32,
    ) {
        let r = Rect::from_center_size(center, Vec2::splat(2.0 * radius));
        self.stroke_rrect(p, r, self::radii(radius), width, color);
    }

    /// A butt-capped element-local line segment `a → b`, `width` CSS px wide.
    pub fn line(&mut self, p: &Projection, a: Pos2, b: Pos2, width: f32, color: Color32) {
        let d = (b - a).normalized();
        let n = vec2(-d.y, d.x) * (width / 2.0);
        let outer = [p.project(a + n), p.project(b + n)];
        let inner = [p.project(a - n), p.project(b - n)];
        self.band_screen(&outer, &inner, false, color);
    }

    /// Paints a laid-out galley with its top-left corner at element-local `top_left`, every
    /// glyph vertex projected onto the plane (labels lie on the tilted surface).
    pub fn text(&mut self, p: &Projection, galley: &Galley, top_left: Pos2) {
        self.text_tinted(p, galley, top_left, None);
    }

    /// [`Batch::text`] with an optional colour that replaces the galley's text colour.
    pub fn text_tinted(
        &mut self,
        p: &Projection,
        galley: &Galley,
        top_left: Pos2,
        tint: Option<Color32>,
    ) {
        for row in &galley.rows {
            let vis = &row.visuals;
            if vis.mesh.is_empty() {
                continue;
            }
            let base = self.mesh.vertices.len() as u32;
            let off = top_left.to_vec2() + row.pos.to_vec2();
            for (i, v) in vis.mesh.vertices.iter().enumerate() {
                let mut color = v.color;
                if let Some(t) = tint
                    && vis.glyph_vertex_range.contains(&i)
                {
                    color = t;
                }
                self.mesh.vertices.push(egui::epaint::Vertex {
                    pos: p.project(v.pos + off),
                    uv: pos2(v.uv.x * self.uv_norm.x, v.uv.y * self.uv_norm.y),
                    color,
                });
            }
            self.mesh
                .indices
                .extend(vis.mesh.indices.iter().map(|i| i + base));
        }
    }

    /// A Gaussian-blurred rect (CSS `box-shadow: 0 0 <2σ>px <color>` around `r`, or a
    /// blurred rectangle): alpha = Φ-differences along x times along y. The rect's corner
    /// radius is small next to σ and is ignored.
    pub fn blurred_rect(&mut self, p: &Projection, r: Rect, sigma: f32, color: Color32) {
        if sigma <= 0.05 {
            self.fill_rect(p, r, color);
            return;
        }
        let xs = BlurSamples::new(r.left(), r.right(), sigma);
        let ys = BlurSamples::new(r.top(), r.bottom(), sigma);
        let mut fx = [0.0f32; BlurSamples::MAX];
        for (f, &x) in fx.iter_mut().zip(xs.as_slice()) {
            *f = box_profile(x, r.left(), r.right(), sigma);
        }
        let base = self.mesh.vertices.len() as u32;
        for &y in ys.as_slice() {
            let fy = box_profile(y, r.top(), r.bottom(), sigma);
            for (i, &x) in xs.as_slice().iter().enumerate() {
                self.mesh
                    .colored_vertex(p.project(pos2(x, y)), color.gamma_multiply(fx[i] * fy));
            }
        }
        add_grid_triangles(&mut self.mesh, base, xs.len, ys.len);
    }

    /// A Gaussian-blurred ellipse filling `r` (`border-radius: 50%; filter: blur(σ px)`), the
    /// contact shadows under the tray and the jars. The alpha grid is cached per size.
    pub fn blurred_ellipse(&mut self, p: &Projection, r: Rect, sigma: f32, color: Color32) {
        let grid = ellipse_grid(r.width(), r.height(), sigma);
        let base = self.mesh.vertices.len() as u32;
        let origin = r.min - Vec2::splat(grid.margin);
        for j in 0..grid.ny {
            for i in 0..grid.nx {
                let a = grid.alpha[j * grid.nx + i];
                let q = origin + vec2(i as f32 * grid.step.x, j as f32 * grid.step.y);
                self.mesh
                    .colored_vertex(p.project(q), color.gamma_multiply(a));
            }
        }
        add_grid_triangles(&mut self.mesh, base, grid.nx, grid.ny);
    }
}

/// Lays out one line of text with CSS `letter-spacing` (px).
pub fn layout_text(
    painter: &Painter,
    text: &str,
    font: egui::FontId,
    color: Color32,
    letter_spacing: f32,
) -> Arc<Galley> {
    let mut job = egui::text::LayoutJob::default();
    job.append(
        text,
        0.0,
        egui::TextFormat {
            font_id: font,
            color,
            extra_letter_spacing: letter_spacing,
            ..Default::default()
        },
    );
    painter.layout_job(job)
}

/// CSS `line-height: normal` metrics (ascent, descent) of the series fonts, rounded like
/// Blink does: JetBrains Mono (ascender 1020, descender −300 per 1000 em) and Inter
/// (1984 / −494 per 2048 em).
pub fn css_line_metrics(font: &egui::FontId) -> (f32, f32) {
    use egui::FontFamily;
    let s = font.size;
    let mono = match &font.family {
        FontFamily::Monospace => true,
        FontFamily::Name(n) => n.as_ref() == ap_theme::font::MONO_MEDIUM,
        FontFamily::Proportional => false,
    };
    let (a, d) = if mono {
        (1.02, 0.30)
    } else {
        (0.968_75, 0.241_21)
    };
    ((a * s).round(), (d * s).round())
}

/// The top-left at which to paint `galley` so that its first baseline lands where Chrome puts
/// the baseline of a CSS line box (`line-height: normal`) whose top is at `top`.
pub fn galley_top_for_css_top(galley: &Galley, left: f32, top: f32, font: &egui::FontId) -> Pos2 {
    let (asc, _) = css_line_metrics(font);
    let baseline = top + asc;
    let egui_baseline = galley
        .rows
        .first()
        .map(|r| r.pos.y + r.glyphs.first().map_or(font.size * 0.8, |g| g.pos.y))
        .unwrap_or(font.size * 0.8);
    pos2(left, baseline - egui_baseline)
}

// ---- geometry helpers ---------------------------------------------------------------------

/// Twice the signed area of a polygon (positive = clockwise on a y-down screen).
pub fn signed_area(pts: &[Pos2]) -> f32 {
    let n = pts.len();
    let mut a = 0.0;
    for i in 0..n {
        let p = pts[i];
        let q = pts[(i + 1) % n];
        a += p.x * q.y - q.x * p.y;
    }
    a
}

/// The outward miter vector at `b` (unit length on straight edges, longer at corners, capped).
#[inline]
fn miter(a: Pos2, b: Pos2, c: Pos2, cw: bool) -> Vec2 {
    let out = |d: Vec2| {
        let d = d.normalized();
        if cw { vec2(d.y, -d.x) } else { vec2(-d.y, d.x) }
    };
    let n0 = out(b - a);
    let n1 = out(c - b);
    let k = 1.0 + n0.dot(n1);
    if k < 1e-3 {
        return n0;
    }
    let m = (n0 + n1) / k;
    if m.length_sq() > 4.0 {
        m.normalized() * 2.0
    } else {
        m
    }
}

/// Corner segments for a rounded rect, from its on-screen corner size.
fn segments_for(p: &Projection, r: Rect, radii: &Radii) -> usize {
    let rmax = radii.iter().copied().fold(0.0f32, f32::max);
    if rmax <= 0.0 {
        return 1;
    }
    let screen_r = rmax * p.magnification(r.center());
    ((screen_r.max(0.0).sqrt() * 1.6).ceil() as usize).clamp(2, 16)
}

/// Clamps radii like CSS (scaled down together when adjacent radii overflow a side).
fn clamp_radii(r: Rect, radii: Radii) -> Radii {
    let [nw, ne, se, sw] = radii.map(|v| v.max(0.0));
    let (w, h) = (r.width().max(0.0), r.height().max(0.0));
    let mut f: f32 = 1.0;
    for (sum, side) in [(nw + ne, w), (sw + se, w), (nw + sw, h), (ne + se, h)] {
        if sum > side && sum > 0.0 {
            f = f.min(side / sum);
        }
    }
    [nw * f, ne * f, se * f, sw * f]
}

/// The outline of a rounded rect, clockwise from the top-left arc; sharp corners give one
/// point.
pub fn rrect_outline(r: Rect, radii: Radii, segments: usize, out: &mut Vec<Pos2>) {
    let radii = clamp_radii(r, radii);
    let corners = [
        (
            pos2(r.left() + radii[0], r.top() + radii[0]),
            radii[0],
            180.0f32,
        ),
        (
            pos2(r.right() - radii[1], r.top() + radii[1]),
            radii[1],
            270.0,
        ),
        (
            pos2(r.right() - radii[2], r.bottom() - radii[2]),
            radii[2],
            0.0,
        ),
        (
            pos2(r.left() + radii[3], r.bottom() - radii[3]),
            radii[3],
            90.0,
        ),
    ];
    for (c, rad, start) in corners {
        if rad <= 1e-3 {
            out.push(c);
            continue;
        }
        for k in 0..=segments {
            let a = (start + 90.0 * k as f32 / segments as f32).to_radians();
            out.push(pos2(c.x + rad * a.cos(), c.y + rad * a.sin()));
        }
    }
}

/// Like [`rrect_outline`] but always `segments + 1` points per corner, so two outlines can be
/// paired point by point (rings, borders).
fn rrect_outline_fixed(r: Rect, radii: Radii, segments: usize, out: &mut Vec<Pos2>) {
    let radii = clamp_radii(r, radii);
    let corners = [
        (
            pos2(r.left() + radii[0], r.top() + radii[0]),
            radii[0],
            180.0f32,
        ),
        (
            pos2(r.right() - radii[1], r.top() + radii[1]),
            radii[1],
            270.0,
        ),
        (
            pos2(r.right() - radii[2], r.bottom() - radii[2]),
            radii[2],
            0.0,
        ),
        (
            pos2(r.left() + radii[3], r.bottom() - radii[3]),
            radii[3],
            90.0,
        ),
    ];
    for (c, rad, start) in corners {
        for k in 0..=segments {
            let a = (start + 90.0 * k as f32 / segments as f32).to_radians();
            out.push(pos2(c.x + rad * a.cos(), c.y + rad * a.sin()));
        }
    }
}

/// The standard normal CDF Φ (Abramowitz & Stegun 7.1.26 erf, |error| < 1.5e-7).
pub fn norm_cdf(x: f32) -> f32 {
    let z = x / std::f32::consts::SQRT_2;
    let t = 1.0 / (1.0 + 0.327_591_1 * z.abs());
    let y = 1.0
        - (((((1.061_405_4 * t - 1.453_152_1) * t) + 1.421_413_7) * t - 0.284_496_74) * t
            + 0.254_829_6)
            * t
            * (-z * z).exp();
    let erf = if z >= 0.0 { y } else { -y };
    0.5 * (1.0 + erf)
}

/// The coverage of `[a, b]` blurred by σ at `x`.
#[inline]
fn box_profile(x: f32, a: f32, b: f32, sigma: f32) -> f32 {
    (norm_cdf((b - x) / sigma) - norm_cdf((a - x) / sigma)).clamp(0.0, 1.0)
}

/// Sample positions along one axis of a blurred span `[a, b]`: dense near the edges, sparse
/// in the flat interior, out to 2.5σ (beyond it the shadow is < 1 % of its peak).
struct BlurSamples {
    v: [f32; BlurSamples::MAX],
    len: usize,
}

impl BlurSamples {
    const MAX: usize = 15;
    const OUT: [f32; 5] = [2.5, 1.6, 0.9, 0.35, 0.0];
    const IN: [f32; 2] = [0.7, 1.8];

    fn new(a: f32, b: f32, sigma: f32) -> BlurSamples {
        let mut s = BlurSamples {
            v: [0.0; Self::MAX],
            len: 0,
        };
        let mid = (a + b) / 2.0;
        for k in Self::OUT {
            s.push(a - k * sigma);
        }
        for k in Self::IN {
            let x = a + k * sigma;
            if x < mid - 0.5 * sigma {
                s.push(x);
            }
        }
        s.push(mid);
        for k in Self::IN.iter().rev() {
            let x = b - k * sigma;
            if x > mid + 0.5 * sigma {
                s.push(x);
            }
        }
        for k in Self::OUT.iter().rev() {
            s.push(b + k * sigma);
        }
        s
    }

    fn push(&mut self, x: f32) {
        if self.len < Self::MAX && (self.len == 0 || x - self.v[self.len - 1] >= 0.25) {
            self.v[self.len] = x;
            self.len += 1;
        }
    }

    fn as_slice(&self) -> &[f32] {
        &self.v[..self.len]
    }
}

fn add_grid_triangles(mesh: &mut Mesh, base: u32, nx: usize, ny: usize) {
    let n = nx as u32;
    for j in 0..ny.saturating_sub(1) as u32 {
        for i in 0..n.saturating_sub(1) {
            let k = base + j * n + i;
            mesh.add_triangle(k, k + 1, k + n);
            mesh.add_triangle(k + 1, k + n + 1, k + n);
        }
    }
}

/// A blurred ellipse sampled on a regular grid (alpha per vertex, row-major).
struct EllipseGrid {
    nx: usize,
    ny: usize,
    step: Vec2,
    margin: f32,
    alpha: Vec<f32>,
}

thread_local! {
    static ELLIPSES: RefCell<HashMap<(u32, u32, u32), Rc<EllipseGrid>>> = RefCell::new(HashMap::new());
}

fn ellipse_grid(w: f32, h: f32, sigma: f32) -> Rc<EllipseGrid> {
    let key = (
        (w * 4.0).round() as u32,
        (h * 4.0).round() as u32,
        (sigma * 4.0).round() as u32,
    );
    ELLIPSES.with(|c| {
        c.borrow_mut()
            .entry(key)
            .or_insert_with(|| Rc::new(build_ellipse(w, h, sigma)))
            .clone()
    })
}

fn build_ellipse(w: f32, h: f32, sigma: f32) -> EllipseGrid {
    let sigma = sigma.max(0.1);
    let margin = 3.0 * sigma;
    let (tw, th) = (w + 2.0 * margin, h + 2.0 * margin);
    let nx = ((tw / (sigma / 2.0).max(2.0)).ceil() as usize).max(2) + 1;
    let ny = ((th / (sigma / 3.0).max(1.5)).ceil() as usize).max(2) + 1;
    let step = vec2(tw / (nx - 1) as f32, th / (ny - 1) as f32);
    let (rx, ry) = (w / 2.0, h / 2.0);
    // ∫ G(y − y') [Φ((x_r(y') − x)/σ) − Φ((x_l(y') − x)/σ)] dy' over the ellipse's rows
    let rows = 48;
    let dy = h / rows as f32;
    let norm = 1.0 / (sigma * (2.0 * std::f32::consts::PI).sqrt());
    let spans: Vec<(f32, f32)> = (0..rows)
        .map(|k| {
            let yy = -ry + (k as f32 + 0.5) * dy;
            (yy, rx * (1.0 - (yy / ry).powi(2)).max(0.0).sqrt())
        })
        .collect();
    let mut alpha = Vec::with_capacity(nx * ny);
    for j in 0..ny {
        let y = -ry - margin + j as f32 * step.y;
        for i in 0..nx {
            let x = -rx - margin + i as f32 * step.x;
            let mut v = 0.0;
            for &(yy, half) in &spans {
                let gy = norm * (-(y - yy).powi(2) / (2.0 * sigma * sigma)).exp() * dy;
                if gy < 1e-6 {
                    continue;
                }
                v += gy * (norm_cdf((half - x) / sigma) - norm_cdf((-half - x) / sigma));
            }
            alpha.push(v.clamp(0.0, 1.0));
        }
    }
    EllipseGrid {
        nx,
        ny,
        step,
        margin,
        alpha,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::projection::{Tilt, View};

    #[test]
    fn norm_cdf_is_accurate() {
        assert!((norm_cdf(0.0) - 0.5).abs() < 1e-6);
        assert!((norm_cdf(1.0) - 0.841_344_7).abs() < 1e-5);
        assert!((norm_cdf(-2.0) - 0.022_750_1).abs() < 1e-5);
    }

    #[test]
    fn rrect_outline_is_clockwise_and_closed_by_corners() {
        let mut v = Vec::new();
        rrect_outline(
            Rect::from_min_size(pos2(0.0, 0.0), vec2(16.0, 16.0)),
            radii(4.0),
            3,
            &mut v,
        );
        assert_eq!(v.len(), 16);
        assert!(signed_area(&v) > 0.0);
        assert!((v[0] - pos2(0.0, 4.0)).length() < 1e-4);
        // radii larger than the box are scaled down (a circle)
        v.clear();
        rrect_outline(
            Rect::from_min_size(pos2(0.0, 0.0), vec2(10.0, 10.0)),
            radii(50.0),
            4,
            &mut v,
        );
        assert!(
            v.iter()
                .all(|p| (p.distance(pos2(5.0, 5.0)) - 5.0).abs() < 1e-3)
        );
    }

    #[test]
    fn convex_fill_emits_ring_and_fan() {
        let mut b = Batch::with_params(2.0, vec2(1.0, 1.0));
        let p = Projection::new(
            Tilt::new(1600.0, 44.0, 0.96, vec2(0.5, 1.0)),
            Rect::from_min_size(pos2(0.0, 0.0), vec2(828.0, 562.0)),
            View::new(pos2(0.0, 0.0), 1.0),
        );
        b.fill_rect(
            &p,
            Rect::from_min_size(pos2(35.0, 36.0), vec2(16.0, 16.0)),
            Color32::RED,
        );
        assert_eq!(b.mesh().vertices.len(), 8);
        assert_eq!(b.mesh().indices.len(), 3 * (8 + 2));
        assert!(b.mesh().is_valid());
    }

    #[test]
    fn blurred_rect_peaks_inside() {
        let mut b = Batch::with_params(1.0, vec2(1.0, 1.0));
        let p = Projection::flat(
            Rect::from_min_size(Pos2::ZERO, vec2(100.0, 100.0)),
            View::new(Pos2::ZERO, 1.0),
        );
        b.blurred_rect(
            &p,
            Rect::from_min_size(pos2(40.0, 40.0), vec2(16.0, 16.0)),
            8.0,
            Color32::WHITE,
        );
        let m = b.mesh();
        assert!(m.is_valid());
        let max_a = m.vertices.iter().map(|v| v.color.a()).max().unwrap();
        // Φ(1) − Φ(−1) squared ≈ 0.466 at the centre of a 16 px box blurred by σ = 8
        assert!((max_a as f32 / 255.0 - 0.466).abs() < 0.02, "{max_a}");
    }

    #[test]
    fn blurred_ellipse_is_cached_and_faint_at_the_rim() {
        let g1 = ellipse_grid(700.0, 60.0, 16.0);
        let g2 = ellipse_grid(700.0, 60.0, 16.0);
        assert!(Rc::ptr_eq(&g1, &g2));
        let corner = g1.alpha[0];
        let centre = g1.alpha[(g1.ny / 2) * g1.nx + g1.nx / 2];
        assert!(corner < 0.01 && centre > 0.6, "{corner} {centre}");
    }
}
