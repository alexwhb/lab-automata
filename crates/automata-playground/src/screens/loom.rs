//! Loom · 1D (SCR-2, LOOM-1…6) — mockup `docs/design/mockup/Loom.dc.html`.
//!
//! An elementary (Wolfram) cellular automaton woven row by row ([`ap_engine::Weave`]: 121
//! cells, wrapping, the newest 72 rows kept) and drawn as fabric on the tilted board of
//! `ap-bench` ([`ap_bench::LoomScene`]), with:
//!
//! - the stage: the fabric (newest row in the newborn colour, the far edge fading), the print
//!   head with its two spools (they turn while weaving and stop when paused), its label,
//!   progress (rows / 72) and "Rule N"; the eight rule keys 111 → 000 as real buttons
//!   ([`ap_bench::paint_key`]; pressing one flips that bit of the rule) and the caption;
//! - the aside: the rule number, its 8-bit binary and blurb, −1 / +1 / Surprise me; the six
//!   famous threads; First row (Single cell / Random) and the Speed slider (2–40 rows/s);
//! - the header's Weave / Pause, New thread, `row 0000` and Share ("Rule N" + the rows as
//!   text).
//!
//! Every rule change restarts the thread (LOOM-4). With a Random first row each restart draws
//! a new row (as the mockup does), from a generator seeded by [`Ctx::rng_seed`], so tests are
//! deterministic.
//!
//! The [control API](State#control-api) is what the MCP bridge drives; see
//! `docs/ARCHITECTURE.md` › "Screen control API".

mod aside;
mod stage;

pub(super) use aside::section;
pub use stage::{CAPTION, KEYS_LABEL, key_name};

use super::Screen;
use crate::ctx::{Command, Ctx, Ticker};
use crate::layout::{self, Column};
use crate::widgets::tabs::RowSlot;
use crate::widgets::{Icon, counter, ghost, prim};
use ap_engine::{Elementary, Rng, Weave, WeaveStart};
use eframe::egui::Ui;
use std::ops::RangeInclusive;

/// The page row: `main` (tab row + stage) · the rule aside (`flex: 1 0 320px; max-width:
/// 360px`, padding 16, gap 16).
pub const COLUMNS: [Column; 2] = [
    Column::main("Loom"),
    Column::readout("Rule and presets").gap(16.0),
];

/// Weaving speed in rows per second (LOOM-6).
pub const SPEED_RANGE: RangeInclusive<f32> = 2.0..=40.0;
/// The default speed.
pub const DEFAULT_SPEED: f32 = 14.0;
/// The rule the Loom starts with (Rule 30, "Chaos").
pub const START_RULE: u8 = 30;
/// `step_n` weaves at most this many rows per call (a bad MCP argument can't hang a frame).
pub const MAX_STEP_N: u32 = 100_000;
/// A live cell in [`State::rows_text`] / Share.
pub const ALIVE_CHAR: char = '█';
/// An empty cell in [`State::rows_text`] / Share.
pub const DEAD_CHAR: char = '·';
/// Mixed into [`Ctx::rng_seed`] so the Loom's random rows don't mirror other screens' soups.
const SEED_SALT: u64 = 0x4C4F_4F4D_2D31_4400;

/// The "First row" choice (LOOM-6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FirstRow {
    /// One live cell in the middle.
    Single,
    /// Each cell alive with probability ½ (a new row on every restart).
    Random,
}

impl FirstRow {
    /// In the segmented group's order.
    pub const ALL: [FirstRow; 2] = [FirstRow::Single, FirstRow::Random];

    /// The segment label ("Single cell" / "Random").
    pub fn label(self) -> &'static str {
        match self {
            FirstRow::Single => "Single cell",
            FirstRow::Random => "Random",
        }
    }

    /// The kind of a thread's start.
    pub fn of(start: WeaveStart) -> FirstRow {
        match start {
            WeaveStart::Single => FirstRow::Single,
            WeaveStart::Random(_) => FirstRow::Random,
        }
    }
}

/// The Loom's state: the thread, the run state and speed, the spools' running time and the
/// generator for Surprise me / random first rows.
#[derive(Debug, Clone)]
pub struct State {
    weave: Weave,
    running: bool,
    speed: f32,
    ticker: Ticker,
    /// Seconds spent weaving (turns the spools; stands still while paused).
    spool_secs: f64,
    rng: Rng,
    started: bool,
}

impl Default for State {
    fn default() -> Self {
        Self {
            weave: Weave::new(START_RULE, WeaveStart::Single),
            running: false,
            speed: DEFAULT_SPEED,
            ticker: Ticker::default(),
            spool_secs: 0.0,
            rng: Rng::new(SEED_SALT),
            started: false,
        }
    }
}

/// # Control API
///
/// What the header, the aside, the keys — and the MCP bridge — use. Every change of rule or
/// first row restarts the thread.
impl State {
    /// The rule number (0–255).
    pub fn rule(&self) -> u8 {
        self.weave.rule().number()
    }

    /// Sets the rule and restarts the thread (LOOM-4).
    pub fn set_rule(&mut self, rule: u8) {
        self.weave.set_rule(rule);
        self.rethread();
    }

    /// Flips the output of neighbourhood pattern `p` (0–7, `left·4 + centre·2 + right`): a
    /// rule key press (LOOM-3). Restarts the thread.
    pub fn toggle_key(&mut self, p: u8) {
        self.set_rule(self.weave.rule().toggled(p).number());
    }

    /// −1 (wraps 0 → 255).
    pub fn previous_rule(&mut self) {
        self.set_rule(self.rule().wrapping_sub(1));
    }

    /// +1 (wraps 255 → 0).
    pub fn next_rule(&mut self) {
        self.set_rule(self.rule().wrapping_add(1));
    }

    /// Surprise me: a different rule drawn from the screen's seeded generator; returns it.
    pub fn surprise(&mut self) -> u8 {
        // one of the 255 other rules, so the press always changes something
        let offset = 1 + self.rng.below(255) as u8;
        let rule = self.rule().wrapping_add(offset);
        self.set_rule(rule);
        rule
    }

    /// How the thread starts.
    pub fn start(&self) -> WeaveStart {
        self.weave.start()
    }

    /// Sets how the thread starts (exactly this seed for [`WeaveStart::Random`]) and restarts.
    pub fn set_start(&mut self, start: WeaveStart) {
        self.weave.set_start(start);
        self.ticker.reset();
    }

    /// The First row segment: Single cell, or Random with a freshly drawn row (every press).
    pub fn pick_first_row(&mut self, first: FirstRow) {
        let start = match first {
            FirstRow::Single => WeaveStart::Single,
            FirstRow::Random => WeaveStart::Random(self.rng.next_u64()),
        };
        self.set_start(start);
    }

    /// New thread: starts over from the first row (a new random row if the start is Random).
    pub fn restart(&mut self) {
        self.rethread();
    }

    /// Whether the loom is weaving.
    pub fn running(&self) -> bool {
        self.running
    }

    /// Weave / Pause.
    pub fn set_running(&mut self, running: bool) {
        if running && !self.running {
            self.ticker.reset();
        }
        self.running = running;
    }

    /// Rows per second.
    pub fn speed(&self) -> f32 {
        self.speed
    }

    /// Sets the speed, rounded and clamped to 2–40 rows/s.
    pub fn set_speed(&mut self, speed: f32) {
        if speed.is_finite() {
            self.speed = speed
                .round()
                .clamp(*SPEED_RANGE.start(), *SPEED_RANGE.end());
        }
    }

    /// Weaves `n` rows now (at most [`MAX_STEP_N`]), running or not.
    pub fn step_n(&mut self, n: u32) {
        for _ in 0..n.min(MAX_STEP_N) {
            self.weave.weave();
        }
    }

    /// Rows woven since the start, the first row included (the `row 0000` counter).
    pub fn row_count(&self) -> u64 {
        self.weave.count()
    }

    /// The visible rows (oldest first, at most 72), one line each, [`ALIVE_CHAR`] /
    /// [`DEAD_CHAR`].
    pub fn rows_text(&self) -> String {
        self.weave.rows_text(ALIVE_CHAR, DEAD_CHAR)
    }

    /// What Share copies (LOOM-6): `"Rule N"`, then [`State::rows_text`].
    pub fn share_text(&self) -> String {
        format!("Rule {}\n{}", self.rule(), self.rows_text())
    }

    /// The thread itself (rows, width, progress).
    pub fn weave(&self) -> &Weave {
        &self.weave
    }

    /// The rule as an engine value (binary, blurb, famous thread).
    pub fn elementary(&self) -> Elementary {
        self.weave.rule()
    }

    /// The fabric's accessible description (A11Y-4): "Woven pattern, rule 30, 121 cells
    /// wide, 47 rows woven".
    pub fn description(&self) -> String {
        ap_bench::a11y::loom(self.rule(), self.weave.width(), self.weave.count())
    }

    /// Restarts the thread; a Random start draws a new row (the mockup's `fresh(start)`).
    fn rethread(&mut self) {
        match self.weave.start() {
            WeaveStart::Single => self.weave.restart(),
            WeaveStart::Random(_) => self
                .weave
                .set_start(WeaveStart::Random(self.rng.next_u64())),
        }
        self.ticker.reset();
    }

    /// Advances the thread by the frame's time while weaving.
    fn tick(&mut self, ctx: &Ctx) {
        if !self.running {
            return;
        }
        let dt = ctx.dt();
        for _ in 0..self.ticker.steps(dt, self.speed) {
            self.weave.weave();
        }
        self.spool_secs += f64::from(dt);
        ctx.animate();
    }
}

impl Screen for State {
    fn enter(&mut self, ctx: &mut Ctx) {
        if !self.started {
            self.started = true;
            self.rng = Rng::new(ctx.rng_seed() ^ SEED_SALT);
            self.running = ctx.autoplay();
        }
    }

    fn header_actions(&mut self, ctx: &mut Ctx, ui: &mut Ui) {
        let run = if self.running {
            prim("Pause").icon(Icon::Pause)
        } else {
            prim("Weave").icon(Icon::Play)
        };
        if run.show(ui).clicked() {
            self.set_running(!self.running);
        }
        if ghost("New thread").icon(Icon::Restart).show(ui).clicked() {
            self.restart();
        }
        counter(ui, &format!("row {:04}", self.row_count()));
        if ghost("Share").show(ui).clicked() {
            ctx.copy(self.share_text());
            ctx.toast(format!(
                "Copied Rule {} and {} rows as text",
                self.rule(),
                self.weave.len()
            ));
        }
    }

    fn tab_actions(&mut self, _ctx: &mut Ctx, slot: &mut RowSlot) {
        slot.hint("Each row is woven from the one above.");
    }

    fn command(&mut self, cmd: Command, _ctx: &mut Ctx) -> bool {
        match cmd {
            Command::RunPause => self.set_running(!self.running),
            Command::Step => {
                // like the Tray's Step: pause, then one row
                self.set_running(false);
                self.step_n(1);
            }
        }
        true
    }

    fn ui(&mut self, ctx: &mut Ctx, ui: &mut Ui) {
        self.tick(ctx);
        layout::columns(ui, ctx, &COLUMNS, |ui, ctx, i| match i {
            0 => layout::bench(ui, ctx, self, |s, _ctx, stage| s.stage(stage)),
            _ => self.aside(ui),
        });
    }

    fn describe(&self) -> String {
        format!(
            "{}, {} at {} rows/s",
            self.description(),
            if self.running { "weaving" } else { "paused" },
            self.speed
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_and_steps_go_through_the_engine() {
        let mut s = State::default();
        assert_eq!(s.rule(), 30);
        s.toggle_key(0);
        assert_eq!(s.rule(), 31);
        s.set_rule(0);
        s.previous_rule();
        assert_eq!(s.rule(), 255);
        s.next_rule();
        assert_eq!(s.rule(), 0);
        s.set_rule(90);
        s.step_n(5);
        assert_eq!(s.row_count(), 6);
        assert!(s.share_text().starts_with("Rule 90\n"));
        assert_eq!(s.rows_text().lines().count(), 6);
    }

    #[test]
    fn surprise_always_changes_the_rule() {
        let mut s = State::default();
        for _ in 0..300 {
            let before = s.rule();
            assert_ne!(s.surprise(), before);
        }
    }

    #[test]
    fn random_restarts_draw_new_rows() {
        let mut s = State::default();
        s.pick_first_row(FirstRow::Random);
        let a = s.weave().newest().to_vec();
        s.restart();
        assert_ne!(s.weave().newest(), a.as_slice());
        assert_eq!(FirstRow::of(s.start()), FirstRow::Random);
        s.set_speed(99.0);
        assert_eq!(s.speed(), 40.0);
        s.set_speed(f32::NAN);
        assert_eq!(s.speed(), 40.0);
    }
}
