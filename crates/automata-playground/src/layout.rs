//! Page layout: the mockup's body rows are `display: flex; flex-wrap: wrap` with fixed-height
//! columns (`flex: <grow> <shrink> <basis>`, `max-width`, `box-sizing: border-box`). At 1440
//! they sit side by side and fill the window under the header; at 390 they stack, each at the
//! mockup's fixed panel height (844 / 904), and the page scrolls.
//!
//! ```ignore
//! const COLUMNS: [Column; 3] = [
//!     Column::aside("Cartridges and stamps", 248.0).pad(14.0, 12.0).border_right(),
//!     Column::main("Bench"),
//!     Column::readout("Rule and readouts").gap(18.0),
//! ];
//! layout::columns(ui, ctx, &COLUMNS, |ui, ctx, i| match i {
//!     0 => rules(ui, ctx),
//!     1 => layout::bench(ui, ctx, self, |s, ctx, stage| s.stage(ctx, stage)),
//!     _ => readouts(ui, ctx),
//! });
//! ```

use crate::ctx::Ctx;
use crate::screens::Screen;
use crate::widgets::tabs::bench_tabs;
use ap_theme as t;
use eframe::egui::{
    Align, Color32, CornerRadius, Layout, Rect, ScrollArea, Stroke, Ui, UiBuilder, accesskit::Role,
    pos2, vec2,
};
use lab_ui::layout::{FlexItem, flex_lines};

/// The smallest height side-by-side columns get (shorter windows scroll the page).
pub const MIN_PANEL_H: f32 = 480.0;

/// One column of a page row.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Column {
    /// The region's accessible name (`<aside aria-label>`).
    pub label: &'static str,
    /// `flex-grow`.
    pub grow: f32,
    /// `flex-shrink`.
    pub shrink: f32,
    /// `flex-basis` (border box).
    pub basis: f32,
    /// `max-width`.
    pub max: Option<f32>,
    /// Background (asides: `rgba(17,17,17,.84)`).
    pub fill: Option<Color32>,
    /// `border-left: 1px solid #242424`.
    pub border_left: bool,
    /// `border-right: 1px solid #242424`.
    pub border_right: bool,
    /// Vertical padding.
    pub pad_y: f32,
    /// Horizontal padding.
    pub pad_x: f32,
    /// `gap` between children (`flex-direction: column`).
    pub gap: f32,
    /// `overflow: auto` (the column scrolls on its own).
    pub scroll: bool,
}

impl Column {
    /// A side panel: `flex: 0 0 <basis>`, aside fill, `padding: 16px`, `gap: 14px`, scrolls.
    pub const fn aside(label: &'static str, basis: f32) -> Self {
        Self {
            label,
            grow: 0.0,
            shrink: 0.0,
            basis,
            max: None,
            fill: Some(t::SIDE),
            border_left: false,
            border_right: false,
            pad_y: 16.0,
            pad_x: 16.0,
            gap: 14.0,
            scroll: true,
        }
    }

    /// The right-hand readout panel: `flex: 1 0 320px; max-width: 360px; border-left`.
    pub const fn readout(label: &'static str) -> Self {
        let mut c = Self::aside(label, t::size::READOUT_MIN);
        c.grow = 1.0;
        c.max = Some(t::size::READOUT_MAX);
        c.border_left = true;
        c
    }

    /// The centre column: `flex: 999 1 560px`, no padding, no fill (it holds the tab row and
    /// the `.graph` stage, or the Rule lab's padded content).
    pub const fn main(label: &'static str) -> Self {
        Self {
            label,
            grow: 999.0,
            shrink: 1.0,
            basis: 560.0,
            max: None,
            fill: None,
            border_left: false,
            border_right: false,
            pad_y: 0.0,
            pad_x: 0.0,
            gap: 0.0,
            scroll: false,
        }
    }

    /// `flex: grow shrink basis`.
    pub const fn flex(mut self, grow: f32, shrink: f32, basis: f32) -> Self {
        self.grow = grow;
        self.shrink = shrink;
        self.basis = basis;
        self
    }
    /// `max-width`.
    pub const fn max(mut self, max: f32) -> Self {
        self.max = Some(max);
        self
    }
    /// `padding: <y> <x>`.
    pub const fn pad(mut self, y: f32, x: f32) -> Self {
        self.pad_y = y;
        self.pad_x = x;
        self
    }
    /// `gap`.
    pub const fn gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }
    /// `border-left`.
    pub const fn border_left(mut self) -> Self {
        self.border_left = true;
        self
    }
    /// `border-right`.
    pub const fn border_right(mut self) -> Self {
        self.border_right = true;
        self
    }
    /// `overflow: auto` on (the column scrolls by itself).
    pub const fn scroll(mut self, on: bool) -> Self {
        self.scroll = on;
        self
    }
}

/// A column placed on a line: index, x offset from the row's left, width.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placed {
    /// Index into the columns.
    pub index: usize,
    /// Offset from the row's left edge.
    pub x: f32,
    /// Resolved width.
    pub width: f32,
}

/// CSS flex-wrap line breaking and width resolution for `cols` in `width` px (no gap), with
/// `max-width` clamping (the excess goes to the line's other growing columns).
pub fn resolve(width: f32, cols: &[Column]) -> Vec<Vec<Placed>> {
    let items: Vec<FlexItem> = cols
        .iter()
        .map(|c| FlexItem::new(c.grow, c.shrink, c.basis))
        .collect();
    flex_lines(width, 0.0, &items)
        .into_iter()
        .map(|line| {
            let mut widths: Vec<f32> = line.iter().map(|p| p.width).collect();
            let mut excess = 0.0;
            let mut frozen = vec![false; line.len()];
            for (k, p) in line.iter().enumerate() {
                if let Some(m) = cols[p.index].max
                    && widths[k] > m
                {
                    excess += widths[k] - m;
                    widths[k] = m;
                    frozen[k] = true;
                }
            }
            let grow_sum: f32 = line
                .iter()
                .enumerate()
                .filter(|(k, _)| !frozen[*k])
                .map(|(_, p)| cols[p.index].grow)
                .sum();
            if excess > 0.0 && grow_sum > 0.0 {
                for (k, p) in line.iter().enumerate() {
                    if !frozen[k] {
                        widths[k] += excess * cols[p.index].grow / grow_sum;
                    }
                }
            }
            let mut x = 0.0;
            line.iter()
                .zip(widths)
                .map(|(p, w)| {
                    let placed = Placed {
                        index: p.index,
                        x,
                        width: w,
                    };
                    x += w;
                    placed
                })
                .collect()
        })
        .collect()
}

/// Lays out a page row of `cols` at the cursor across the available width and calls
/// `add(ui, ctx, i)` for each column inside its padding (a top-down `Ui` with `gap` spacing,
/// scrolling if the column does). Side by side, columns fill [`Ctx::body_height`]; stacked,
/// each is the route's [`panel_height`](crate::Route::panel_height). Returns the rect used.
pub fn columns(
    ui: &mut Ui,
    ctx: &mut Ctx,
    cols: &[Column],
    mut add: impl FnMut(&mut Ui, &mut Ctx, usize),
) -> Rect {
    let origin = ui.cursor().min;
    let width = ui.available_width();
    let lines = resolve(width, cols);
    let height = if lines.len() <= 1 {
        ctx.body_height().max(MIN_PANEL_H)
    } else {
        ctx.route().panel_height().max(MIN_PANEL_H)
    };
    let mut y = origin.y;
    for line in &lines {
        for p in line {
            let c = &cols[p.index];
            let rect = Rect::from_min_size(pos2(origin.x + p.x, y), vec2(p.width, height));
            let mut col = ui.new_child(
                UiBuilder::new()
                    .id_salt(("column", p.index))
                    .max_rect(rect)
                    .layout(Layout::top_down(Align::Min))
                    .accessibility_role(Role::Region)
                    .accessibility_label(c.label),
            );
            col.set_clip_rect(rect.intersect(ui.clip_rect()));
            crate::widgets::set_bounds(&col, rect);
            paint_column(&col, rect, c);
            let inner = Rect::from_min_max(
                pos2(
                    rect.left() + c.pad_x + c.border_left as u8 as f32,
                    rect.top() + c.pad_y,
                ),
                pos2(
                    rect.right() - c.pad_x - c.border_right as u8 as f32,
                    rect.bottom() - c.pad_y,
                ),
            );
            if c.scroll {
                let mut area_ui = col.new_child(
                    UiBuilder::new()
                        .max_rect(Rect::from_min_max(
                            pos2(rect.left() + c.border_left as u8 as f32, rect.top()),
                            pos2(rect.right() - c.border_right as u8 as f32, rect.bottom()),
                        ))
                        .layout(Layout::top_down(Align::Min)),
                );
                ScrollArea::vertical()
                    .id_salt(("column-scroll", ctx.route().id(), p.index))
                    .auto_shrink([false, false])
                    .show(&mut area_ui, |ui| {
                        eframe::egui::Frame::new()
                            .inner_margin(eframe::egui::Margin::symmetric(
                                c.pad_x.round() as i8,
                                c.pad_y.round() as i8,
                            ))
                            .show(ui, |ui| {
                                ui.set_width(inner.width());
                                ui.spacing_mut().item_spacing = vec2(c.gap, c.gap);
                                add(ui, ctx, p.index);
                            });
                    });
            } else {
                let mut inner_ui = col.new_child(
                    UiBuilder::new()
                        .max_rect(inner)
                        .layout(Layout::top_down(Align::Min)),
                );
                inner_ui.spacing_mut().item_spacing = vec2(c.gap, c.gap);
                add(&mut inner_ui, ctx, p.index);
            }
        }
        y += height;
    }
    let used = Rect::from_min_max(origin, pos2(origin.x + width, y));
    lab_ui::paint::advance_block(ui, used);
    used
}

fn paint_column(ui: &Ui, rect: Rect, c: &Column) {
    let painter = ui.painter();
    if let Some(fill) = c.fill {
        painter.rect_filled(rect, CornerRadius::ZERO, fill);
    }
    let stroke = Stroke::new(1.0, t::BORDER);
    if c.border_left {
        painter.vline(rect.left() + 0.5, rect.y_range(), stroke);
    }
    if c.border_right {
        painter.vline(rect.right() - 0.5, rect.y_range(), stroke);
    }
}

/// Paints the `.graph` stage (`#0B0C0E`, 120 / 24 px grid anchored at its corner) over the rest
/// of `ui`'s rect and returns that rect.
pub fn stage(ui: &mut Ui) -> Rect {
    let rect = ui.available_rect_before_wrap();
    lab_ui::paint::paint_grid(ui.painter(), rect, rect.min, &t::GRAPH);
    rect
}

/// A Playground bench's centre column: the bench tab row (with the screen's
/// [`Screen::tab_actions`] at its right), then the `.graph` stage filling the rest, in which
/// `body` draws (its `Ui` is clipped to the stage).
pub fn bench<S: Screen + ?Sized>(
    ui: &mut Ui,
    ctx: &mut Ctx,
    screen: &mut S,
    body: impl FnOnce(&mut S, &mut Ctx, &mut Ui),
) {
    ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
    bench_tabs(ui, ctx, |slot, ctx| screen.tab_actions(ctx, slot));
    let rect = stage(ui);
    let mut stage_ui = ui.new_child(
        UiBuilder::new()
            .id_salt("stage")
            .max_rect(rect)
            .layout(Layout::top_down(Align::Min)),
    );
    stage_ui.set_clip_rect(rect.intersect(ui.clip_rect()));
    body(screen, ctx, &mut stage_ui);
    lab_ui::paint::advance_block(ui, rect);
}

#[cfg(test)]
mod tests {
    use super::*;

    const TRAY: [Column; 3] = [
        Column::aside("Cartridges and stamps", 248.0)
            .pad(14.0, 12.0)
            .border_right(),
        Column::main("Bench"),
        Column::readout("Rule and readouts"),
    ];

    /// 1440: 248 · 871.7 · 320.3 (free 312 shared 999 : 1 between main and readouts).
    #[test]
    fn tray_at_1440() {
        let lines = resolve(1440.0, &TRAY);
        assert_eq!(lines.len(), 1);
        let w: Vec<f32> = lines[0].iter().map(|p| p.width).collect();
        assert_eq!(w[0], 248.0);
        assert!((w[2] - (320.0 + 312.0 / 1000.0)).abs() < 1e-3, "{w:?}");
        assert!((w.iter().sum::<f32>() - 1440.0).abs() < 1e-3);
    }

    /// 390: three lines, each full width except the readouts, capped at 360.
    #[test]
    fn tray_stacks_at_390() {
        let lines = resolve(390.0, &TRAY);
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0][0].width, 248.0, "flex: 0 0 248px keeps its width");
        assert_eq!(lines[1][0].width, 390.0);
        assert_eq!(lines[2][0].width, 360.0, "max-width: 360px");
    }

    /// A medium window: the readouts wrap onto their own line and stop at 360.
    #[test]
    fn readouts_wrap_and_cap() {
        let lines = resolve(1000.0, &TRAY);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[1][0].width, 360.0);
    }

    /// Rule lab: 300 · 999 1 560 · 1 0 340 max 380 at 1440.
    #[test]
    fn rule_lab_at_1440() {
        let cols = [
            Column::aside("Rule spec", 300.0).border_right(),
            Column::main("Run").pad(16.0, 16.0),
            Column::readout("Script and analysis")
                .flex(1.0, 0.0, 340.0)
                .max(380.0),
        ];
        let lines = resolve(1440.0, &cols);
        assert_eq!(lines.len(), 1);
        assert!((lines[0][2].width - (340.0 + 240.0 / 1000.0)).abs() < 1e-3);
    }
}
