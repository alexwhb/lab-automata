//! Series (SCR-5, SER-1) — mockup `docs/design/mockup/Series.dc.html`: "Same family, one new
//! accent". The four app identity cards with their accent / ink / tint / edge ramps, the
//! cell-age scale, "Shared, unchanged" and the cell-glyph icon family.
//!
//! NAV-3: [`App::open_series`](crate::App::open_series) highlights an app's card (2 px ring in
//! its accent, scrolled into view, AccessKit `selected`); the header has no app switcher for
//! now, so nothing in the window links here. Each sibling card's tag is its Open button
//! ("Open" → the injectable launcher; Bio Playground's reads "Planned" and is disabled);
//! the launcher's answer is reported with a toast.

use super::Screen;
use crate::ctx::Ctx;
use crate::launcher::{AppId, Availability, Launch};
use crate::widgets::text::{
    css_line_height, galley, lbl_galley, paint_line, text_top, wrapped, wrapped_job,
};
use crate::widgets::{CellGlyph, hit, lbl_colored};
use ap_theme as t;
use eframe::egui::{
    Align, Color32, CornerRadius, Frame, Id, Layout, Margin, Rect, Sense, Stroke, StrokeKind, Ui,
    UiBuilder, WidgetInfo, accesskit::Role, epaint::Shadow, pos2, vec2,
};
use lab_ui::layout::{auto_fit, column_width};
use lab_ui::paint::focus_ring_if;

/// The Series screen's state.
#[derive(Debug, Default)]
pub struct State {
    /// The app card to highlight (set by `App::open_series`, NAV-3).
    pub highlight: Option<AppId>,
    scrolled: Option<AppId>,
}

impl State {
    /// What the last Open did, for tests: see `AutomataApp::toasts`.
    fn open(&mut self, app: AppId, ctx: &mut Ctx) {
        let result = ctx.launch(app);
        ctx.toast(result.message(app));
        if let Launch::Failed(e) = &result {
            log::warn!("open {}: {e}", app.name());
        }
    }
}

/// The page padding (`48px 56px`; tighter on phones).
fn padding(width: f32) -> (f32, f32) {
    if width < t::size::NARROW {
        (32.0, 16.0)
    } else {
        (48.0, 56.0)
    }
}

impl Screen for State {
    fn enter(&mut self, _ctx: &mut Ctx) {
        self.scrolled = None;
    }

    fn ui(&mut self, ctx: &mut Ctx, ui: &mut Ui) {
        let width = ui.available_width();
        let (pad_y, pad_x) = padding(width);
        Frame::new()
            .inner_margin(Margin::symmetric(pad_x as i8, pad_y as i8))
            .show(ui, |ui| {
                ui.set_width(width - 2.0 * pad_x);
                // `flex-direction: column; gap: 24px`
                ui.spacing_mut().item_spacing = vec2(0.0, 24.0);
                intro(ui);
                self.identities(ctx, ui);
                lower_row(ui);
            });
    }

    fn describe(&self) -> String {
        match self.highlight {
            Some(app) => format!(
                "Series: the lab series' identities, {} highlighted",
                app.name()
            ),
            None => "Series: the lab series' identities".into(),
        }
    }
}

/// Eyebrow, H1 and paragraph (`gap: 8px`).
fn intro(ui: &mut Ui) {
    let width = ui.available_width();
    let lbl = lbl_galley(ui, "SPICE-for-X series · fourth bench", t::TEXT_MUTED);
    let h1 = wrapped_job(
        ui,
        "Same family, one new accent",
        lab_theme::font::heading(36.0),
        t::TEXT,
        width,
        Some(39.6),
        -0.72,
        false,
    );
    let p = wrapped(
        ui,
        "Shares the greys, grid ground, header and 2.5D bench. Adds one magenta ramp, a cell-age scale and cell-glyph icons.",
        lab_theme::font::body(15.0),
        t::TEXT_DIM,
        width.min(880.0),
        Some(23.25),
    );
    let lbl_h = css_line_height(ui, &lab_theme::font::semibold(11.0));
    let h = lbl_h + 8.0 + h1.size().y + 8.0 + p.size().y;
    let (rect, _) = ui.allocate_exact_size(vec2(width, h), Sense::hover());
    let painter = ui.painter();
    let mut y = rect.top();
    paint_line(painter, &lbl, rect.left(), y, lbl_h, t::TEXT_MUTED);
    y += lbl_h + 8.0;
    painter.galley(pos2(rect.left(), y), h1.clone(), t::TEXT);
    y += h1.size().y + 8.0;
    painter.galley(pos2(rect.left(), y), p.clone(), t::TEXT_DIM);
    // accessible text: the eyebrow, the heading and the paragraph
    let labels = [
        (Role::Label, "SPICE-for-X series · fourth bench"),
        (Role::Heading, "Same family, one new accent"),
        (
            Role::Label,
            "Shares the greys, grid ground, header and 2.5D bench. Adds one magenta ramp, a cell-age scale and cell-glyph icons.",
        ),
    ];
    for (i, (role, text)) in labels.into_iter().enumerate() {
        let r = ui.interact(rect, ui.scope_id().with(("intro", i)), Sense::hover());
        r.widget_info(|| WidgetInfo::labeled(role, true, text));
    }
}

impl State {
    /// The four identity cards (`repeat(4, minmax(0, 1fr))`, gap 16; fewer columns when
    /// narrow).
    fn identities(&mut self, ctx: &mut Ctx, ui: &mut Ui) {
        let width = ui.available_width();
        let (cols, _) = auto_fit(width, 280.0, 16.0, 4);
        let cw = column_width(width, 16.0, cols);
        let origin = ui.cursor().min;
        let section = Rect::from_min_size(origin, vec2(width, 0.0));
        let mut sec = ui.new_child(
            UiBuilder::new()
                .id_salt("identities")
                .max_rect(section.with_max_y(f32::INFINITY))
                .accessibility_role(Role::Group)
                .accessibility_label("App identities"),
        );
        let mut y = origin.y;
        let mut bottom = origin.y;
        for (i, app) in AppId::ALL.into_iter().enumerate() {
            let (c, r) = (i % cols, i / cols);
            if c == 0 && r > 0 {
                y = bottom + 16.0;
            }
            let rect =
                Rect::from_min_size(pos2(origin.x + c as f32 * (cw + 16.0), y), vec2(cw, 0.0));
            let used = self.identity_card(ctx, &mut sec, rect, app);
            bottom = bottom.max(used.bottom());
        }
        let used = Rect::from_min_max(origin, pos2(origin.x + width, bottom));
        crate::widgets::set_bounds(&sec, used);
        ui.advance_cursor_after_rect(used);
    }

    /// One identity card at `rect.min`, `rect.width()` wide; returns the rect used.
    fn identity_card(&mut self, ctx: &mut Ctx, ui: &mut Ui, rect: Rect, app: AppId) -> Rect {
        let id = app.identity();
        let highlighted = self.highlight == Some(app);
        let mut card = ui.new_child(
            UiBuilder::new()
                .id_salt(("identity", app.name()))
                .max_rect(rect.with_max_y(f32::INFINITY))
                .layout(Layout::top_down(Align::Min))
                .accessibility_role(Role::Group)
                .accessibility_label(app.name()),
        );
        ui.ctx()
            .accesskit_node_builder(card.unique_id(), |b| b.set_selected(highlighted));
        let border_w = if highlighted { 2.0 } else { 1.0 };
        let border = if highlighted { id.accent } else { id.edge };
        let inner = Frame::new()
            .fill(t::SIDE)
            .stroke(Stroke::new(border_w, border))
            .corner_radius(CornerRadius::same(t::radius::CARD))
            .inner_margin(Margin::same((19.0 - border_w) as i8))
            .show(&mut card, |ui| {
                ui.set_width(rect.width() - 36.0);
                ui.spacing_mut().item_spacing = vec2(0.0, 12.0);
                self.identity_head(ctx, ui, app);
                ramp(ui, &id);
            });
        let used = inner.response.rect;
        crate::widgets::set_bounds(&card, used);
        if highlighted {
            // a soft halo in the app's accent around the ring
            let halo = Shadow {
                offset: [0, 0],
                blur: 18,
                spread: 0,
                color: id.accent.gamma_multiply(0.35),
            };
            ui.painter()
                .add(halo.as_shape(used, CornerRadius::same(t::radius::CARD)));
            if self.scrolled != Some(app) {
                self.scrolled = Some(app);
                ui.scroll_to_rect(used, Some(Align::Center));
            }
        }
        used
    }

    /// Swatch frame · name / domain · the tag (Open button for siblings).
    fn identity_head(&mut self, ctx: &mut Ctx, ui: &mut Ui, app: AppId) {
        let id = app.identity();
        let width = ui.available_width();
        let name_font = lab_theme::font::heading(17.0);
        let domain_font = lab_theme::font::body(12.0);
        let name_h = css_line_height(ui, &name_font);
        let domain_h = css_line_height(ui, &domain_font);
        let block_h = name_h + 2.0 + domain_h;
        // `align-items: center`: the row is as tall as its tallest part
        let (rect, _) = ui.allocate_exact_size(vec2(width, block_h.max(34.0)), Sense::hover());
        let painter = ui.painter();
        // 34 × 34 swatch frame: radius 8, 1.5 px accent border, #171717, a 12 px accent square
        let frame =
            Rect::from_min_size(pos2(rect.left(), rect.center().y - 17.0), vec2(34.0, 34.0));
        painter.rect(
            frame,
            CornerRadius::same(8),
            t::LOGO_BODY,
            Stroke::new(1.5, id.accent),
            StrokeKind::Inside,
        );
        painter.rect_filled(
            Rect::from_center_size(frame.center(), vec2(12.0, 12.0)),
            CornerRadius::same(3),
            id.accent,
        );
        // the tag / Open button, right-aligned
        let tag_text = match app.availability() {
            Availability::Released => "Open",
            Availability::Planned => "Planned",
            Availability::Current => app.tag(),
        };
        let tag_g = galley(ui, tag_text, lab_theme::font::body(11.0), id.ink);
        let tag_line = css_line_height(ui, &lab_theme::font::body(11.0));
        let tag_size = vec2(tag_g.size().x + 16.0 + 2.0, tag_line + 6.0 + 2.0);
        let tag = Rect::from_min_size(
            pos2(
                rect.right() - tag_size.x,
                rect.center().y - tag_size.y / 2.0,
            ),
            tag_size,
        );
        // name over domain (gap 2), centred on the row
        let text_w = (tag.left() - 10.0 - frame.right() - 10.0).max(10.0);
        let name = galley(ui, id.name, name_font, t::TEXT);
        let domain = galley(ui, id.domain, domain_font, t::TEXT_MUTED);
        let x = frame.right() + 10.0;
        let y = rect.center().y - block_h / 2.0;
        let clip = painter.with_clip_rect(Rect::from_min_size(
            pos2(x, rect.top() - 4.0),
            vec2(text_w, rect.height() + 8.0),
        ));
        paint_line(&clip, &name, x, y, name_h, t::TEXT);
        paint_line(&clip, &domain, x, y + name_h + 2.0, domain_h, t::TEXT_MUTED);
        let (name_s, domain_s) = (id.name.to_string(), id.domain.to_string());
        ui.interact(
            Rect::from_min_size(pos2(x, y), vec2(text_w, block_h)),
            ui.scope_id().with("name"),
            Sense::hover(),
        )
        .widget_info(|| WidgetInfo::labeled(Role::Label, true, format!("{name_s}, {domain_s}")));
        match app.availability() {
            Availability::Current => {
                tag_pill(ui, tag, &tag_g, &id, false);
                let s = format!("{} (this app, {})", id.name, app.tag());
                ui.interact(tag, ui.scope_id().with("tag"), Sense::hover())
                    .widget_info(|| WidgetInfo::labeled(Role::Label, true, &s));
            }
            availability => {
                let enabled = availability == Availability::Released;
                let name = if enabled {
                    format!("Open {}", id.name)
                } else {
                    format!("Open {} (planned, not built yet)", id.name)
                };
                let target = hit::target(tag);
                let resp = crate::widgets::interact(
                    ui,
                    ui.scope_id().with("open"),
                    target,
                    enabled,
                    Role::Button,
                    &name,
                    None,
                );
                tag_pill(ui, tag, &tag_g, &id, enabled && resp.hovered());
                focus_ring_if(ui, &resp, tag, CornerRadius::same(4));
                if resp.clicked() {
                    self.open(app, ctx);
                }
            }
        }
    }
}

/// The tag pill: 11 px, `padding: 3px 8px`, radius 4, edge border, ink on tint.
fn tag_pill(
    ui: &Ui,
    rect: Rect,
    g: &std::sync::Arc<eframe::egui::Galley>,
    id: &t::Identity,
    hovered: bool,
) {
    let painter = ui.painter();
    let fill = if hovered {
        t::mix(id.tint, id.edge, 0.35)
    } else {
        id.tint
    };
    painter.rect(
        rect,
        CornerRadius::same(4),
        fill,
        Stroke::new(1.0, id.edge),
        StrokeKind::Inside,
    );
    painter.galley(
        pos2(
            rect.center().x - g.size().x / 2.0,
            text_top(rect.center().y, g),
        ),
        g.clone(),
        id.ink,
    );
}

/// The four swatches (`repeat(4, minmax(0, 1fr))`, gap 6): a 36 px chip, the role, the hex.
fn ramp(ui: &mut Ui, id: &t::Identity) {
    let width = ui.available_width();
    let cw = column_width(width, 6.0, 4);
    let role_font = lab_theme::font::body(11.0);
    let hex_font = lab_theme::font::mono(10.5);
    let role_h = css_line_height(ui, &role_font);
    let hex_h = css_line_height(ui, &hex_font);
    // `height: 36px; border: 1px` on a content-box span: 38 px tall
    let h = 38.0 + 4.0 + role_h + 4.0 + hex_h;
    let (rect, _) = ui.allocate_exact_size(vec2(width, h), Sense::hover());
    let swatches = [
        ("Accent", id.accent),
        ("Ink", id.ink),
        ("Tint", id.tint),
        ("Edge", id.edge),
    ];
    for (i, (role, color)) in swatches.into_iter().enumerate() {
        let x = rect.left() + i as f32 * (cw + 6.0);
        let chip = Rect::from_min_size(pos2(x, rect.top()), vec2(cw, 38.0));
        let painter = ui.painter();
        painter.rect(
            chip,
            CornerRadius::same(6),
            color,
            Stroke::new(1.0, t::BORDER_STRONG),
            StrokeKind::Inside,
        );
        let hex = hex_of(color);
        let rg = galley(ui, role, role_font.clone(), t::TEXT_DIM);
        let hg = galley(ui, &hex, hex_font.clone(), t::TEXT_MUTED);
        paint_line(painter, &rg, x, chip.bottom() + 4.0, role_h, t::TEXT_DIM);
        paint_line(
            painter,
            &hg,
            x,
            chip.bottom() + 4.0 + role_h + 4.0,
            hex_h,
            t::TEXT_MUTED,
        );
        let label = format!("{} {role} {hex}", id.name);
        ui.interact(chip, ui.scope_id().with(("swatch", i)), Sense::hover())
            .widget_info(|| WidgetInfo::labeled(Role::Image, true, &label));
    }
}

/// `#RRGGBB` of an opaque colour.
pub fn hex_of(c: Color32) -> String {
    let [r, g, b, _] = c.to_srgba_unmultiplied();
    format!("#{r:02X}{g:02X}{b:02X}")
}

/// Cell-age scale · Shared, unchanged · Icon family (`repeat(3, minmax(0, 1fr))`, gap 16,
/// rows stretch to the tallest card).
fn lower_row(ui: &mut Ui) {
    let width = ui.available_width();
    let (cols, _) = auto_fit(width, 300.0, 16.0, 3);
    let cw = column_width(width, 16.0, cols);
    let origin = ui.cursor().min;
    let row_id = Id::unique("series-lower-row");
    let remembered: Vec<f32> = ui.data(|d| d.get_temp(row_id)).unwrap_or_default();
    let mut heights = Vec::with_capacity(3);
    let mut y = origin.y;
    let mut row_bottom = origin.y;
    for i in 0..3 {
        let (c, r) = (i % cols, i / cols);
        if c == 0 && r > 0 {
            y = row_bottom + 16.0;
        }
        // stretch: every card in a row is as tall as the row's tallest (last pass)
        let row_cards: Vec<usize> = (0..3).filter(|k| k / cols == r).collect();
        let stretch = row_cards
            .iter()
            .filter_map(|k| remembered.get(*k))
            .copied()
            .fold(0.0, f32::max);
        let rect = Rect::from_min_size(
            pos2(origin.x + c as f32 * (cw + 16.0), y),
            vec2(cw, stretch),
        );
        let (natural, used) = lower_card(ui, i, rect);
        heights.push(natural);
        row_bottom = row_bottom.max(used.bottom());
    }
    if heights.len() != remembered.len()
        || heights
            .iter()
            .zip(&remembered)
            .any(|(a, b)| (a - b).abs() > 0.5)
    {
        ui.data_mut(|d| d.insert_temp(row_id, heights));
        ui.ctx().request_discard("series card heights changed");
    }
    ui.advance_cursor_after_rect(Rect::from_min_max(
        origin,
        pos2(origin.x + width, row_bottom),
    ));
}

/// Draws lower card `i` at `rect` (min height `rect.height()`); returns (natural height, rect).
fn lower_card(ui: &mut Ui, i: usize, rect: Rect) -> (f32, Rect) {
    let (label, border) = match i {
        0 => ("Cell-age scale", t::BORDER),
        1 => ("Shared with the series", t::BORDER),
        _ => ("Icon family", t::EDGE),
    };
    let mut card = ui.new_child(
        UiBuilder::new()
            .id_salt(("lower", i))
            .max_rect(Rect::from_min_size(
                rect.min,
                vec2(rect.width(), f32::INFINITY),
            ))
            .layout(Layout::top_down(Align::Min))
            .accessibility_role(Role::Group)
            .accessibility_label(label),
    );
    let mut natural = 0.0;
    let r = Frame::new()
        .fill(t::SIDE)
        .stroke(Stroke::new(1.0, border))
        .corner_radius(CornerRadius::same(t::radius::CARD))
        .inner_margin(Margin::same(18))
        .show(&mut card, |ui| {
            ui.set_width(rect.width() - 36.0);
            ui.spacing_mut().item_spacing = vec2(0.0, 12.0);
            let top = ui.cursor().top();
            match i {
                0 => cell_age(ui),
                1 => shared(ui),
                _ => icon_family(ui),
            }
            natural = ui.cursor().top() - top - 12.0 + 36.0;
            // grid rows stretch their items: grow to the row's height (never shrink)
            let min_inner = (rect.height() - 36.0).max(0.0);
            let left = ui.min_rect().left();
            ui.expand_to_include_rect(Rect::from_min_size(pos2(left, top), vec2(1.0, min_inner)));
        });
    crate::widgets::set_bounds(&card, r.response.rect);
    (natural, r.response.rect)
}

/// "Cell-age scale · automata only": five 40 px swatches (live ones lift on their 6 px side,
/// the newborn glows), their names and hexes, and the note.
fn cell_age(ui: &mut Ui) {
    lbl_colored(ui, "Cell-age scale · automata only", t::INK);
    let name_font = lab_theme::font::body(11.0);
    let hex_font = lab_theme::font::mono(10.5);
    let items: [(&str, Color32, Option<Color32>, bool, bool); 5] = [
        ("Newborn", t::NEWBORN.face, t::NEWBORN.side, true, false),
        ("Alive", t::ALIVE.face, t::ALIVE.side, false, false),
        ("Elder", t::ELDER.face, t::ELDER.side, false, false),
        ("Trail", t::TRAIL[2], None, false, false),
        ("Empty", t::EMPTY, None, false, true),
    ];
    let name_h = css_line_height(ui, &name_font);
    let hex_h = css_line_height(ui, &hex_font);
    // swatch 40, gap 6 + margin-top 6, name, gap 6, hex
    let h = 40.0 + 12.0 + name_h + 6.0 + hex_h;
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), h), Sense::hover());
    let mut x = rect.left();
    for (name, face, side, glow, border) in items {
        let hex = hex_of(face);
        let ng = galley(ui, name, name_font.clone(), t::TEXT_DIM);
        let hg = galley(ui, &hex, hex_font.clone(), t::TEXT_MUTED);
        let col_w = 40.0f32.max(ng.size().x).max(hg.size().x);
        let cx = x + col_w / 2.0;
        let sw = Rect::from_center_size(pos2(cx, rect.top() + 20.0), vec2(40.0, 40.0));
        let painter = ui.painter();
        if glow {
            let s = Shadow {
                offset: [0, 0],
                blur: 16,
                spread: 0,
                color: t::rgba(255, 170, 232, 0.7),
            };
            painter.add(s.as_shape(sw, CornerRadius::same(8)));
        }
        if let Some(side) = side {
            painter.rect_filled(sw.translate(vec2(0.0, 6.0)), CornerRadius::same(8), side);
        }
        if border {
            painter.rect(
                sw,
                CornerRadius::same(8),
                face,
                Stroke::new(1.0, t::BORDER_STRONG),
                StrokeKind::Inside,
            );
        } else {
            painter.rect_filled(sw, CornerRadius::same(8), face);
        }
        let ny = sw.bottom() + 12.0;
        paint_line(
            painter,
            &ng,
            cx - ng.size().x / 2.0,
            ny,
            name_h,
            t::TEXT_DIM,
        );
        paint_line(
            painter,
            &hg,
            cx - hg.size().x / 2.0,
            ny + name_h + 6.0,
            hex_h,
            t::TEXT_MUTED,
        );
        let label = format!("{name} {hex}");
        ui.interact(sw, ui.scope_id().with(("age", name)), Sense::hover())
            .widget_info(|| WidgetInfo::labeled(Role::Image, true, &label));
        x += col_w + 10.0;
    }
    crate::widgets::text::text_block(
        ui,
        "Reads in greyscale too. Live cells lift 6 px off the tray.",
        lab_theme::font::body(12.0),
        t::TEXT_MUTED,
        Some(18.0),
    );
}

/// "Shared, unchanged": five lines, 13 px / 1.45, 8 px apart.
fn shared(ui: &mut Ui) {
    crate::widgets::lbl(ui, "Shared, unchanged");
    ui.scope(|ui| {
        ui.spacing_mut().item_spacing = vec2(0.0, 8.0);
        for line in [
            "Neutrals, grid ground, tilted bench",
            "Header, mode switch, Run · Step",
            "Parts palette (here: rules and stamps)",
            "Advanced mode as a netlist-style script",
            "Space Grotesk, Inter, JetBrains Mono",
        ] {
            crate::widgets::text::text_block(
                ui,
                line,
                lab_theme::font::body(13.0),
                t::TEXT_DIM,
                Some(18.85),
            );
        }
    });
}

/// "Icon family · cell glyphs": five 48 px tiles with the 27 px glyphs, and the note.
fn icon_family(ui: &mut Ui) {
    lbl_colored(ui, "Icon family · cell glyphs", t::INK);
    let names = ["Glider", "Blinker", "Block", "Ring", "Corners"];
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 48.0), Sense::hover());
    for (i, g) in CellGlyph::FAMILY.iter().enumerate() {
        let tile = Rect::from_min_size(
            pos2(rect.left() + i as f32 * 58.0, rect.top()),
            vec2(48.0, 48.0),
        );
        let painter = ui.painter();
        painter.rect(
            tile,
            CornerRadius::same(8),
            t::CARD,
            Stroke::new(1.0, t::BORDER),
            StrokeKind::Inside,
        );
        g.paint(
            painter,
            Rect::from_center_size(tile.center(), vec2(27.0, 27.0)),
        );
        let label = format!("{} glyph", names[i]);
        ui.interact(tile, ui.scope_id().with(("glyph", i)), Sense::hover())
            .widget_info(|| WidgetInfo::labeled(Role::Image, true, &label));
    }
    crate::widgets::text::text_block(
        ui,
        "3 × 3 cell grids: grey empty, magenta live.",
        lab_theme::font::body(12.0),
        t::TEXT_MUTED,
        Some(18.0),
    );
}
