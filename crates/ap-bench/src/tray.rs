//! The Petri tray (Tray · 2D, SPEC TRAY-1/2/3/10): a grid of cells on a tray tilted
//! `perspective(1600px) rotateX(44deg) scale(.96)` over the floor, flattening to
//! `rotateX(0) scale(.9)` for the top-down view.
//!
//! Layout follows `docs/design/mockup/Tray.dc.html`: an 840 × 800 container centred at the top
//! of the stage (`left: 50%; margin-left: -420px`), the floor at (−360, 300) 1560 × 760, the
//! band at y 286, the blurred shadow ellipse at (70, 636) 700 × 60, and the tray at (6, 66)
//! 828 × 562 (origin 50% 100%) holding a grid of 16 px cells with 3 px gaps at (35, 36).

use crate::draw::{Batch, Radii, galley_top_for_css_top, layout_text, radii};
use crate::projection::{Projection, Tilt, View};
use crate::stage;
use crate::tokens::{self, CellLook};
use egui::{Color32, Painter, Pos2, Rect, Vec2, pos2, vec2};

/// What one cell looks like.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum CellVisual {
    /// An empty cell.
    #[default]
    Empty,
    /// A ghost trail after death, fading 3 → 2 → 1 (0 or > 3 are clamped).
    Trail(u8),
    /// A live cell of the given age (1 = newborn, 2–10 alive, > 10 elder); lifted 6 px.
    Live(u16),
}

impl CellVisual {
    /// Live cells lift off the tray.
    pub fn is_lifted(self) -> bool {
        matches!(self, CellVisual::Live(_))
    }
}

/// Size of a cell in CSS px.
pub const CELL: f32 = 16.0;
/// Gap between cells.
pub const GAP: f32 = 3.0;
/// Cell pitch (cell + gap).
pub const PITCH: f32 = CELL + GAP;
/// The grid's offset inside the tray (`left: 35px; top: 36px`).
pub const GRID_ORIGIN: Vec2 = Vec2::new(35.0, 36.0);
/// Tray margins right and bottom of the grid (828 − 35 − 757, 562 − 36 − 491).
const GRID_MARGIN_RB: Vec2 = Vec2::new(36.0, 35.0);
/// The container: 840 × 800, horizontally centred at the top of the stage.
pub const CONTAINER: Vec2 = Vec2::new(840.0, 800.0);
/// The tray's bottom edge in the container (66 + 562): the transform origin's row.
const TRAY_BOTTOM: f32 = 628.0;
/// Cell corner radius.
const CELL_RADIUS: f32 = 4.0;
/// Tray corner radius and edge.
const TRAY_RADIUS: f32 = 20.0;
const TRAY_EDGE_W: f32 = 1.5;
/// The tray's slab depth (`box-shadow: 0 22px 0 …`).
const SLAB: f32 = 22.0;

/// The tilted transform (`perspective(1600px) rotateX(<deg>) scale(.96)`, origin 50% 100%).
pub fn tilted(deg: f32) -> Tilt {
    Tilt::new(1600.0, deg, 0.96, vec2(0.5, 1.0))
}

/// The top-down transform (`perspective(1600px) rotateX(0deg) scale(0.9)`).
pub const FLAT: Tilt = Tilt::new(1600.0, 0.0, 0.9, Vec2::new(0.5, 1.0));

/// The tray's transform for a tilt amount `t` (0 = flat, 1 = tilted by `deg`), interpolated
/// like the CSS transition does (feed it an eased progress, see [`crate::anim::ease`]).
pub fn tray_tilt(t: f32, deg: f32) -> Tilt {
    Tilt::lerp(FLAT, tilted(deg), t.clamp(0.0, 1.0))
}

/// The inputs of one Tray frame. Build it each frame from the engine's grid and the screen's
/// UI state, hit-test with [`TrayScene::geometry`], then [`TrayScene::paint`].
pub struct TrayScene<'a> {
    /// Grid size (the mockup's 40 × 26).
    pub cols: usize,
    pub rows: usize,
    /// The look of the cell at (col, row).
    pub cell: &'a dyn Fn(usize, usize) -> CellVisual,
    /// 0 = flat top-down, 1 = tilted (animate with [`crate::anim::Transition`] + `ease`).
    pub tilt: f32,
    /// The tilt angle in degrees (mockup default 44).
    pub tilt_deg: f32,
    /// The cell under the pointer (hover colours).
    pub hover: Option<(usize, usize)>,
    /// The keyboard cell cursor, drawn as a focus ring (TRAY-10).
    pub cursor: Option<(usize, usize)>,
    /// The engraved label on the left ("PETRI TRAY · 40 × 26 · TORUS").
    pub label_left: &'a str,
    /// The engraved label on the right (the rule code, "B3/S23").
    pub label_right: &'a str,
    /// Stage CSS px → screen points (1 at 1440 px; smaller on narrow windows).
    pub zoom: f32,
}

impl<'a> TrayScene<'a> {
    /// A tilted scene with no hover, cursor or labels at zoom 1.
    pub fn new(cols: usize, rows: usize, cell: &'a dyn Fn(usize, usize) -> CellVisual) -> Self {
        TrayScene {
            cols,
            rows,
            cell,
            tilt: 1.0,
            tilt_deg: 44.0,
            hover: None,
            cursor: None,
            label_left: "",
            label_right: "",
            zoom: 1.0,
        }
    }

    /// The tray's untransformed size in CSS px for this grid (828 × 562 for 40 × 26).
    pub fn tray_size(&self) -> Vec2 {
        tray_size(self.cols, self.rows)
    }

    /// The stage height (screen points) that holds the tray, its slab and its shadow at this
    /// scene's zoom: allocate at least this much on narrow, scrolling layouts.
    pub fn content_height(&self) -> f32 {
        (TRAY_BOTTOM + SLAB + 50.0) * self.zoom
    }

    /// The largest zoom (≤ 1) at which the tray scene fits `avail` (screen points): use it on
    /// narrow windows.
    pub fn fit_zoom(&self, avail: Vec2) -> f32 {
        let content = vec2(
            CONTAINER.x.max(self.tray_size().x + 12.0),
            TRAY_BOTTOM + 60.0,
        );
        (avail.x / content.x)
            .min(avail.y / content.y)
            .clamp(0.05, 1.0)
    }

    /// The screen geometry for a stage at `stage` (the `.graph` rect): the tray projection and
    /// which cells are lifted, for hit-testing and focus rings.
    pub fn geometry(&self, stage: Rect) -> TrayGeometry {
        let view = View::new(stage.min, self.zoom.max(1e-3));
        let container = container_offset(stage, view.zoom);
        let size = self.tray_size();
        let tray = Rect::from_min_size(
            pos2(
                container.x + (CONTAINER.x - size.x) / 2.0,
                container.y + TRAY_BOTTOM - size.y,
            ),
            size,
        );
        let projection = Projection::new(tray_tilt(self.tilt, self.tilt_deg), tray, view);
        let mut lifted = vec![false; self.cols * self.rows];
        for r in 0..self.rows {
            for c in 0..self.cols {
                lifted[r * self.cols + c] = (self.cell)(c, r).is_lifted();
            }
        }
        TrayGeometry {
            projection,
            cols: self.cols,
            rows: self.rows,
            lifted,
            container,
            view,
        }
    }

    /// Paints the floor, band, shadow, tray and cells into `stage` (call [`stage::stage`]
    /// first for the graph paper) and returns the geometry.
    pub fn paint(&self, painter: &Painter, stage_rect: Rect) -> TrayGeometry {
        let geo = self.geometry(stage_rect);
        let painter = painter.with_clip_rect(painter.clip_rect().intersect(stage_rect));
        let mut b = Batch::new(&painter);
        b.reserve(self.cols * self.rows + 64);
        self.paint_ground(&mut b, &geo, stage_rect);
        self.paint_tray(&mut b, &painter, &geo);
        self.paint_cells(&mut b, &geo);
        b.paint(&painter);
        geo
    }

    /// Builds the whole frame into `batch` without labels (no font access needed): the perf
    /// benchmark and tests use it.
    pub fn build(&self, batch: &mut Batch, stage_rect: Rect) -> TrayGeometry {
        let geo = self.geometry(stage_rect);
        self.paint_ground(batch, &geo, stage_rect);
        self.paint_tray_body(batch, &geo.projection);
        self.paint_cells(batch, &geo);
        geo
    }

    fn paint_ground(&self, b: &mut Batch, geo: &TrayGeometry, stage_rect: Rect) {
        let c = geo.container;
        let v = geo.view;
        stage::floor(
            b,
            Rect::from_min_size(c + vec2(-360.0, 300.0), vec2(1560.0, 760.0)),
            v,
            stage_rect,
        );
        stage::floor_band(
            b,
            Rect::from_min_size(c + vec2(0.0, 286.0), vec2(CONTAINER.x, 30.0)),
            v,
        );
        let tray = geo.projection.rect();
        let shadow = Rect::from_min_size(
            pos2(tray.left() + 64.0, tray.bottom() + 8.0),
            vec2((tray.width() - 128.0).max(10.0), 60.0),
        );
        stage::contact_shadow(b, shadow, 16.0, tokens::TRAY_SHADOW, v);
    }

    fn paint_tray_body(&self, b: &mut Batch, p: &Projection) {
        let r = p.local_rect();
        let slab = r.translate(vec2(0.0, SLAB));
        // box-shadow: 0 22px 0 #0B0C0E, 0 22px 0 1.5px #2A2D33 (the first one on top)
        b.fill_rrect(
            p,
            slab.expand(TRAY_EDGE_W),
            radii(TRAY_RADIUS + TRAY_EDGE_W),
            tokens::TRAY_EDGE,
        );
        b.fill_rrect(p, slab, radii(TRAY_RADIUS), tokens::TRAY_SLAB);
        b.fill_rrect(p, r, radii(TRAY_RADIUS), tokens::TRAY_BODY);
        b.border(p, r, radii(TRAY_RADIUS), TRAY_EDGE_W, tokens::TRAY_EDGE);
        // the two screws: (10, 10) and right/bottom 10, 6 × 6
        b.fill_circle(p, pos2(13.0, 13.0), 3.0, tokens::TRAY_EDGE);
        b.fill_circle(
            p,
            pos2(r.right() - 13.0, r.bottom() - 13.0),
            3.0,
            tokens::TRAY_EDGE,
        );
    }

    fn paint_tray(&self, b: &mut Batch, painter: &Painter, geo: &TrayGeometry) {
        let p = &geo.projection;
        self.paint_tray_body(b, p);
        // engraved labels: font 500 11px JetBrains Mono, letter-spacing .1em, top 12
        let font = ap_theme::font::mono_medium(11.0);
        let spacing = 1.1;
        if !self.label_left.is_empty() {
            let g = layout_text(
                painter,
                self.label_left,
                font.clone(),
                tokens::TRAY_LABEL,
                spacing,
            );
            b.text(p, &g, galley_top_for_css_top(&g, 22.0, 12.0, &font));
        }
        if !self.label_right.is_empty() {
            let g = layout_text(
                painter,
                self.label_right,
                font.clone(),
                tokens::TRAY_CODE,
                spacing,
            );
            // CSS right: 22px; Chrome counts the trailing letter-spacing in the width
            let w = g.size().x + spacing;
            let left = p.size().x - 22.0 - w;
            b.text(p, &g, galley_top_for_css_top(&g, left, 12.0, &font));
        }
    }

    fn paint_cells(&self, b: &mut Batch, geo: &TrayGeometry) {
        let p = &geo.projection;
        let cell_size = Vec2::splat(CELL);
        let seg = cell_segments(p, self.zoom);
        let outline = Outline::rrect(cell_size, radii(CELL_RADIUS), seg);
        // pass 1 — cells in normal flow (empty, trails)
        for r in 0..self.rows {
            for c in 0..self.cols {
                let v = (self.cell)(c, r);
                if v.is_lifted() {
                    continue;
                }
                let at = cell_origin(c, r);
                let hovered = self.hover == Some((c, r));
                let fill = if hovered {
                    tokens::EMPTY_HOVER
                } else {
                    match v {
                        CellVisual::Trail(t) if t > 0 => tokens::TRAIL[(t.min(3) - 1) as usize],
                        _ => tokens::EMPTY,
                    }
                };
                b.fill_outline(p, &outline, at, fill);
                // box-shadow: inset 0 1px 0 rgba(255,255,255,.05)
                top_highlight(b, p, at);
            }
        }
        // pass 2 — lifted cells (transformed: painted after the flow, in tree order),
        // each: glow, then its 6 px side, then the face
        for r in 0..self.rows {
            for c in 0..self.cols {
                let CellVisual::Live(age) = (self.cell)(c, r) else {
                    continue;
                };
                let look: CellLook = tokens::cell_look(age);
                let at = cell_origin(c, r);
                let face_at = at - vec2(0.0, tokens::LIFT_PX);
                if let Some(glow) = look.glow {
                    b.blurred_rect(
                        p,
                        Rect::from_min_size(face_at, cell_size),
                        look.glow_px / 2.0,
                        glow,
                    );
                }
                if let Some(side) = look.side {
                    b.fill_outline(p, &outline, at, side);
                }
                let face = if self.hover == Some((c, r)) {
                    Color32::WHITE
                } else {
                    look.face
                };
                b.fill_outline(p, &outline, face_at, face);
            }
        }
        // the keyboard cursor: outline 2px, offset 2px, around the visible face
        if let Some((c, r)) = self.cursor
            && c < self.cols
            && r < self.rows
        {
            let face = geo.cell_face_local(c, r);
            b.ring(
                p,
                face.expand(4.0),
                radii(CELL_RADIUS + 4.0),
                face.expand(2.0),
                radii(CELL_RADIUS + 2.0),
                tokens::FOCUS,
            );
        }
    }

    /// The accessible description of the tray (SPEC A11Y-4), e.g.
    /// "Cell tray, 40 by 26, generation 30, 90 alive".
    pub fn description(&self, generation: u64, alive: usize) -> String {
        crate::a11y::tray(self.cols, self.rows, generation, alive)
    }
}

/// The untransformed tray size for a grid.
pub fn tray_size(cols: usize, rows: usize) -> Vec2 {
    let grid = grid_size(cols, rows);
    GRID_ORIGIN + grid + GRID_MARGIN_RB
}

/// The grid's size in CSS px (757 × 491 for 40 × 26).
pub fn grid_size(cols: usize, rows: usize) -> Vec2 {
    vec2(
        (cols as f32 * PITCH - GAP).max(0.0),
        (rows as f32 * PITCH - GAP).max(0.0),
    )
}

/// Element-local top-left of cell (col, row), unlifted.
pub fn cell_origin(col: usize, row: usize) -> Pos2 {
    pos2(
        GRID_ORIGIN.x + col as f32 * PITCH,
        GRID_ORIGIN.y + row as f32 * PITCH,
    )
}

/// The container's top-left in stage CSS px: `left: 50%; margin-left: -420px; top: 0`.
fn container_offset(stage: Rect, zoom: f32) -> Pos2 {
    pos2(stage.width() / zoom / 2.0 - CONTAINER.x / 2.0, 0.0)
}

fn cell_segments(p: &Projection, zoom: f32) -> usize {
    let m = p.magnification(pos2(p.size().x / 2.0, 0.0)).max(zoom * 0.5);
    ((CELL_RADIUS * m).sqrt() * 1.5).ceil().clamp(2.0, 6.0) as usize
}

/// `inset 0 1px 0 rgba(255,255,255,.05)`: a 1 px crescent under the cell's top edge,
/// tapering into the corners.
fn top_highlight(b: &mut Batch, p: &Projection, at: Pos2) {
    let x0 = at.x;
    let y0 = at.y;
    let pts = [
        (pos2(x0 + 1.2, y0 + 1.25), 0.35),
        (pos2(x0 + 3.0, y0 + 0.15), 0.9),
        (pos2(x0 + CELL - 3.0, y0 + 0.15), 0.9),
        (pos2(x0 + CELL - 1.2, y0 + 1.25), 0.35),
    ];
    b.band_local(p, &pts, tokens::EMPTY_HIGHLIGHT);
}

/// A reusable element-local outline (relative to a box's top-left): build it once per frame,
/// stamp it at many positions with [`Batch::fill_outline`].
#[derive(Clone, Debug, Default)]
pub struct Outline {
    pts: Vec<Vec2>,
}

impl Outline {
    /// A rounded rect of `size` at the origin.
    pub fn rrect(size: Vec2, radii: Radii, segments: usize) -> Outline {
        let mut v = Vec::new();
        crate::draw::rrect_outline(
            Rect::from_min_size(Pos2::ZERO, size),
            radii,
            segments,
            &mut v,
        );
        Outline {
            pts: v.into_iter().map(|p| p.to_vec2()).collect(),
        }
    }

    /// The points, relative to the box's top-left.
    pub fn points(&self) -> &[Vec2] {
        &self.pts
    }
}

/// Screen geometry of a painted tray: hit-testing (through the inverse projection, aware of
/// the 6 px lift of live cells and of the 3 px gaps) and cell rects for chips and focus rings.
#[derive(Clone, Debug)]
pub struct TrayGeometry {
    projection: Projection,
    cols: usize,
    rows: usize,
    lifted: Vec<bool>,
    container: Pos2,
    view: View,
}

impl TrayGeometry {
    /// The tray's projection (element-local CSS px of the 828 × 562 tray ↔ screen).
    pub fn projection(&self) -> &Projection {
        &self.projection
    }

    /// Grid size.
    pub fn grid(&self) -> (usize, usize) {
        (self.cols, self.rows)
    }

    fn is_lifted(&self, c: usize, r: usize) -> bool {
        self.lifted.get(r * self.cols + c).copied().unwrap_or(false)
    }

    /// The cell whose visible face is under `screen`, exactly as the mockup's buttons
    /// hit-test: a live cell is hit on its lifted face (6 px up the tray), its vacated side
    /// and the 3 px gaps belong to no cell, and points off the grid give `None`.
    pub fn cell_at(&self, screen: Pos2) -> Option<(usize, usize)> {
        let l = self.projection.unproject(screen)?;
        let g = l - GRID_ORIGIN.to_pos2();
        if g.x < 0.0 {
            return None;
        }
        let col = (g.x / PITCH).floor() as usize;
        if col >= self.cols || g.x - col as f32 * PITCH >= CELL {
            return None;
        }
        // lifted faces paint on top: test them first
        let gy_face = g.y + tokens::LIFT_PX;
        if gy_face >= 0.0 {
            let row = (gy_face / PITCH).floor() as usize;
            if row < self.rows && gy_face - row as f32 * PITCH < CELL && self.is_lifted(col, row) {
                return Some((col, row));
            }
        }
        if g.y >= 0.0 {
            let row = (g.y / PITCH).floor() as usize;
            if row < self.rows && g.y - row as f32 * PITCH < CELL && !self.is_lifted(col, row) {
                return Some((col, row));
            }
        }
        None
    }

    /// The cell nearest to `screen` while the point is on the grid (gaps included, plus half
    /// a gap around it): use it while drag-painting so a stroke doesn't skip in the gaps.
    pub fn cell_near(&self, screen: Pos2) -> Option<(usize, usize)> {
        if let Some(hit) = self.cell_at(screen) {
            return Some(hit);
        }
        let l = self.projection.unproject(screen)?;
        let g = l - GRID_ORIGIN.to_pos2();
        let size = grid_size(self.cols, self.rows);
        let m = GAP;
        if g.x < -m || g.y < -m - tokens::LIFT_PX || g.x > size.x + m || g.y > size.y + m {
            return None;
        }
        let col = ((g.x + GAP / 2.0) / PITCH)
            .floor()
            .clamp(0.0, (self.cols - 1) as f32);
        let row = ((g.y + GAP / 2.0) / PITCH)
            .floor()
            .clamp(0.0, (self.rows - 1) as f32);
        Some((col as usize, row as usize))
    }

    /// The element-local rect of a cell's visible face (lifted for live cells).
    pub fn cell_face_local(&self, col: usize, row: usize) -> Rect {
        let lift = if self.is_lifted(col, row) {
            tokens::LIFT_PX
        } else {
            0.0
        };
        Rect::from_min_size(cell_origin(col, row) - vec2(0.0, lift), Vec2::splat(CELL))
    }

    /// The projected corners of a cell's visible face, clockwise from the top-left.
    pub fn cell_quad(&self, col: usize, row: usize) -> [Pos2; 4] {
        self.projection.quad(self.cell_face_local(col, row))
    }

    /// The screen bounding box of a cell's visible face (place chips, focus rings, AccessKit
    /// bounds with it).
    pub fn cell_rect(&self, col: usize, row: usize) -> Rect {
        self.projection.bounds(self.cell_face_local(col, row))
    }

    /// The screen bounding box of the tray body (without the slab).
    pub fn tray_rect(&self) -> Rect {
        self.projection.element_bounds()
    }

    /// The screen bounding box of the whole grid.
    pub fn grid_rect(&self) -> Rect {
        self.projection.bounds(Rect::from_min_size(
            GRID_ORIGIN.to_pos2(),
            grid_size(self.cols, self.rows),
        ))
    }
}

impl Batch {
    /// Fills an [`Outline`] placed with its box's top-left at element-local `at`.
    pub fn fill_outline(&mut self, p: &Projection, outline: &Outline, at: Pos2, color: Color32) {
        let mut screen = self.take_screen();
        screen.extend(outline.pts.iter().map(|d| p.project(at + *d)));
        self.fill_convex_screen(&screen, color);
        self.put_screen(screen);
    }
}
