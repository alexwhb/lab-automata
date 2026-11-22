//! Tray · 2D (SCR-1, TRAY-1…10) — mockup `docs/design/mockup/Tray.dc.html`.
//!
//! A 40 × 26 torus of Life-like cells ([`model::Model`] on `ap-engine`) drawn as the tilted
//! Petri tray of `ap-bench` ([`ap_bench::TrayScene`]), with:
//!
//! - the rules aside: six cartridges (the one matching the rule is lit) and eight stamps (plus
//!   a pasted / dropped pattern armed as a custom stamp, TRAY-9);
//! - the bench: the tab row's Draw / Erase / Stamp tools, Random soup and Clear; the stage
//!   with the tray (hover, drag-painting through the perspective, the keyboard cursor), the
//!   `gen · alive` and tool-hint chips and the cell-age legend;
//! - the readouts: rule code and name, Born-with / Survives-with bits, speed, Ghost trails /
//!   Top-down view, the population sparkline, the four tiles and the "what is happening"
//!   callout;
//! - the header's Run / Pause, Step, Restart, `gen 0000` and Share (RLE to the clipboard).
//!
//! The [control API](State#control-api) (`rule`, `set_rule`, `load_pattern`, `step_n`, `rle`,
//! `stats`, …) is what the MCP bridge drives; see `docs/ARCHITECTURE.md` › "Screen control
//! API".

mod model;
mod panels;
mod stage;

pub use model::{Model, TrayStats, fit_to_tray, line_cells};
pub use stage::DROP_HINT;

use super::Screen;
use crate::ctx::{Carry, Command, Ctx, Ticker};
use crate::layout::{self, Column};
use crate::widgets::tabs::RowSlot;
use crate::widgets::{Icon, SegGroup, SegItem, SegStyle, counter, ghost, prim};
use ap_bench::anim::{Transition, ease};
use ap_bench::{CellVisual, TrayGeometry};
use ap_engine::pattern::{self, library};
use ap_engine::presets::{TRAY_H, TRAY_W};
use ap_engine::{Grid, LifeRule, Pattern, PatternFile};
use eframe::egui::{self, Pos2, Rect, Ui};

/// The page row: `aside` 248 (padding 14 12, gap 14) · `main` · readouts (gap 18).
pub const COLUMNS: [Column; 3] = [
    Column::aside("Cartridges and stamps", 248.0)
        .pad(14.0, 12.0)
        .border_right(),
    Column::main("Bench"),
    Column::readout("Rule and readouts").gap(18.0),
];

/// The drawing tools, in the tab row's order.
pub const TOOLS: [SegItem<'static>; 3] = [
    SegItem::new("Draw"),
    SegItem::new("Erase"),
    SegItem::new("Stamp"),
];

/// A drawing tool (TRAY-3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    /// Press toggles a cell, dragging paints that value.
    Draw,
    /// Dragging clears cells.
    Erase,
    /// A click places the selected stamp centred on the cell (wrapping).
    Stamp,
}

impl Tool {
    /// In [`TOOLS`] order.
    pub const ALL: [Tool; 3] = [Tool::Draw, Tool::Erase, Tool::Stamp];

    /// The index into [`TOOLS`].
    pub fn index(self) -> usize {
        self as usize
    }
}

/// Speed range in generations per second (TRAY-7).
pub const SPEED_RANGE: std::ops::RangeInclusive<f32> = 1.0..=30.0;
/// The default speed.
pub const DEFAULT_SPEED: f32 = 10.0;
/// The tray tilt's transition (the mockup's `transition: transform .4s ease`).
pub const TILT_SECS: f32 = 0.4;
/// Pasted text and dropped / opened files larger than this are refused.
const MAX_PATTERN_BYTES: u64 = 4 << 20;
/// Patterns with more live cells than this aren't armed as a stamp: ten trays' worth (the tray
/// holds 1 040), and the preview paints every cell each frame.
pub const MAX_STAMP_CELLS: usize = 10_000;

/// Which stamp the Stamp tool places.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StampChoice {
    /// One of the eight built-in stamps (index into [`library::STAMPS`]).
    Library(usize),
    /// The pasted / dropped pattern (TRAY-9).
    Custom,
}

/// A pattern armed from the clipboard or a dropped file (TRAY-9).
#[derive(Debug, Clone, PartialEq)]
pub struct CustomStamp {
    /// Its name (the file's `#N` / `!Name:`, else the file name, else "Pasted pattern").
    pub name: String,
    /// The cells.
    pub pattern: Pattern,
    cells: Vec<(i32, i32)>,
}

impl CustomStamp {
    fn new(name: String, pattern: Pattern) -> Self {
        let pattern = pattern.trimmed();
        let cells = preview_cells(&pattern);
        Self {
            name,
            pattern,
            cells,
        }
    }
}

/// A drag in progress on the tray.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Stroke {
    /// No button down.
    #[default]
    Idle,
    /// The press didn't start on a cell (or the tool is Stamp): ignore the rest of the drag.
    Inert,
    /// Painting `value`; `last` is the cell painted last (interpolate from it).
    Paint { value: bool, last: (usize, usize) },
}

/// The Tray's state. The app keeps it across navigation.
///
/// `running`, `generation`, `tool` and `received` are public for the app shell, the smoke
/// run and tests; `generation` mirrors the engine (change it through the methods).
///
/// # Control API
///
/// The MCP bridge and tests drive the Tray with [`State::rule`], [`State::set_rule`],
/// [`State::load_pattern`], [`State::set_running`], [`State::running`], [`State::step_n`],
/// [`State::clear`], [`State::restart`], [`State::randomize`], [`State::rle`],
/// [`State::stats`] and [`State::grid`].
#[derive(Debug)]
pub struct State {
    /// Run / Pause.
    pub running: bool,
    /// Generations since the last reset (a mirror of the engine's counter).
    pub generation: u64,
    /// Selected tool (index into [`TOOLS`]).
    pub tool: usize,
    /// The last rule / pattern carried here (NAV-4).
    pub received: Option<Carry>,
    model: Model,
    speed: f32,
    trails: bool,
    flat: bool,
    tilt: Transition,
    stamp: StampChoice,
    custom: Option<CustomStamp>,
    stamp_cells: Vec<Vec<(i32, i32)>>,
    cursor: (usize, usize),
    stroke: Stroke,
    soups: u64,
    ticker: Ticker,
    started: bool,
    geometry: Option<TrayGeometry>,
    has_focus: bool,
}

impl Default for State {
    fn default() -> Self {
        Self {
            running: false,
            generation: 0,
            tool: Tool::Draw.index(),
            received: None,
            model: Model::default(),
            speed: DEFAULT_SPEED,
            trails: true,
            flat: false,
            tilt: Transition::new(1.0, TILT_SECS, ease),
            stamp: StampChoice::Library(0),
            custom: None,
            stamp_cells: library::STAMPS
                .iter()
                .map(|s| preview_cells(&s.pattern()))
                .collect(),
            cursor: (TRAY_W / 2, TRAY_H / 2),
            stroke: Stroke::Idle,
            soups: 0,
            ticker: Ticker::default(),
            started: false,
            geometry: None,
            has_focus: false,
        }
    }
}

/// A pattern's live cells as the preview's `(x, y)` list.
fn preview_cells(p: &Pattern) -> Vec<(i32, i32)> {
    p.cells()
        .iter()
        .map(|&(x, y)| (x as i32, y as i32))
        .collect()
}

/// The bench's look of cell `(x, y)` (TRAY-2): live cells by age, dead cells with a ghost
/// trail when trails are on.
pub fn cell_visual(grid: &Grid, trails: bool, x: usize, y: usize) -> CellVisual {
    match grid.age(x, y) {
        0 => match grid.trail(x, y) {
            t if trails && t > 0 => CellVisual::Trail(t),
            _ => CellVisual::Empty,
        },
        a => CellVisual::Live(a),
    }
}

// ---- control API ---------------------------------------------------------------------------

impl State {
    /// The current rule.
    pub fn rule(&self) -> &LifeRule {
        self.model.rule()
    }

    /// Sets the rule (cartridges, bits, NAV-4, MCP); the cells stay.
    pub fn set_rule(&mut self, rule: LifeRule) {
        self.model.set_rule(rule);
    }

    /// Clears the tray and places `pattern` centred on it (too-large patterns are cropped to
    /// the tray around their centre); the counters start over. Returns whether it was cropped.
    pub fn load_pattern(&mut self, pattern: &Pattern) -> bool {
        let cropped = self.model.load_pattern(pattern);
        self.sync();
        cropped
    }

    /// Run (`true`) or pause.
    pub fn set_running(&mut self, running: bool) {
        if !running {
            self.ticker.reset();
        }
        self.running = running;
    }

    /// Whether it is running.
    pub fn running(&self) -> bool {
        self.running
    }

    /// Advances `n` generations now (whether running or not).
    pub fn step_n(&mut self, n: u32) {
        for _ in 0..n {
            self.model.step();
        }
        self.sync();
    }

    /// Kills every cell (Clear); the rule stays.
    pub fn clear(&mut self) {
        self.model.clear();
        self.sync();
    }

    /// The starting garden under Life (Restart).
    pub fn restart(&mut self) {
        self.model.restart();
        self.sync();
    }

    /// A 30 % random soup from `seed` (Random soup); the rule stays.
    pub fn randomize(&mut self, seed: u64) {
        self.model.randomize(seed);
        self.sync();
    }

    /// The whole tray as RLE with `rule =` (what Share copies).
    pub fn rle(&self) -> String {
        self.model.rle()
    }

    /// Generation, population, born, died and the mood.
    pub fn stats(&self) -> TrayStats {
        self.model.stats()
    }

    /// The grid (ages and trails).
    pub fn grid(&self) -> &Grid {
        self.model.grid()
    }

    /// The population history behind the sparkline (oldest first, at most 120).
    pub fn history(&self) -> &[usize] {
        self.model.history().values()
    }

    /// The selected tool.
    pub fn current_tool(&self) -> Tool {
        Tool::ALL.get(self.tool).copied().unwrap_or(Tool::Draw)
    }

    /// Selects a tool.
    pub fn set_tool(&mut self, tool: Tool) {
        self.tool = tool.index();
    }

    /// Speed in generations per second.
    pub fn speed(&self) -> f32 {
        self.speed
    }

    /// Sets the speed (clamped to 1–30 gen/s; NaN and infinities are ignored, as they would
    /// stall the ticker).
    pub fn set_speed(&mut self, speed: f32) {
        if speed.is_finite() {
            self.speed = speed.clamp(*SPEED_RANGE.start(), *SPEED_RANGE.end());
        }
    }

    /// Ghost trails on?
    pub fn trails(&self) -> bool {
        self.trails
    }

    /// Top-down view on (the tilt animates towards flat)?
    pub fn flat(&self) -> bool {
        self.flat
    }

    /// The tray's tilt now: 1 = tilted 44°, 0 = flat (animates over 0.4 s).
    pub fn tilt(&self) -> f32 {
        self.tilt.value()
    }

    /// The selected stamp.
    pub fn stamp(&self) -> StampChoice {
        self.stamp
    }

    /// The pattern armed from the clipboard or a dropped file, if any (TRAY-9).
    pub fn custom_stamp(&self) -> Option<&CustomStamp> {
        self.custom.as_ref()
    }

    /// The keyboard cursor (TRAY-10).
    pub fn cursor(&self) -> (usize, usize) {
        self.cursor
    }

    /// How cell `(x, y)` is drawn right now.
    pub fn cell_visual(&self, x: usize, y: usize) -> CellVisual {
        cell_visual(self.model.grid(), self.trails, x, y)
    }

    /// The screen point at the centre of cell `(x, y)`'s visible face (lifted for live cells)
    /// as of the last frame drawn, projected through the tray's perspective — where to click
    /// that cell. `None` before the first frame or for cells off the grid.
    pub fn cell_center(&self, x: usize, y: usize) -> Option<Pos2> {
        let geo = self.geometry.as_ref()?;
        (x < TRAY_W && y < TRAY_H).then(|| {
            let face = geo.cell_face_local(x, y);
            geo.projection().project(face.center())
        })
    }

    /// The screen bounding box of cell `(x, y)`'s visible face as of the last frame.
    pub fn cell_rect(&self, x: usize, y: usize) -> Option<Rect> {
        let geo = self.geometry.as_ref()?;
        (x < TRAY_W && y < TRAY_H).then(|| geo.cell_rect(x, y))
    }

    /// The cell whose visible face is under `screen` as of the last frame.
    pub fn cell_at(&self, screen: Pos2) -> Option<(usize, usize)> {
        self.geometry.as_ref()?.cell_at(screen)
    }

    /// The tool hint chip's text (TRAY-3).
    pub fn tool_hint(&self) -> String {
        match self.current_tool() {
            Tool::Draw => "Click or drag to draw".into(),
            Tool::Erase => "Drag to erase".into(),
            Tool::Stamp => format!("Click to place {}", self.stamp_name()),
        }
    }

    /// The selected stamp's name.
    pub fn stamp_name(&self) -> &str {
        match (self.stamp, &self.custom) {
            (StampChoice::Custom, Some(c)) => &c.name,
            (StampChoice::Library(i), _) => library::STAMPS.get(i).map_or("", |s| s.name),
            (StampChoice::Custom, None) => library::STAMPS[0].name,
        }
    }

    /// The pattern the Stamp tool places.
    fn stamp_pattern(&self) -> Pattern {
        match (self.stamp, &self.custom) {
            (StampChoice::Custom, Some(c)) => c.pattern.clone(),
            (StampChoice::Library(i), _) => library::STAMPS
                .get(i)
                .map(|s| s.pattern())
                .unwrap_or_default(),
            (StampChoice::Custom, None) => library::STAMPS[0].pattern(),
        }
    }

    /// Arms `file` as the custom stamp and selects it (TRAY-9).
    pub fn arm(&mut self, name: impl Into<String>, pattern: &Pattern) {
        self.custom = Some(CustomStamp::new(name.into(), pattern.clone()));
        self.stamp = StampChoice::Custom;
        self.set_tool(Tool::Stamp);
    }

    /// Picks one of the built-in stamps (selects the Stamp tool).
    pub fn pick_stamp(&mut self, index: usize) {
        if index < library::STAMPS.len() {
            self.stamp = StampChoice::Library(index);
            self.set_tool(Tool::Stamp);
        }
    }

    /// The accessible description of the tray (A11Y-4).
    pub fn description(&self) -> String {
        ap_bench::a11y::tray(
            TRAY_W,
            TRAY_H,
            self.model.generation(),
            self.model.grid().population(),
        )
    }

    fn sync(&mut self) {
        self.generation = self.model.generation();
    }

    /// Step (pauses, then advances one generation).
    fn step_once(&mut self) {
        self.set_running(false);
        self.step_n(1);
    }

    fn toggle_run(&mut self) {
        let r = !self.running;
        self.set_running(r);
    }

    fn set_flat(&mut self, flat: bool) {
        self.flat = flat;
        self.tilt.set_target(if flat { 0.0 } else { 1.0 });
    }

    fn share(&mut self, ctx: &mut Ctx) {
        ctx.copy(self.rle());
        ctx.toast("Copied the tray as RLE");
    }

    fn random_soup(&mut self, ctx: &Ctx) {
        self.soups += 1;
        let seed = ctx.rng_seed() ^ self.soups.wrapping_mul(0x9E37_79B9_7F4A_7C15);
        self.randomize(seed);
    }

    /// Applies the current tool at cell `(x, y)` once (keyboard / a click with Stamp).
    fn apply_tool(&mut self, x: usize, y: usize) {
        match self.current_tool() {
            Tool::Draw => {
                let alive = !self.model.grid().is_alive(x, y);
                self.model.set_cell(x, y, alive);
            }
            Tool::Erase => {
                self.model.set_cell(x, y, false);
            }
            Tool::Stamp => {
                let p = self.stamp_pattern();
                self.model.stamp(&p, x, y);
            }
        }
    }

    /// Reads ⌘V pastes and dropped `.rle` / `.cells` files (TRAY-9).
    fn receive_patterns(&mut self, ctx: &mut Ctx) {
        if ctx.text_focus() {
            return;
        }
        let (pastes, dropped) = ctx.egui().input(|i| {
            let pastes: Vec<String> = i
                .events
                .iter()
                .filter_map(|e| match e {
                    egui::Event::Paste(s) => Some(s.clone()),
                    _ => None,
                })
                .collect();
            (pastes, i.raw.dropped_files.clone())
        });
        for text in pastes {
            if text.len() as u64 > MAX_PATTERN_BYTES {
                ctx.toast("The pasted text is too large for a pattern (over 4 MB)");
                continue;
            }
            match pattern::parse_any(&text) {
                Ok(file) if !file.pattern.is_empty() => {
                    let note = self
                        .arm_parsed(file, "Pasted pattern")
                        .unwrap_or_else(|why| why);
                    ctx.toast(note);
                }
                Ok(_) => ctx.toast("The clipboard's pattern has no live cells"),
                Err(e) => ctx.toast(format!("Couldn't read the pasted pattern: {e}")),
            }
        }
        for file in dropped {
            let path = file.path().to_path_buf();
            let text = if too_big(&path) {
                Err("the file is too large for a pattern".to_string())
            } else {
                file.bytes()
                    .map(|b| String::from_utf8_lossy(&b).into_owned())
            };
            match self.arm_text(&path, text) {
                Ok(note) | Err(note) => ctx.toast(note),
            }
        }
    }

    /// Reads a `.rle` / `.cells` file and arms it as the custom stamp (TRAY-9: a file opened
    /// from Finder, `open -a`, or named on the command line; dropped files take the same
    /// path). `Ok` says what was armed, `Err` why the file couldn't be — the toast to show.
    pub fn open_file(&mut self, path: &std::path::Path) -> Result<String, String> {
        let text = if too_big(path) {
            Err("the file is too large for a pattern".to_string())
        } else {
            std::fs::read(path)
                .map(|b| String::from_utf8_lossy(&b).into_owned())
                .map_err(|e| e.to_string())
        };
        self.arm_text(path, text)
    }

    /// Parses `text` (read from `path`, or why it couldn't be) and arms it, named by the
    /// file's `#N` / `!Name:` or else its stem. Returns the toast.
    fn arm_text(
        &mut self,
        path: &std::path::Path,
        text: Result<String, String>,
    ) -> Result<String, String> {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "dropped file".into());
        match text.and_then(|t| pattern::parse_file(&name, &t).map_err(|e| e.to_string())) {
            Ok(f) if !f.pattern.is_empty() => {
                let stem = path
                    .file_stem()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_else(|| name.clone());
                self.arm_parsed(f, &stem)
            }
            Ok(_) => Err(format!("{name} has no live cells")),
            Err(e) => Err(format!("Couldn't read {name}: {e}")),
        }
    }

    /// Arms a parsed file (named by its `#N` / `!Name:`, else `fallback`) unless it has more
    /// than [`MAX_STAMP_CELLS`] live cells; returns the toast (`Err`: why it wasn't armed).
    fn arm_parsed(&mut self, file: PatternFile, fallback: &str) -> Result<String, String> {
        let name = file
            .name
            .as_deref()
            .map(str::trim)
            .filter(|n| !n.is_empty())
            .unwrap_or(fallback)
            .to_string();
        let cells = file.pattern.population();
        if cells > MAX_STAMP_CELLS {
            return Err(format!(
                "{name} has {cells} live cells; a stamp can have at most {MAX_STAMP_CELLS} (the tray holds {})",
                TRAY_W * TRAY_H
            ));
        }
        self.arm(name.clone(), &file.pattern);
        Ok(format!(
            "Armed {name} as a stamp: click the tray to place it"
        ))
    }
}

/// Files larger than [`MAX_PATTERN_BYTES`] aren't read as patterns.
fn too_big(path: &std::path::Path) -> bool {
    std::fs::metadata(path).is_ok_and(|m| m.len() > MAX_PATTERN_BYTES)
}

impl Screen for State {
    fn enter(&mut self, ctx: &mut Ctx) {
        if !self.started {
            self.started = true;
            self.set_running(ctx.autoplay());
        }
    }

    fn header_actions(&mut self, ctx: &mut Ctx, ui: &mut Ui) {
        let run = if self.running {
            prim("Pause").icon(Icon::Pause)
        } else {
            prim("Run").icon(Icon::Play)
        };
        if run.show(ui).clicked() {
            self.toggle_run();
        }
        if ghost("Step").icon(Icon::Step).show(ui).clicked() {
            self.step_once();
        }
        if ghost("Restart").icon(Icon::Restart).show(ui).clicked() {
            self.restart();
        }
        counter(ui, &format!("gen {:04}", self.model.generation()));
        if ghost("Share").show(ui).clicked() {
            self.share(ctx);
        }
    }

    fn tab_actions(&mut self, ctx: &mut Ctx, slot: &mut RowSlot) {
        slot.add(|ui| {
            if let Some(i) = SegGroup::new("Tool", &TOOLS, Some(self.tool))
                .style(SegStyle::Tool)
                .show(ui)
                .clicked
            {
                self.tool = i;
            }
        });
        let soup = slot.add(|ui| ghost("Random soup").icon(Icon::Dice).height(30.0).show(ui));
        if soup.clicked() {
            self.random_soup(ctx);
        }
        if slot
            .add(|ui| ghost("Clear").height(30.0).show(ui))
            .clicked()
        {
            self.clear();
        }
    }

    fn receive(&mut self, carry: Carry, ctx: &mut Ctx) {
        match LifeRule::parse(&carry.rule) {
            Ok(rule) => {
                self.set_rule(rule);
                let mut note = format!("{} from {}", rule.code(), carry.from.title());
                if let Some(text) = &carry.rle {
                    match pattern::parse_any(text) {
                        Ok(file) => {
                            if self.load_pattern(&file.pattern) {
                                note.push_str(", cropped to the tray");
                            }
                        }
                        Err(e) => note.push_str(&format!(" (its pattern didn't load: {e})")),
                    }
                }
                ctx.toast(format!("Tray: {note}"));
            }
            Err(e) => ctx.toast(format!("The Tray can't run {}: {e}", carry.rule)),
        }
        self.received = Some(carry);
    }

    fn command(&mut self, cmd: Command, _ctx: &mut Ctx) -> bool {
        match cmd {
            Command::RunPause => self.toggle_run(),
            Command::Step => self.step_once(),
        }
        true
    }

    fn tick(&mut self, ctx: &mut Ctx) {
        if self.running {
            let n = self.ticker.steps(ctx.dt(), self.speed);
            self.step_n(n);
            // nothing moves between generations: the next frame when the next one is due
            ctx.wake_in(self.ticker.due_in(self.speed));
        }
    }

    fn ui(&mut self, ctx: &mut Ctx, ui: &mut Ui) {
        self.receive_patterns(ctx);
        self.tilt.step(ctx.dt());
        layout::columns(ui, ctx, &COLUMNS, |ui, ctx, i| match i {
            0 => self.rules_aside(ui),
            1 => layout::bench(ui, ctx, self, |s, ctx, stage| s.stage(ctx, stage)),
            _ => self.readouts(ui),
        });
        // the tilt eases at display rate (also right after "Top-down view" this frame)
        if self.tilt.is_animating() {
            ctx.animate();
        }
    }

    fn describe(&self) -> String {
        self.description()
    }
}
