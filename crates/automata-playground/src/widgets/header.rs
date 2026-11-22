//! The header (NAV-1 / NAV-3), drawn to the mockup's `<header>`: `min-height: 56px; display:
//! flex; flex-wrap: wrap; align-items: center; gap: 16px; padding: 0 16px; border-bottom:
//! 1px solid #242424; background: rgba(12,12,12,.8)` holding
//!
//! 1. the brand — the 26 px logo, "Automata Playground" (Space Grotesk 600 16 px,
//!    `letter-spacing: -.01em`), a `#4A4A4A` "/", the document name (13 px `#D9D9D9`); an
//!    AccessKit `Link` to the Tray;
//! 2. the mode switcher (`margin-left: 12px`): Playground (cube) · Advanced (lines) links in a
//!    `rgba(6,6,6,.88)` well, `Navigation` named "Mode";
//! 3. the app switcher: Breadbox, Chem Playground, Bio Playground, Automata Playground
//!    (current, `.sw-on`) — 32 px squares with the 16 px glyphs, a `Group` named "Switch app";
//! 4. the screen's actions (`margin-left: auto; flex-wrap: wrap; gap: 8px`).
//!
//! At 390 px each part wraps onto its own row exactly like the browser (see `flow`).

use super::flow::{FlowItem, Remembered, flow};
use super::icons::Icon;
use super::seg::{SegGroup, SegItem, SegStyle};
use super::text::{galley, galley_spaced, text_top};
use super::{hit, interact};
use crate::ctx::Ctx;
use crate::launcher::AppId;
use crate::router::{Mode, Route};
use ap_theme as t;
use eframe::egui::{
    Align, CornerRadius, Id, Layout, Rect, Stroke, StrokeKind, Ui, UiBuilder, accesskit::Role,
    pos2, vec2,
};
use lab_ui::paint::{advance_block, focus_ring_if};

/// Header padding (left / right).
pub const PAD_X: f32 = 16.0;
/// Gap between the header's parts (both directions).
pub const GAP: f32 = 16.0;
/// Gap between the actions.
pub const SLOT_GAP: f32 = 8.0;

const MODES: [SegItem<'static>; 2] = [
    SegItem::new("Playground").icon(Icon::Cube),
    SegItem::new("Advanced").icon(Icon::Lines),
];

/// The mode switcher's group for `route`.
fn mode_group(route: Route) -> SegGroup<'static> {
    let current = route
        .mode()
        .and_then(|m| Mode::ALL.iter().position(|x| *x == m));
    SegGroup::new("Mode", &MODES, current)
        .style(SegStyle::Header)
        .links()
        .navigation()
}

/// Draws the header across the full width at the cursor and advances past it. `actions` fills
/// the right-hand slot (left to right, 8 px apart, wrapping; make each item 32 px tall).
/// Returns the header's rect (border included).
pub fn header(ui: &mut Ui, ctx: &mut Ctx, actions: impl FnOnce(&mut Ui, &mut Ctx)) -> Rect {
    let route = ctx.route();
    let outer = ui.available_rect_before_wrap();
    let (left, top, width) = (outer.left(), outer.top(), outer.width());
    let inner_w = (width - 2.0 * PAD_X).max(0.0);

    // ---- measure
    let brand_font = lab_theme::font::heading(16.0);
    let word = galley_spaced(ui, crate::APP_NAME, brand_font, t::TEXT, -0.16);
    let slash = galley(ui, "/", lab_theme::font::body(16.0), t::SLASH);
    let doc = galley(
        ui,
        route.document(),
        lab_theme::font::body(13.0),
        t::TEXT_DIM,
    );
    let brand_w = 26.0 + 10.0 + word.size().x + 10.0 + slash.size().x + 10.0 + doc.size().x;
    let modes = mode_group(route);
    let modes_size = modes.one_row_size(ui);
    let switch_size = vec2(2.0 + 6.0 + 4.0 * 32.0 + 3.0 * 2.0, 40.0);
    let slot_id = Id::unique("ap-header-slot").with(route);
    let remembered = Remembered::load(ui.ctx(), slot_id);
    // never measured: assume it needs its own line (then it's measured unwrapped)
    let slot = remembered.get(0, vec2(f32::INFINITY, t::size::BUTTON));
    let items = [
        FlowItem::fixed(vec2(brand_w, 26.0)),
        FlowItem::fixed(modes_size).margin(12.0),
        FlowItem::fixed(switch_size),
        FlowItem::fixed(slot).auto().shrink(),
    ];
    let lay = flow(&items, inner_w, vec2(GAP, GAP), t::size::HEADER);
    let at = |i: usize| lay.rects[i].translate(vec2(left + PAD_X, top));
    let bar = Rect::from_min_size(pos2(left, top), vec2(width, lay.height + 1.0));

    // ---- paint the bar
    let painter = ui.painter();
    painter.rect_filled(bar, CornerRadius::ZERO, t::BAR);
    painter.hline(
        bar.x_range(),
        bar.bottom() - 0.5,
        Stroke::new(1.0, t::BORDER),
    );
    let mut bar_ui = ui.new_child(
        UiBuilder::new()
            .id_salt("ap-header")
            .max_rect(bar)
            .accessibility_role(Role::Banner),
    );
    super::set_bounds(&bar_ui, bar);

    // ---- brand (a link home: the Tray)
    let brand = at(0);
    let resp = interact(
        &bar_ui,
        bar_ui.scope_id().with("brand"),
        hit::target(brand),
        true,
        Role::Link,
        crate::APP_NAME,
        None,
    );
    let cy = brand.center().y;
    let p = bar_ui.painter();
    let logo = Rect::from_min_size(pos2(brand.left(), cy - 13.0), vec2(26.0, 26.0));
    Icon::Logo.paint(p, logo, t::ACCENT);
    let mut x = logo.right() + 10.0;
    p.galley(pos2(x, text_top(cy, &word)), word.clone(), t::TEXT);
    x += word.size().x + 10.0;
    p.galley(pos2(x, text_top(cy, &slash)), slash.clone(), t::SLASH);
    x += slash.size().x + 10.0;
    let doc_rect = Rect::from_min_size(pos2(x, cy - doc.size().y / 2.0), doc.size());
    p.galley(pos2(x, text_top(cy, &doc)), doc, t::TEXT_DIM);
    focus_ring_if(&bar_ui, &resp, brand, CornerRadius::same(6));
    if resp.clicked() {
        ctx.go(Route::Tray);
    }
    // the document name, as text for assistive tech (NAV-1)
    let document = route.document();
    bar_ui
        .interact(
            doc_rect,
            bar_ui.scope_id().with("document"),
            eframe::egui::Sense::hover(),
        )
        .widget_info(|| eframe::egui::WidgetInfo::labeled(Role::Label, true, document));

    // ---- mode switcher
    let mut mode_ui = bar_ui.new_child(UiBuilder::new().id_salt("modes").max_rect(at(1)));
    if let Some(i) = modes.show(&mut mode_ui).clicked {
        ctx.go(match Mode::ALL[i] {
            Mode::Playground => ctx.last_bench(),
            Mode::Advanced => Route::RuleLab,
        });
    }

    // ---- app switcher
    if let Some(app) = app_switcher(&mut bar_ui, at(2)) {
        match app {
            AppId::Automata => ctx.go(Route::Tray),
            other => ctx.open_series(other),
        }
    }

    // ---- the screen's actions
    let slot_rect = at(3);
    let own_line = lay.lines.last().is_some_and(|l| l == &vec![3]);
    let max_w = if own_line {
        inner_w
    } else {
        // room to grow a little before wrapping; a real change re-runs the pass
        slot_rect.width() + PAD_X
    };
    let mut slot_ui = bar_ui.new_child(
        UiBuilder::new()
            .id_salt("actions")
            // one 32 px row tall: egui centres a wrapping row in the rect's height, so taller
            // rects would push every row down; overflow rows extend below it
            .max_rect(Rect::from_min_size(
                slot_rect.min,
                vec2(max_w, t::size::BUTTON),
            ))
            .layout(Layout::left_to_right(Align::Center).with_main_wrap(true)),
    );
    slot_ui.spacing_mut().item_spacing = vec2(SLOT_GAP, SLOT_GAP);
    slot_ui.spacing_mut().interact_size.y = t::size::BUTTON;
    actions(&mut slot_ui, ctx);
    let measured = slot_ui.min_rect().size();
    let measured = if slot_ui.min_rect().is_positive() {
        measured
    } else {
        vec2(0.0, t::size::BUTTON)
    };
    let wrapped = measured.y > t::size::BUTTON + 0.5;
    let mut rem = remembered;
    rem.measure(if wrapped {
        // wider than its line: the exact width only matters as "too wide for the line"
        let line_room = if own_line { inner_w } else { slot_rect.width() };
        vec2(slot.x.max(line_room + 1.0).min(1e6), measured.y)
    } else {
        measured
    });
    rem.store(ui.ctx(), "header actions resized");

    advance_block(ui, bar);
    bar
}

/// The app switcher in `rect`: returns the app clicked.
fn app_switcher(ui: &mut Ui, rect: Rect) -> Option<AppId> {
    let group = ui.new_child(
        UiBuilder::new()
            .id_salt("switch-app")
            .max_rect(rect)
            .accessibility_role(Role::Group)
            .accessibility_label("Switch app"),
    );
    super::set_bounds(&group, rect);
    group.painter().rect(
        rect,
        CornerRadius::same(t::radius::WELL),
        t::INSET_BAR,
        Stroke::new(1.0, t::BORDER),
        StrokeKind::Inside,
    );
    let visuals: Vec<Rect> = (0..4)
        .map(|i| {
            Rect::from_min_size(
                pos2(rect.left() + 4.0 + i as f32 * 34.0, rect.top() + 4.0),
                vec2(32.0, 32.0),
            )
        })
        .collect();
    let targets = hit::tile(&visuals);
    let mut clicked = None;
    for (i, app) in AppId::ALL.into_iter().enumerate() {
        let r = visuals[i];
        let current = app == AppId::Automata;
        let resp = interact(
            &group,
            group.scope_id().with(i),
            targets[i],
            true,
            Role::Link,
            app.switcher_label(),
            None,
        );
        group
            .ctx()
            .accesskit_node_builder(resp.id, |b| b.set_selected(current));
        let fill = if current {
            Some(t::SEG_ON)
        } else if resp.hovered() {
            Some(t::SW_HOVER)
        } else {
            None
        };
        if let Some(f) = fill {
            group
                .painter()
                .rect_filled(r, CornerRadius::same(t::radius::BUTTON), f);
        }
        Icon::app(app).paint(
            group.painter(),
            Rect::from_center_size(r.center(), vec2(16.0, 16.0)),
            t::TEXT,
        );
        focus_ring_if(&group, &resp, r, CornerRadius::same(t::radius::BUTTON));
        if resp.clicked() {
            clicked = Some(app);
        }
    }
    clicked
}
