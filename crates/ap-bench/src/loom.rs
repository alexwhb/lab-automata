//! The Loom (Loom · 1D, SPEC LOOM-1/2/3): the woven fabric on a board tilted
//! `perspective(1400px) rotateX(36deg)` (origin 50% 100%), the print head bar with its two
//! spinning spools and progress, and the eight rule key caps.
//!
//! Layout follows `docs/design/mockup/Loom.dc.html`: an 860 × 800 container centred at the
//! top of the stage, the floor at (−360, 250) 1580 × 760, the band at y 236, the board at
//! (54, 18) 752 × 452, the head at (30, 462) 800 × 56 and the keys at (30, 560), 76 × 75 each,
//! spread over 800 px.

use crate::draw::{Batch, galley_top_for_css_top, layout_text, radii};
use crate::projection::{Projection, Tilt, View};
use crate::stage;
use crate::tokens;
use egui::{Color32, Painter, Pos2, Rect, Vec2, pos2, vec2};

/// The container: 860 × 800, horizontally centred at the top of the stage.
pub const CONTAINER: Vec2 = Vec2::new(860.0, 800.0);
/// The board's transform.
pub const BOARD_TILT: Tilt = Tilt::new(1400.0, 36.0, 1.0, Vec2::new(0.5, 1.0));
/// The board's box in the container.
pub const BOARD: Rect = Rect {
    min: Pos2::new(54.0, 18.0),
    max: Pos2::new(806.0, 470.0),
};
/// The print head's box in the container.
pub const HEAD: Rect = Rect {
    min: Pos2::new(30.0, 462.0),
    max: Pos2::new(830.0, 518.0),
};
/// A key cap's size (`.key { width: 76px }`, 75 px tall with its content).
pub const KEY_SIZE: Vec2 = Vec2::new(76.0, 75.0);
/// Where the key row starts in the container and how wide it is.
const KEYS_ORIGIN: Pos2 = Pos2::new(30.0, 560.0);
const KEYS_WIDTH: f32 = 800.0;
/// The caption under the keys (`left: 30px; top: 700px`).
pub const CAPTION: Pos2 = Pos2::new(30.0, 700.0);

/// The inputs of one Loom frame.
pub struct LoomScene<'a> {
    /// The rows woven so far, oldest first, at most `capacity` of them (the newest is drawn
    /// in the newborn colour).
    pub rows: &'a [&'a [bool]],
    /// Cells per row (121).
    pub width: usize,
    /// Rows the board holds (72).
    pub capacity: usize,
    /// The print head label ("PRINT HEAD · 121 CELLS · WRAPS").
    pub head_label: &'a str,
    /// The rule label at the head's right ("Rule 30").
    pub rule_label: &'a str,
    /// Progress bar fill, 0‥1 (rows filled / capacity).
    pub progress: f32,
    /// Spool rotation in radians ([`crate::anim::spool_angle`]).
    pub spool_angle: f32,
    /// Stage CSS px → screen points.
    pub zoom: f32,
}

impl<'a> LoomScene<'a> {
    /// A scene with the mockup's 121 × 72 board and empty labels.
    pub fn new(rows: &'a [&'a [bool]]) -> Self {
        LoomScene {
            rows,
            width: 121,
            capacity: 72,
            head_label: "",
            rule_label: "",
            progress: 0.0,
            spool_angle: 0.0,
            zoom: 1.0,
        }
    }

    /// The largest zoom (≤ 1) at which the loom (board, head, keys, caption) fits `avail`.
    pub fn fit_zoom(avail: Vec2) -> f32 {
        (avail.x / CONTAINER.x)
            .min(avail.y / (CAPTION.y + 30.0))
            .clamp(0.05, 1.0)
    }

    /// Where everything goes for a stage at `stage`.
    pub fn geometry(&self, stage: Rect) -> LoomGeometry {
        let view = View::new(stage.min, self.zoom.max(1e-3));
        let c = pos2(stage.width() / view.zoom / 2.0 - CONTAINER.x / 2.0, 0.0);
        let board = Projection::new(BOARD_TILT, BOARD.translate(c.to_vec2()), view);
        let gap = (KEYS_WIDTH - 8.0 * KEY_SIZE.x) / 7.0;
        let keys = std::array::from_fn(|i| {
            let min = c + KEYS_ORIGIN.to_vec2() + vec2(i as f32 * (KEY_SIZE.x + gap), 0.0);
            view.rect_to_screen(Rect::from_min_size(min, KEY_SIZE))
        });
        LoomGeometry {
            board,
            head: view.rect_to_screen(HEAD.translate(c.to_vec2())),
            keys,
            caption: view.to_screen(c + CAPTION.to_vec2()),
            container: c,
            view,
        }
    }

    /// Paints floor, board (fabric + fade) and print head into `stage`; the keys are separate
    /// ([`paint_key`] at [`LoomGeometry::keys`]) because the app wraps them in buttons.
    pub fn paint(&self, painter: &Painter, stage_rect: Rect) -> LoomGeometry {
        let geo = self.geometry(stage_rect);
        let painter = painter.with_clip_rect(painter.clip_rect().intersect(stage_rect));
        let mut b = Batch::new(&painter);
        self.paint_ground(&mut b, &geo, stage_rect);
        self.paint_board(&mut b, &geo.board);
        self.paint_head(&mut b, &painter, &geo);
        b.paint(&painter);
        geo
    }

    fn paint_ground(&self, b: &mut Batch, geo: &LoomGeometry, stage_rect: Rect) {
        let c = geo.container;
        stage::floor(
            b,
            Rect::from_min_size(c + vec2(-360.0, 250.0), vec2(1580.0, 760.0)),
            geo.view,
            stage_rect,
        );
        stage::floor_band(
            b,
            Rect::from_min_size(c + vec2(0.0, 236.0), vec2(CONTAINER.x, 30.0)),
            geo.view,
        );
    }

    /// The board: ring, body, fabric and fade (no fonts needed; the perf test uses it).
    pub fn paint_board(&self, b: &mut Batch, p: &Projection) {
        let r = p.local_rect();
        // box-shadow: 0 0 0 1px #0B0C0E
        b.fill_rrect(p, r.expand(1.0), radii(7.0), tokens::LOOM_BOARD_RING);
        b.fill_rrect(p, r, radii(6.0), tokens::LOOM_BOARD);
        b.border(p, r, radii(6.0), 1.0, tokens::LOOM_BOARD_EDGE);
        // the fabric (svg 752 × 452): cell i of row ri at (i·cw + .4, (ri + off)·rh)
        let cap = self.capacity.max(1);
        let cw = r.width() / self.width.max(1) as f32;
        let rh = r.height() / cap as f32;
        let shown = &self.rows[self.rows.len().saturating_sub(cap)..];
        let off = cap - shown.len();
        let last = shown.len().saturating_sub(1);
        for (ri, row) in shown.iter().enumerate() {
            let y = (ri + off) as f32 * rh;
            let color = if ri == last {
                tokens::NEWBORN.face
            } else {
                tokens::ACCENT
            };
            for (i, &on) in row.iter().enumerate().take(self.width) {
                if on {
                    let cell =
                        Rect::from_min_size(pos2(i as f32 * cw + 0.4, y), vec2(cw - 0.8, rh - 0.8));
                    b.fill_rect(p, cell, color);
                }
            }
        }
        // the fade: top 90 px, linear-gradient(#0B0C0E, rgba(11,12,14,0)), radius 6 6 0 0
        let fade = Rect::from_min_size(Pos2::ZERO, vec2(r.width(), 90.0));
        b.fill_rrect_with(p, fade, [6.0, 6.0, 0.0, 0.0], |q| {
            tokens::LOOM_FADE.gamma_multiply(1.0 - (q.y / 90.0).clamp(0.0, 1.0))
        });
    }

    fn paint_head(&self, b: &mut Batch, painter: &Painter, geo: &LoomGeometry) {
        let view = geo
            .view
            .offset((HEAD.min + geo.container.to_vec2()).to_vec2());
        let size = HEAD.size();
        let p = Projection::flat(Rect::from_min_size(Pos2::ZERO, size), view);
        let r = p.local_rect();
        // box-shadow: 0 12px 0 #0B0C0E, 0 12px 0 1px #2A2D33, 0 30px 40px rgba(0,0,0,.6)
        b.blurred_rect(
            &p,
            r.translate(vec2(0.0, 30.0)),
            20.0,
            tokens::LOOM_HEAD_SHADOW,
        );
        let drop = r.translate(vec2(0.0, 12.0));
        b.fill_rrect(
            &p,
            drop.expand(1.0),
            radii(13.0),
            tokens::LOOM_HEAD_DROP_EDGE,
        );
        b.fill_rrect(&p, drop, radii(12.0), tokens::LOOM_HEAD_DROP);
        // body: linear-gradient(#26282E, #17191D), border 1px #34373E, radius 12
        b.fill_rrect_with(&p, r, radii(12.0), |q| {
            lerp_color(
                tokens::LOOM_HEAD_TOP,
                tokens::LOOM_HEAD_BOTTOM,
                q.y / size.y,
            )
        });
        b.border(&p, r, radii(12.0), 1.0, tokens::LOOM_HEAD_BORDER);
        // flex row: padding 0 18px, gap 14px, align-items center
        let cy = size.y / 2.0;
        let left = 1.0 + 18.0;
        let right = size.x - 1.0 - 18.0;
        spool(
            b,
            &p,
            pos2(left + 15.0, cy),
            self.spool_angle,
            tokens::ACCENT,
        );
        spool(
            b,
            &p,
            pos2(right - 15.0, cy),
            self.spool_angle,
            tokens::LINK,
        );
        let label_font = ap_theme::font::mono_medium(11.0);
        let rule_font = ap_theme::font::mono_medium(13.0);
        let mut x = left + 30.0 + 14.0;
        if !self.head_label.is_empty() {
            let g = layout_text(
                painter,
                self.head_label,
                label_font.clone(),
                tokens::LOOM_HEAD_LABEL,
                1.1,
            );
            let (asc, desc) = crate::draw::css_line_metrics(&label_font);
            b.text(
                &p,
                &g,
                galley_top_for_css_top(&g, x, cy - (asc + desc) / 2.0, &label_font),
            );
            x += g.size().x + 1.1 + 14.0;
        }
        let mut track_right = right - 30.0 - 14.0;
        if !self.rule_label.is_empty() {
            let g = layout_text(
                painter,
                self.rule_label,
                rule_font.clone(),
                tokens::INK,
                0.0,
            );
            let (asc, desc) = crate::draw::css_line_metrics(&rule_font);
            let lx = track_right - g.size().x;
            b.text(
                &p,
                &g,
                galley_top_for_css_top(&g, lx, cy - (asc + desc) / 2.0, &rule_font),
            );
            track_right = lx - 14.0;
        }
        // the progress track: flex 1, height 8, radius 4
        if track_right > x {
            let track = Rect::from_min_max(pos2(x, cy - 4.0), pos2(track_right, cy + 4.0));
            b.fill_rrect(&p, track, radii(4.0), tokens::LOOM_TRACK);
            let w = track.width() * self.progress.clamp(0.0, 1.0);
            if w > 0.0 {
                let fill = Rect::from_min_size(track.min, vec2(w, 8.0));
                b.fill_rrect(&p, fill, radii(4.0), tokens::ACCENT);
            }
        }
    }

    /// The accessible description of the loom (SPEC A11Y-4).
    pub fn description(&self, rule: u8, rows_woven: u64) -> String {
        crate::a11y::loom(rule, self.width, rows_woven)
    }
}

/// One spool (svg 30 × 30 centred at `c`): `circle r=12 fill #101114 stroke #3A3D44 2`, the
/// spokes `M15 4v22M4 15h22 stroke #4A4D55 2` turned by `angle`, and the fixed hub `r=3`.
fn spool(b: &mut Batch, p: &Projection, c: Pos2, angle: f32, hub: Color32) {
    b.fill_circle(p, c, 12.0, tokens::LOOM_SPOOL);
    b.stroke_circle(p, c, 12.0, 2.0, tokens::LOOM_SPOOL_RIM);
    let (s, co) = angle.sin_cos();
    for d in [vec2(0.0, 11.0), vec2(11.0, 0.0)] {
        let d = vec2(d.x * co - d.y * s, d.x * s + d.y * co);
        b.line(p, c - d, c + d, 2.0, tokens::LOOM_SPOKE);
    }
    b.fill_circle(p, c, 3.0, hub);
}

/// Linear interpolation of two opaque-or-premultiplied colours (CSS gradients interpolate in
/// premultiplied sRGB).
pub(crate) fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgba_premultiplied(
        l(a.r(), b.r()),
        l(a.g(), b.g()),
        l(a.b(), b.b()),
        l(a.a(), b.a()),
    )
}

/// Screen geometry of a painted loom.
#[derive(Clone, Debug)]
pub struct LoomGeometry {
    /// The board's projection (element-local px of the 752 × 452 board ↔ screen).
    pub board: Projection,
    /// The print head bar's screen rect.
    pub head: Rect,
    /// The eight key caps' screen rects (unpressed position, without the 7 px drop), for
    /// keys 111 → 000.
    pub keys: [Rect; 8],
    /// Screen position of the caption's top-left ("These 8 keys are the rule…").
    pub caption: Pos2,
    container: Pos2,
    view: View,
}

impl LoomGeometry {
    /// The screen zoom (points per CSS px) — scale fonts you draw next to the loom with it.
    pub fn zoom(&self) -> f32 {
        self.view.zoom
    }

    /// The screen rect of a key including its drop (the click target).
    pub fn key_target(&self, i: usize) -> Rect {
        let r = self.keys[i.min(7)];
        Rect::from_min_max(r.min, r.max + vec2(0.0, 7.0 * self.view.zoom))
    }

    /// The fabric cell (cell index, row index into `rows`) under `screen`, if any.
    pub fn fabric_cell_at(&self, scene: &LoomScene<'_>, screen: Pos2) -> Option<(usize, usize)> {
        let l = self.board.unproject_inside(screen)?;
        let cap = scene.capacity.max(1);
        let cw = self.board.size().x / scene.width.max(1) as f32;
        let rh = self.board.size().y / cap as f32;
        let shown = scene.rows.len().min(cap);
        let row = (l.y / rh).floor() as usize;
        let off = cap - shown;
        let i = (l.x / cw).floor() as usize;
        (row >= off && row < cap && i < scene.width)
            .then(|| (i, scene.rows.len() - shown + row - off))
    }
}

/// The state of one rule key cap (`.key`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct KeyVisual {
    /// The three-cell neighbourhood pattern, left to right (111 → `[true; 3]`).
    pub pattern: [bool; 3],
    /// The rule bit for this pattern (`.key-on`): the key is sunk 4 px, tinted, glowing, and
    /// its output cell is lit.
    pub on: bool,
    /// Pointer over the key (`.key:hover` border `#6B2A5C`, which wins over `.key-on`'s).
    pub hovered: bool,
    /// Pointer held down on the key: it sinks like an "on" key while pressed.
    pub pressed: bool,
    /// Keyboard focus: a 2 px accent ring, 2 px out.
    pub focused: bool,
}

impl KeyVisual {
    /// The key for pattern `p` (0‥7, bit 2 = left cell) of `rule`.
    pub fn for_rule(rule: u8, p: u8) -> KeyVisual {
        KeyVisual {
            pattern: [p & 4 != 0, p & 2 != 0, p & 1 != 0],
            on: (rule >> (p & 7)) & 1 == 1,
            ..KeyVisual::default()
        }
    }
}

/// Paints a 76 × 75 rule key cap into `rect` (its unpressed box, without the 7 px drop below;
/// [`LoomGeometry::keys`]), scaled to `rect.width() / 76`.
pub fn paint_key(painter: &Painter, rect: Rect, key: KeyVisual) {
    let mut b = Batch::new(painter);
    build_key(&mut b, rect, key);
    b.paint(painter);
}

/// [`paint_key`] into a batch.
pub fn build_key(b: &mut Batch, rect: Rect, key: KeyVisual) {
    let zoom = rect.width() / KEY_SIZE.x;
    let sunk = key.on || key.pressed;
    let dy = if sunk { 4.0 } else { 0.0 };
    let view = View::new(rect.min + vec2(0.0, dy * zoom), zoom);
    let p = Projection::flat(Rect::from_min_size(Pos2::ZERO, KEY_SIZE), view);
    let r = p.local_rect();
    let rad = radii(10.0);
    // shadows, bottom first
    if key.on {
        // 0 3px 0 #561C49, 0 3px 0 1px #F06BD0, 0 0 18px rgba(240,107,208,.25)
        b.blurred_rect(&p, r, 9.0, tokens::KEY_GLOW);
        let drop = r.translate(vec2(0.0, 3.0));
        b.fill_rrect(&p, drop.expand(1.0), radii(11.0), tokens::ACCENT);
        b.fill_rrect(&p, drop, rad, tokens::KEY_DROP_ON);
    } else {
        // 0 7px 0 #0B0C0E, 0 7px 0 1px #2A2D33 (3 px while held down)
        let drop = r.translate(vec2(0.0, if sunk { 3.0 } else { 7.0 }));
        b.fill_rrect(&p, drop.expand(1.0), radii(11.0), tokens::KEY_EDGE);
        b.fill_rrect(&p, drop, rad, tokens::KEY_DROP);
    }
    let (fill, edge) = match (key.on, key.hovered) {
        (_, true) => (
            if key.on { tokens::TINT } else { tokens::KEY },
            tokens::KEY_HOVER_EDGE,
        ),
        (true, false) => (tokens::TINT, tokens::ACCENT),
        (false, false) => (tokens::KEY, tokens::KEY_EDGE),
    };
    b.fill_rrect(&p, r, rad, fill);
    b.border(&p, r, rad, 1.0, edge);
    // the pattern: svg 46 × 14 at (15, 11); rects at x 1, 17, 33, 12 × 12, rx 2.5, stroke 1
    for (k, &on) in key.pattern.iter().enumerate() {
        let sq = Rect::from_min_size(pos2(15.0 + 1.0 + 16.0 * k as f32, 12.0), vec2(12.0, 12.0));
        let c = if on {
            tokens::ACCENT
        } else {
            tokens::KEY_EMPTY
        };
        b.fill_rrect(&p, sq, radii(2.5), c);
        b.stroke_rrect(&p, sq, radii(2.5), 1.0, tokens::KEY_PATTERN_STROKE);
    }
    // the arrow: svg 12 × 10 at (32, 32): M6 1v7 M3 5.5 6 8.5 9 5.5, stroke 1.4, round
    let a = |x: f32, y: f32| pos2(32.0 + x, 32.0 + y);
    let w = 1.4;
    for (from, to) in [
        ((6.0, 1.0), (6.0, 8.0)),
        ((3.0, 5.5), (6.0, 8.5)),
        ((6.0, 8.5), (9.0, 5.5)),
    ] {
        b.line(&p, a(from.0, from.1), a(to.0, to.1), w, tokens::KEY_ARROW);
    }
    for (x, y) in [(6.0, 1.0), (3.0, 5.5), (6.0, 8.5), (9.0, 5.5)] {
        b.fill_circle(&p, a(x, y), w / 2.0, tokens::KEY_ARROW);
    }
    // the output cell: 14 × 14 + 1 px border at (30, 49), radius 3
    let out = Rect::from_min_size(pos2(30.0, 49.0), vec2(16.0, 16.0));
    if key.on {
        b.blurred_rect(&p, out, 5.0, tokens::KEY_OUT_GLOW);
    }
    let out_fill = if key.on {
        tokens::NEWBORN.face
    } else {
        tokens::KEY_EMPTY
    };
    b.fill_rrect(&p, out, radii(3.0), out_fill);
    b.border(&p, out, radii(3.0), 1.0, tokens::KEY_PATTERN_STROKE);
    if key.focused {
        b.ring(
            &p,
            r.expand(4.0),
            radii(14.0),
            r.expand(2.0),
            radii(12.0),
            tokens::FOCUS,
        );
    }
}
