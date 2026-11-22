//! Rule lab (SCR-4, LAB-1…5) — mockup `docs/design/mockup/RuleLab.dc.html`: the series'
//! "SPICE" mode. The rule script is the netlist of a run and it is editable.
//!
//! - **Rule spec** (left, LAB-1): Family (Life-like; the others disabled, "Not in this
//!   version"), the 5 × 5 neighbourhood editor with its presets and "N cells", Born / Survives
//!   bits 0…N, Boundary (Torus / Dead edge) and Seed (R-pentomino / Acorn / Soup).
//! - **Run** (centre, LAB-2): the rule code, neighbourhood · boundary, `gen 120 / 400 · 148
//!   alive`; the top-down grid at the scrubbed generation ([`ap_bench::TopDownGrid`]); Play /
//!   Pause (loops 0 → gens, 60 ms per generation) and Scrub; the population plot with the
//!   stable band from the first repeat and the playhead.
//! - **Script and analysis** (right, LAB-3/4/5): the editable script ([`editor`]), the four
//!   analysis tiles, the verdict and the scrubbed generation's RLE.
//! - Header: "Open in Playground" (NAV-4: the Tray gets the rule and the scrubbed cells) and
//!   "Export" (Copy RLE, Copy script, Save .rle…, Save .rule…).
//!
//! Every number comes from [`ap_engine::analysis`] (DONE-2); the GUI-free state and the
//! script ⇄ spec sync live in [`model`]. The `pub` methods on [`State`] are the screen's
//! control API (the MCP bridge drives the Rule lab through them).

pub mod editor;
pub mod model;

use super::Screen;
use crate::ctx::{Carry, Command, Ctx, Ticker};
use crate::layout::{self, Column};
use crate::router::Route;
use crate::widgets::flow::{FlowItem, flow};
use crate::widgets::menu::{Menu, MenuItem, menu_button_state};
use crate::widgets::text::{galley, lbl_row, text_top, wrapped};
use crate::widgets::{
    BitRow, Card, CodeBlock, Icon, MiniButton, NbGrid, Plot, SegGroup, SegItem, SegStyle, Slider,
    StatTile, ghost, lbl, prim, verdict,
};
use ap_bench::{TopCell, TopDownGrid};
use ap_engine::{
    Boundary, CellChange, LifeRule, NeighbourhoodPreset, Report, RuleScript, RunSpec, ScriptError,
    Seed,
};
use ap_theme as t;
use eframe::egui::{
    Align, Color32, FontId, Galley, Id, Layout, Rect, Sense, Ui, UiBuilder, Vec2, WidgetInfo,
    accesskit::Role, pos2, vec2,
};
use model::Lab;
use std::sync::Arc;

/// The page row: spec 300 · run (padding 16, gap 14, scrolls) · script 1 0 340 max 380.
pub const COLUMNS: [Column; 3] = [
    Column::aside("Rule spec", 300.0).border_right(),
    Column::main("Run").pad(16.0, 16.0).gap(14.0).scroll(true),
    Column::readout("Script and analysis")
        .flex(1.0, 0.0, 340.0)
        .max(380.0),
];

/// Play's pace: the mockup's `setInterval(…, 60)`, one generation per tick.
pub const PLAY_MS: f32 = 60.0;

/// The rule families (LAB-1); only Life-like is in this version.
pub const FAMILIES: [SegItem<'static>; 4] = [
    SegItem::new("Life-like"),
    SegItem::new("Generations").enabled(false),
    SegItem::new("Larger than Life").enabled(false),
    SegItem::new("Lenia").enabled(false),
];

/// What the disabled families say (DONE-3).
pub const NOT_IN_THIS_VERSION: &str = "Not in this version";

const BOUNDARIES: [SegItem<'static>; 2] = [SegItem::new("Torus"), SegItem::new("Dead edge")];
const SEEDS: [SegItem<'static>; 3] = [
    SegItem::new("R-pentomino"),
    SegItem::new("Acorn"),
    SegItem::new("Soup"),
];

/// The Export menu.
pub const EXPORT_ITEMS: [MenuItem<'static>; 4] = [
    MenuItem::new("Copy RLE"),
    MenuItem::new("Copy script"),
    MenuItem::new("Save .rle…").group(),
    MenuItem::new("Save .rule…"),
];

/// The Rule lab's state (the app keeps it across navigation).
#[derive(Debug, Default)]
pub struct State {
    /// Play is on (the scrubber loops 0 → gens).
    pub playing: bool,
    /// The last carry-over received (NAV-4).
    pub received: Option<Carry>,
    lab: Lab,
    ticker: Ticker,
    editor_focused: bool,
    export_open: bool,
    export_focus: bool,
    export_anchor: Option<Rect>,
}

// ---- control API ---------------------------------------------------------------------------

impl State {
    /// The script as shown in the editor (may have errors; see [`State::errors`]).
    pub fn script_text(&self) -> String {
        self.lab.text().to_string()
    }

    /// Replaces the script. When it parses, its spec takes effect and runs (synchronously);
    /// otherwise nothing changes and the errors (line-numbered) are returned.
    pub fn set_script(&mut self, text: &str) -> Result<(), Vec<ScriptError>> {
        self.lab.set_script(text)
    }

    /// The spec in effect (the last valid script's).
    pub fn spec(&self) -> RunSpec {
        self.lab.spec()
    }

    /// The last valid script (document name, spec, probes).
    pub fn script(&self) -> &RuleScript {
        self.lab.script()
    }

    /// The script text's errors (empty when it parses).
    pub fn errors(&self) -> &[ScriptError] {
        self.lab.errors()
    }

    /// Sets the rule (counts and neighbourhood), re-runs, and rewrites the script's `.rule` /
    /// `.neigh` lines.
    pub fn set_rule(&mut self, rule: LifeRule) {
        self.lab.set_rule(rule, self.editor_focused);
        self.lab.finish();
    }

    /// Sets the whole spec as the controls do (re-runs; the script follows).
    pub fn set_spec(&mut self, spec: RunSpec) {
        self.lab.set_spec(spec, self.editor_focused);
        self.lab.finish();
    }

    /// Scrubs to `generation` (clamped to the run) and pauses, like the slider.
    pub fn scrub(&mut self, generation: usize) {
        self.playing = false;
        self.lab.set_generation(generation);
    }

    /// The scrubbed generation.
    pub fn generation(&self) -> usize {
        self.lab.generation()
    }

    /// Live cells at the scrubbed generation.
    pub fn alive(&self) -> usize {
        self.lab.alive()
    }

    /// The analysis of the run (LAB-3).
    pub fn report(&self) -> Report {
        self.lab.report().clone()
    }

    /// The scrubbed generation as RLE with `rule =` (full, untruncated).
    pub fn rle(&self) -> String {
        self.lab.rle()
    }

    /// The population of every generation of the run.
    pub fn populations(&self) -> &[usize] {
        self.lab.run().populations()
    }

    /// The model (read-only).
    pub fn lab(&self) -> &Lab {
        &self.lab
    }

    /// One line for the MCP `describe` tool and screen readers.
    fn summary(&self) -> String {
        let spec = self.lab.spec();
        format!(
            "Rule lab: {}, {}, {}; {}; {}",
            spec.rule.code(),
            spec.rule.neighbourhood().name(),
            spec.boundary.description(),
            self.well_description(),
            self.lab.report().verdict
        )
    }

    fn well_description(&self) -> String {
        let run = self.lab.run();
        ap_bench::a11y::top_down(
            run.spec().w,
            run.spec().h,
            self.lab.generation(),
            self.lab.alive(),
        )
    }
}

// ---- Screen ---------------------------------------------------------------------------------

impl Screen for State {
    fn enter(&mut self, _ctx: &mut Ctx) {
        self.export_open = false;
        self.ticker.reset();
    }

    fn header_actions(&mut self, ctx: &mut Ctx, ui: &mut Ui) {
        if ghost("Open in Playground").show(ui).clicked() {
            self.open_in_playground(ctx);
        }
        let export = ghost("Export").show(ui);
        if export.clicked() {
            self.export_open = !self.export_open;
            self.export_focus = self.export_open && export.has_focus();
        }
        menu_button_state(ui, &export, self.export_open);
        // the visual 32 px button inside its 44 px target
        self.export_anchor = Some(Rect::from_center_size(
            export.rect.center(),
            vec2(export.rect.width(), t::size::BUTTON),
        ));
        if self.export_open
            && let Some(anchor) = self.export_anchor
        {
            let out = Menu::new(
                Id::unique("rule-lab-export"),
                "Export",
                anchor,
                &EXPORT_ITEMS,
            )
            .focus_first(std::mem::take(&mut self.export_focus))
            .show(ui.ctx());
            if let Some(i) = out.clicked {
                self.export(i, ctx);
            }
            if out.close {
                self.export_open = false;
            }
        }
    }

    fn command(&mut self, cmd: Command, _ctx: &mut Ctx) -> bool {
        match cmd {
            Command::RunPause => self.toggle_play(),
            Command::Step => {
                let next = (self.lab.generation() + 1) % (self.lab.gens() + 1);
                self.scrub(next);
            }
        }
        true
    }

    fn receive(&mut self, carry: Carry, ctx: &mut Ctx) {
        match LifeRule::parse(&carry.rule) {
            Ok(rule) => {
                self.set_rule(rule);
                ctx.toast(format!(
                    "Analysing {} from {}",
                    rule.code(),
                    carry.from.title()
                ));
            }
            Err(e) => ctx.toast(format!("Couldn't read the rule {}: {e}", carry.rule)),
        }
        self.received = Some(carry);
    }

    fn ui(&mut self, ctx: &mut Ctx, ui: &mut Ui) {
        if self.lab.poll() || self.lab.is_running() {
            ctx.animate();
        }
        if self.playing {
            for _ in 0..self.ticker.steps(ctx.dt(), 1000.0 / PLAY_MS) {
                self.lab.advance();
            }
            ctx.animate();
        }
        layout::columns(ui, ctx, &COLUMNS, |ui, _ctx, i| match i {
            0 => self.spec_aside(ui),
            1 => self.run_column(ui),
            _ => self.script_aside(ui),
        });
    }

    fn describe(&self) -> String {
        self.summary()
    }
}

// ---- actions --------------------------------------------------------------------------------

impl State {
    fn toggle_play(&mut self) {
        self.playing = !self.playing;
        self.ticker.reset();
    }

    /// A spec control changed the spec.
    fn change_spec(&mut self, f: impl FnOnce(&mut RunSpec)) {
        let mut spec = self.lab.spec();
        f(&mut spec);
        self.lab.set_spec(spec, self.editor_focused);
    }

    /// NAV-4: the Tray gets the rule and the scrubbed generation's cells.
    fn open_in_playground(&mut self, ctx: &mut Ctx) {
        let code = self.lab.spec().rule.code();
        ctx.go_with(
            Route::Tray,
            Carry::rule(code, Route::RuleLab).with_rle(self.lab.rle()),
        );
    }

    fn export(&mut self, item: usize, ctx: &mut Ctx) {
        let stem = self.lab.file_stem();
        let generation = self.lab.generation();
        match item {
            0 => {
                ctx.copy(self.lab.rle());
                ctx.toast(format!("Copied generation {generation} as RLE"));
            }
            1 => {
                ctx.copy(self.lab.script_for_export());
                ctx.toast(self.script_note("Copied the rule script"));
            }
            2 => self.save(ctx, &format!("{stem}.rle"), &(self.lab.rle() + "\n")),
            _ => {
                let text = self.lab.script_for_export() + "\n";
                self.save(ctx, &format!("{stem}.rule"), &text);
            }
        }
    }

    /// The toast after exporting the script, noting when the text had errors.
    fn script_note(&self, done: &str) -> String {
        if self.lab.errors().is_empty() {
            done.to_string()
        } else {
            format!("{done} (the last valid one: the editor has errors)")
        }
    }

    fn save(&mut self, ctx: &mut Ctx, name: &str, contents: &str) {
        match ctx.save_text(name, contents) {
            Ok(Some(path)) => {
                let file = path
                    .file_name()
                    .map_or_else(|| name.to_string(), |f| f.to_string_lossy().into_owned());
                let done = format!("Saved {file}");
                let msg = if name.ends_with(".rule") {
                    self.script_note(&done)
                } else {
                    done
                };
                ctx.toast(msg);
            }
            Ok(None) => {}
            Err(e) => {
                log::warn!("save {name}: {e:#}");
                ctx.toast(format!("Couldn't save {name}: {e}"));
            }
        }
    }
}

// ---- left: rule spec (LAB-1) ----------------------------------------------------------------

impl State {
    fn spec_aside(&mut self, ui: &mut Ui) {
        lbl(ui, "Rule spec");
        self.family(ui);
        group(ui, "Neighbourhood", |ui| {
            Card::panel().show(ui, |ui| self.neighbourhood(ui));
        });
        group(ui, "Birth and survival", |ui| {
            Card::panel().show(ui, |ui| self.bits(ui));
        });
        group(ui, "Boundary and seed", |ui| {
            Card::panel().show(ui, |ui| self.boundary_and_seed(ui));
        });
    }

    fn family(&mut self, ui: &mut Ui) {
        ui.scope(|ui| {
            ui.spacing_mut().item_spacing.y = 6.0;
            caption(ui, "Family");
            let out = SegGroup::new("Rule family", &FAMILIES, Some(0)).show(ui);
            // Generations, Larger than Life and Lenia are disabled and say why (DONE-3)
            for resp in out.items.iter().skip(1) {
                ui.ctx().accesskit_node_builder(resp.id, |b| {
                    b.set_description(NOT_IN_THIS_VERSION);
                });
                resp.clone().on_hover_text(NOT_IN_THIS_VERSION);
            }
        });
    }

    fn neighbourhood(&mut self, ui: &mut Ui) {
        let rule = self.lab.spec().rule;
        let nb = rule.neighbourhood();
        caption_row(
            ui,
            "Neighbourhood",
            &format!("{} cells", nb.len()),
            lab_theme::font::mono(11.0),
        );
        let mut picked = None;
        let mut toggled = None;
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing = Vec2::ZERO;
            toggled = NbGrid::new(nb.bits()).show(ui);
            ui.add_space(14.0);
            let width = preset_width(ui);
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing = vec2(0.0, 4.0);
                for p in NeighbourhoodPreset::ALL {
                    let on = nb.preset() == Some(p);
                    if MiniButton::new(p.name(), on)
                        .width(width)
                        .show(ui)
                        .clicked()
                    {
                        picked = Some(p);
                    }
                }
            });
        });
        if let Some((dx, dy)) = toggled {
            self.change_spec(|s| {
                s.rule.toggle_neighbour(dx, dy);
            });
        }
        if let Some(p) = picked {
            self.change_spec(|s| s.rule.set_neighbourhood(p.neighbourhood()));
        }
    }

    fn bits(&mut self, ui: &mut Ui) {
        let rule = self.lab.spec().rule;
        let n = rule.neighbourhood().len();
        caption(ui, "Born with");
        if let Some(b) = BitRow::new("Born with", n + 1, rule.birth().bits())
            .lab()
            .show(ui)
        {
            self.change_spec(|s| {
                s.rule.toggle_birth(b);
            });
        }
        gap(ui);
        caption(ui, "Survives with");
        if let Some(b) = BitRow::new("Survives with", n + 1, rule.survive().bits())
            .lab()
            .show(ui)
        {
            self.change_spec(|s| {
                s.rule.toggle_survive(b);
            });
        }
    }

    fn boundary_and_seed(&mut self, ui: &mut Ui) {
        let spec = self.lab.spec();
        caption(ui, &format!("Boundary · {} × {}", spec.w, spec.h));
        let b = Boundary::ALL.iter().position(|b| *b == spec.boundary);
        if let Some(i) = SegGroup::new("Boundary", &BOUNDARIES, b)
            .style(SegStyle::Mini { fill: true })
            .show(ui)
            .clicked
        {
            self.change_spec(|s| s.boundary = Boundary::ALL[i]);
        }
        gap(ui);
        caption(ui, "Seed");
        let current = match spec.seed {
            Seed::RPentomino => 0,
            Seed::Acorn => 1,
            Seed::Soup { .. } => 2,
        };
        if let Some(i) = SegGroup::new("Seed", &SEEDS, Some(current))
            .style(SegStyle::Mini { fill: true })
            .show(ui)
            .clicked
        {
            self.change_spec(|s| s.seed = model::seed_choice(s.seed, i));
        }
    }
}

// ---- centre: the run (LAB-2) ----------------------------------------------------------------

impl State {
    fn run_column(&mut self, ui: &mut Ui) {
        self.run_header(ui);
        self.well(ui);
        group(ui, "Playback and population", |ui| {
            Card::panel().gap(8.0).show(ui, |ui| self.playback(ui));
        });
    }

    /// `B3/S23  Moore r1 · wraps at edges  ……  gen 120 / 400 · 148 alive`.
    fn run_header(&mut self, ui: &mut Ui) {
        let spec = self.lab.spec();
        let run_spec = *self.lab.run().spec();
        let code = spec.rule.code();
        let what = format!(
            "{} · {}",
            spec.rule.neighbourhood().name(),
            spec.boundary.description()
        );
        let gens = self.lab.gens();
        let digits = gens.to_string().len().max(3);
        let mut now = format!(
            "gen {:0digits$} / {gens} · {} alive",
            self.lab.generation(),
            self.lab.alive()
        );
        if self.lab.is_running() || run_spec != spec.clamped() {
            now.push_str(" · running…");
        }
        let parts: [(String, FontId, Color32); 3] = [
            (code, lab_theme::font::mono_medium(22.0), t::INK),
            (what, lab_theme::font::body(12.0), t::TEXT_MUTED),
            (now, lab_theme::font::mono(12.0), t::TEXT_DIM),
        ];
        let galleys: Vec<Arc<Galley>> = parts
            .iter()
            .map(|(s, f, c)| galley(ui, s, f.clone(), *c))
            .collect();
        let items = [
            FlowItem::fixed(galleys[0].size()),
            FlowItem::fixed(galleys[1].size()),
            FlowItem::fixed(galleys[2].size()).auto(),
        ];
        let width = ui.available_width();
        let lay = flow(&items, width, vec2(12.0, 6.0), 0.0);
        let (rect, _) = ui.allocate_exact_size(vec2(width, lay.height), Sense::hover());
        for (i, ((s, _, c), g)) in parts.iter().zip(galleys).enumerate() {
            let r = lay.rects[i].translate(rect.min.to_vec2());
            ui.painter()
                .galley(pos2(r.left(), text_top(r.center().y, &g)), g, *c);
            ui.interact(r, ui.scope_id().with(("run-header", i)), Sense::hover())
                .widget_info(|| WidgetInfo::labeled(Role::Label, true, s));
        }
    }

    /// The top-down grid at the scrubbed generation, on the `.well`.
    fn well(&mut self, ui: &mut Ui) {
        let run = self.lab.run();
        let generation = self.lab.generation();
        let cell = |x: usize, y: usize| match run.cell(generation, x, y) {
            CellChange::Empty => TopCell::Empty,
            CellChange::Born => TopCell::Born,
            CellChange::Alive => TopCell::Alive,
            CellChange::Ghost => TopCell::Ghost,
        };
        let grid = TopDownGrid::new(run.spec().w, run.spec().h, &cell);
        let width = ui.available_width();
        let min = ui.cursor().min;
        let geo = grid.paint(ui.painter(), min, width);
        let resp = ui.allocate_rect(geo.well, Sense::hover());
        let label = grid.description(generation, self.lab.alive());
        resp.widget_info(|| WidgetInfo::labeled(Role::Image, true, &label));
    }

    /// Play / Pause · SCRUB · slider; the population plot; the generation axis.
    fn playback(&mut self, ui: &mut Ui) {
        let gens = self.lab.gens();
        // `display: flex; align-items: center; gap: 10px` around the 30 px Play button
        let row = vec2(ui.available_width(), 30.0);
        ui.allocate_ui_with_layout(row, Layout::left_to_right(Align::Center), |ui| {
            ui.set_height(30.0);
            ui.spacing_mut().item_spacing = vec2(10.0, 0.0);
            let play = if self.playing {
                prim("Pause").icon(Icon::Pause)
            } else {
                prim("Play").icon(Icon::Play)
            };
            if play.height(30.0).show(ui).clicked() {
                self.toggle_play();
            }
            lbl(ui, "Scrub");
            let mut g = self.lab.generation() as f32;
            let resp = Slider::new("Scrub", &mut g, 0.0..=gens as f32)
                .step(1.0)
                .show(ui);
            if resp.changed() {
                self.scrub(g.round().max(0.0) as usize);
            }
        });
        let pops: Vec<f32> = self
            .lab
            .run()
            .populations()
            .iter()
            .map(|&p| p as f32)
            .collect();
        let report = self.lab.report();
        let label = format!("Population across {gens} generations");
        // the svg's 120 px plus its 1 px border
        let resp = Plot::series(&pops)
            .height(122.0)
            .stable_from(report.first_repeat.map(|r| r.start))
            .playhead(Some(self.lab.generation()))
            .label(&label)
            .show(ui);
        let mut description = format!(
            "Peak {} at generation {}, final {}",
            report.peak, report.peak_gen, report.final_pop
        );
        if let Some(r) = report.first_repeat {
            description.push_str(&format!(", stable from generation {}", r.start));
        }
        description.push_str(&format!(
            "; playhead at generation {}",
            self.lab.generation()
        ));
        ui.ctx()
            .accesskit_node_builder(resp.id, |b| b.set_description(description));
        axis(ui, gens);
    }
}

/// `0 · 100 · 200 · 300 · 400 gens`, spread like `justify-content: space-between`.
fn axis(ui: &mut Ui, gens: usize) {
    let font = lab_theme::font::mono(11.0);
    let labels: Vec<String> = (0..=4)
        .map(|k| {
            let v = (gens * k + 2) / 4;
            if k == 4 {
                format!("{v} gens")
            } else {
                v.to_string()
            }
        })
        .collect();
    let galleys: Vec<_> = labels
        .iter()
        .map(|s| galley(ui, s, font.clone(), t::TEXT_FAINT))
        .collect();
    let width = ui.available_width();
    let h = galleys.iter().map(|g| g.size().y).fold(0.0, f32::max);
    let (rect, resp) = ui.allocate_exact_size(vec2(width, h), Sense::hover());
    let used: f32 = galleys.iter().map(|g| g.size().x).sum();
    let gap = ((width - used) / 4.0).max(0.0);
    let mut x = rect.left();
    for g in galleys {
        let w = g.size().x;
        ui.painter().galley(pos2(x, rect.top()), g, t::TEXT_FAINT);
        x += w + gap;
    }
    let text = labels.join(", ");
    resp.widget_info(|| WidgetInfo::labeled(Role::Label, true, format!("Generations {text}")));
}

// ---- right: script and analysis (LAB-3/4/5) -------------------------------------------------

impl State {
    fn script_aside(&mut self, ui: &mut Ui) {
        lbl(ui, "Rule script");
        let out = editor::script_editor(ui, self.lab.text(), self.lab.errors());
        if let Some(text) = out.changed {
            self.lab.edit_text(text);
        }
        if self.editor_focused && !out.focused {
            self.lab.flush_deferred();
        }
        self.editor_focused = out.focused;

        lbl(ui, "Analysis");
        let report = self.lab.report().clone();
        let tiles = [
            ("Peak population", report.peak_text(), t::INK),
            ("Final population", report.final_pop.to_string(), t::TEXT),
            ("First repeat", report.first_repeat_text(), t::LINK),
            ("Period", report.period_text(), t::LINK),
        ];
        ui.scope(|ui| {
            ui.spacing_mut().item_spacing = vec2(8.0, 8.0);
            for pair in tiles.chunks(2) {
                ui.columns(2, |cols| {
                    for (col, (k, v, c)) in cols.iter_mut().zip(pair) {
                        StatTile::new(k, v).color(*c).compact().show(col);
                    }
                });
            }
        });
        verdict(ui, &report.verdict);

        lbl_row(
            ui,
            "Pattern export · RLE",
            &format!("gen {:03}", self.lab.generation()),
            lab_theme::font::body(11.0),
            t::TEXT_MUTED,
        );
        let preview = self.lab.rle_preview();
        let name = format!("RLE of generation {}: {preview}", self.lab.generation());
        Card::panel().fill(t::INSET).show(ui, |ui| {
            CodeBlock::new(&preview)
                .wrap()
                .color(t::TEXT_MUTED)
                .label(&name)
                .show(ui);
        });
    }
}

// ---- small helpers --------------------------------------------------------------------------

/// An AccessKit group around a card (`<section aria-label>`), keeping the column's gap after
/// it.
fn group(ui: &mut Ui, label: &str, add: impl FnOnce(&mut Ui)) {
    let builder = UiBuilder::new()
        .accessibility_role(Role::Group)
        .accessibility_label(label);
    ui.scope_builder(builder, |ui| {
        add(ui);
        crate::widgets::set_bounds(ui, ui.min_rect());
    });
}

/// The parent's gap after a packed widget (bit rows and segmented wells advance without it).
fn gap(ui: &mut Ui) {
    ui.add_space(ui.spacing().item_spacing.y);
}

/// A 12 px `#D9D9D9` caption ("Family", "Born with", …).
fn caption(ui: &mut Ui, text: &str) {
    crate::widgets::text::line(ui, text, lab_theme::font::body(12.0), t::TEXT_DIM);
}

/// "Neighbourhood ……… 8 cells": a caption with a right-aligned note on the same baseline.
fn caption_row(ui: &mut Ui, left: &str, right: &str, right_font: FontId) {
    let width = ui.available_width();
    let lg = galley(ui, left, lab_theme::font::body(12.0), t::TEXT_DIM);
    let rg = galley(ui, right, right_font, t::TEXT_MUTED);
    let base = |g: &Galley| {
        g.rows
            .first()
            .and_then(|r| r.row.glyphs.first())
            .map_or(g.size().y, |gl| gl.pos.y)
    };
    let baseline = base(&lg).max(base(&rg));
    let h = (baseline - base(&lg) + lg.size().y).max(baseline - base(&rg) + rg.size().y);
    let (rect, _) = ui.allocate_exact_size(vec2(width, h), Sense::hover());
    let painter = ui.painter();
    let ly = rect.top() + baseline - base(&lg);
    let ry = rect.top() + baseline - base(&rg);
    let rw = rg.size().x;
    painter.galley(pos2(rect.left(), ly), lg, t::TEXT_DIM);
    painter.galley(pos2(rect.right() - rw, ry), rg, t::TEXT_MUTED);
    for (i, (s, r)) in [
        (left, Rect::from_min_size(rect.min, vec2(width - rw, h))),
        (
            right,
            Rect::from_min_size(pos2(rect.right() - rw, rect.top()), vec2(rw, h)),
        ),
    ]
    .into_iter()
    .enumerate()
    {
        ui.interact(
            r,
            ui.scope_id().with(("caption-row", left, i)),
            Sense::hover(),
        )
        .widget_info(|| WidgetInfo::labeled(Role::Label, true, s));
    }
}

/// The presets column's width: the longest single word plus `.mini`'s padding (the column's
/// min-content in the mockup, so "von Neumann" wraps and "Hexagonal" doesn't).
fn preset_width(ui: &Ui) -> f32 {
    let font = lab_theme::font::medium(12.0);
    NeighbourhoodPreset::ALL
        .iter()
        .flat_map(|p| p.name().split_whitespace())
        .map(|w| {
            wrapped(ui, w, font.clone(), t::TEXT, f32::INFINITY, None)
                .size()
                .x
        })
        .fold(0.0, f32::max)
        .ceil()
        + 20.0
}
