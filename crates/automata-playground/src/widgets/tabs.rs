//! The bench tab row (NAV-2), drawn to the mockup's `min-height: 44px; display: flex;
//! align-items: center; gap: 18px; padding: 0 14px; border-bottom: 1px solid #242424;
//! background: rgba(12,12,12,.8); flex-wrap: wrap` with the `.tab` links (Inter 500 12 px,
//! 44 px tall, `padding: 0 2px`, 18 px apart; the current one `#F5F5F5` with a 2 px accent
//! underline) and the screen's own items after them — the first pushed right with
//! `margin-left: auto` (Tray: the tool group, Random soup, Clear; Loom / Terrarium: a hint).

use super::flow::{FlowItem, Remembered, flow};
use super::text::{galley, text_top};
use super::{hit, interact};
use crate::ctx::Ctx;
use crate::router::Route;
use ap_theme as t;
use eframe::egui::{
    Align, CornerRadius, Id, Layout, Rect, Sense, Stroke, Ui, UiBuilder, Vec2, WidgetInfo,
    accesskit::Role, pos2, vec2,
};
use lab_ui::paint::{advance_block, focus_ring_if};

/// Row padding (left / right).
pub const PAD_X: f32 = 14.0;
/// Gap between items and between the tabs.
pub const GAP: f32 = 18.0;

/// The items a screen puts at the right of the tab row, each drawn by one call (so each can
/// wrap onto its own line like the mockup's flex items).
pub struct RowSlot<'u> {
    bar: &'u mut Ui,
    rects: Vec<Rect>,
    next: usize,
    measured: Remembered,
    max_w: f32,
}

impl RowSlot<'_> {
    /// Adds one item (a button, a segmented group, a text). Its size is measured and
    /// remembered for the next layout pass.
    pub fn add<R>(&mut self, add: impl FnOnce(&mut Ui) -> R) -> R {
        let i = self.next;
        self.next += 1;
        // items not measured yet start at the right edge (re-laid out next pass)
        let rect =
            self.rects.get(i).copied().unwrap_or_else(|| {
                Rect::from_min_size(self.bar.max_rect().right_top(), Vec2::ZERO)
            });
        let mut child = self.bar.new_child(
            UiBuilder::new()
                .id_salt(("tab-slot", i))
                .max_rect(Rect::from_min_size(
                    rect.min,
                    vec2(self.max_w.max(rect.width()), rect.height().max(28.0)),
                ))
                .layout(Layout::left_to_right(Align::Center)),
        );
        child.spacing_mut().item_spacing = Vec2::ZERO;
        let r = add(&mut child);
        let size = if child.min_rect().is_positive() {
            child.min_rect().size()
        } else {
            Vec2::ZERO
        };
        self.measured.measure(size);
        r
    }

    /// The mockup's hint text (12 px muted): "Each row is woven from the one above."
    pub fn hint(&mut self, text: &str) {
        self.add(|ui| {
            let g = galley(ui, text, lab_theme::font::body(12.0), t::TEXT_MUTED);
            let (rect, resp) = ui.allocate_exact_size(g.size(), Sense::hover());
            ui.painter().galley(rect.min, g, t::TEXT_MUTED);
            let s = text.to_string();
            resp.widget_info(|| WidgetInfo::labeled(Role::Label, true, &s));
        });
    }
}

/// Draws the bench tab row at the cursor across the available width (a `TabList` named
/// "Benches"); clicking a tab navigates. `slot` adds the screen's items. Returns the row's
/// rect (border included).
pub fn bench_tabs(ui: &mut Ui, ctx: &mut Ctx, slot: impl FnOnce(&mut RowSlot, &mut Ctx)) -> Rect {
    let route = ctx.route();
    let outer = ui.available_rect_before_wrap();
    let (left, top, width) = (outer.left(), outer.top(), outer.width());
    let inner_w = (width - 2.0 * PAD_X).max(0.0);
    let font = lab_theme::font::medium(12.0);
    let tab_ws: Vec<f32> = Route::BENCHES
        .iter()
        .map(|r| {
            galley(ui, r.tab_label().unwrap_or_default(), font.clone(), t::TEXT)
                .size()
                .x
                + 4.0
        })
        .collect();
    let tabs_w = tab_ws.iter().sum::<f32>() + GAP * (tab_ws.len() - 1) as f32;
    let slot_id = Id::unique("ap-tab-slot").with(route);
    let remembered = Remembered::load(ui.ctx(), slot_id);
    let mut items = vec![FlowItem::fixed(vec2(tabs_w, t::size::TAB_ROW))];
    for i in 0..remembered.len() {
        let it = FlowItem::fixed(remembered.get(i, Vec2::ZERO));
        items.push(if i == 0 { it.auto() } else { it });
    }
    let lay = flow(&items, inner_w, vec2(GAP, GAP), t::size::TAB_ROW);
    let row = Rect::from_min_size(pos2(left, top), vec2(width, lay.height + 1.0));
    let painter = ui.painter();
    painter.rect_filled(row, CornerRadius::ZERO, t::BAR);
    painter.hline(
        row.x_range(),
        row.bottom() - 0.5,
        Stroke::new(1.0, t::BORDER),
    );
    let origin = vec2(left + PAD_X, top);

    // ---- tabs
    let tabs_rect = lay.rects[0].translate(origin);
    let nav = ui.new_child(
        UiBuilder::new()
            .id_salt("ap-bench-tabs")
            .max_rect(tabs_rect)
            .accessibility_role(Role::TabList)
            .accessibility_label("Benches"),
    );
    let mut x = tabs_rect.left();
    for (i, r) in Route::BENCHES.into_iter().enumerate() {
        let tr = Rect::from_min_size(pos2(x, tabs_rect.top()), vec2(tab_ws[i], t::size::TAB_ROW));
        x += tab_ws[i] + GAP;
        let on = r == route;
        let label = r.tab_label().unwrap_or_default();
        let resp = interact(
            &nav,
            nav.scope_id().with(i),
            hit::target(tr).intersect(tr.expand2(vec2(GAP / 2.0, 0.0))),
            true,
            Role::Tab,
            label,
            None,
        );
        nav.ctx()
            .accesskit_node_builder(resp.id, |b| b.set_selected(on));
        let fg = if on {
            t::TEXT_STRONG
        } else if resp.hovered() {
            t::TEXT
        } else {
            t::TEXT_MUTED
        };
        let g = galley(&nav, label, font.clone(), fg);
        nav.painter()
            .galley(pos2(tr.left() + 2.0, text_top(tr.center().y, &g)), g, fg);
        if on {
            let underline = Rect::from_min_max(pos2(tr.left(), tr.bottom() - 2.0), tr.max);
            nav.painter()
                .rect_filled(underline, CornerRadius::ZERO, t::ACCENT);
        }
        focus_ring_if(
            &nav,
            &resp,
            tr.shrink2(vec2(0.0, 8.0)),
            CornerRadius::same(4),
        );
        if resp.clicked() && !on {
            ctx.go(r);
        }
    }

    // ---- the screen's items
    let rects: Vec<Rect> = lay.rects[1..].iter().map(|r| r.translate(origin)).collect();
    let mut bar_ui = ui.new_child(UiBuilder::new().id_salt("ap-tab-row").max_rect(row));
    let mut s = RowSlot {
        bar: &mut bar_ui,
        rects,
        next: 0,
        measured: remembered,
        max_w: inner_w,
    };
    slot(&mut s, ctx);
    let RowSlot { measured, .. } = s;
    measured.store(ui.ctx(), "tab row items resized");
    advance_block(ui, row);
    row
}
