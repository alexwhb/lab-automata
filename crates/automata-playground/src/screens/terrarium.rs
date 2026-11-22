//! Terrarium · breed (SCR-3, TERR-1…4) — mockup `docs/design/mockup/Terrarium.dc.html`.
//!
//! Six jars, each a 32 × 32 torus running its own Life-like rule ([`ap_engine::Terrarium`],
//! seeded from [`Ctx::rng_seed`]), drawn as the tilted, bobbing jars of `ap-bench`
//! ([`ap_bench::JarScene`]) in an auto-filling grid of cards, with:
//!
//! - each card: the jar (its own ink, a shadow, the bob while running), the rule code and name
//!   ("Mutant" for unnamed rules), the Keep / Kept heart and the liveliness meter with "N
//!   alive". The jar reserves [`JarScene::FOOTPRINT`](ap_bench::JarScene::FOOTPRINT), so its
//!   slab never covers the code under it (the mockup's overlap is fixed, TERR-1);
//! - the aside: Breeding (Mutation 1–4 bit flips, "Breed brood N+1", which parents), the
//!   lineage (newest first) and "Use a favourite": Plant in the Tray / Analyse in Rule lab
//!   carry the first kept jar's rule, or jar 1's (NAV-4);
//! - the header's Run / Pause, Fresh soup, `brood N` and Share (the six codes and names).
//!
//! The jars tick every 140 ms while running (the mockup's interval). The
//! [control API](State#control-api) is what the MCP bridge drives; see
//! `docs/ARCHITECTURE.md` › "Screen control API".

mod aside;
mod jars;

use super::Screen;
use crate::ctx::{Carry, Command, Ctx, Ticker};
use crate::layout::{self, Column};
use crate::router::Route;
use crate::widgets::tabs::RowSlot;
use crate::widgets::{Icon, counter, ghost, prim};
use ap_engine::breed::MUTATION_RANGE;
use ap_engine::{LineageEntry, Terrarium};
use eframe::egui::Ui;

/// The page row: `main` (tab row + the scrolling jar grid) · the breeding aside (`flex: 1 0
/// 320px; max-width: 360px`, padding 16, gap 16).
pub const COLUMNS: [Column; 2] = [Column::main("Jars"), Column::readout("Breeding").gap(16.0)];

/// Seconds between jar ticks while running (the mockup's `setInterval(…, 140)`).
pub const TICK_SECS: f32 = 0.14;
/// `tick_n` runs at most this many generations per call.
pub const MAX_TICK_N: u32 = 10_000;
/// Mixed into [`Ctx::rng_seed`] so the jars' soups don't mirror other screens'.
const SEED_SALT: u64 = 0x5445_5252_4152_4955;

/// One jar as the control API reports it.
#[derive(Debug, Clone, PartialEq)]
pub struct JarInfo {
    /// The rule code, e.g. `B3/S23`.
    pub code: String,
    /// The rule's name ("Life", …) or "Mutant".
    pub name: &'static str,
    /// Live cells.
    pub population: usize,
    /// The liveliness meter, 0–1 (mean changed fraction over the last 12 ticks × 6).
    pub liveliness: f64,
    /// Whether the jar is kept as a parent.
    pub kept: bool,
}

/// The Terrarium's state: the engine bench, the run state and the bob clock.
#[derive(Debug, Clone)]
pub struct State {
    terr: Terrarium,
    running: bool,
    ticker: Ticker,
    /// Seconds of bobbing (advances while running, like the jars).
    bob_secs: f64,
    started: bool,
    /// Nothing has changed the bench yet: the first `enter` may re-seed it from the app seed.
    pristine: bool,
}

impl Default for State {
    fn default() -> Self {
        Self {
            terr: Terrarium::new(SEED_SALT),
            running: false,
            ticker: Ticker::default(),
            bob_secs: 0.0,
            started: false,
            pristine: true,
        }
    }
}

/// # Control API
///
/// What the header, the cards, the aside — and the MCP bridge — use.
impl State {
    /// The six jars: code, name, population, liveliness, kept.
    pub fn jars(&self) -> Vec<JarInfo> {
        self.terr
            .jars()
            .iter()
            .enumerate()
            .map(|(i, j)| JarInfo {
                code: j.rule().code(),
                name: j.name(),
                population: j.population(),
                liveliness: j.liveliness(),
                kept: self.terr.is_kept(i),
            })
            .collect()
    }

    /// Keeps (or un-keeps) jar `i` (0-based); out-of-range indices are ignored.
    pub fn keep(&mut self, i: usize, kept: bool) {
        if i < self.terr.jars().len() && self.terr.is_kept(i) != kept {
            self.terr.toggle_keep(i);
            self.pristine = false;
        }
    }

    /// Whether jar `i` is kept.
    pub fn is_kept(&self, i: usize) -> bool {
        self.terr.is_kept(i)
    }

    /// "Breed brood N+1" (TERR-2): kept jars carry over unchanged in the first slots, the rest
    /// are children of the parents; the lineage records the parents; keeps are cleared.
    pub fn breed(&mut self) {
        self.terr.breed();
        self.pristine = false;
    }

    /// Fresh soup: re-seeds every jar, same rules (TERR-4).
    pub fn fresh_soup(&mut self) {
        self.terr.reseed();
        self.pristine = false;
    }

    /// Bit flips per child (1–4).
    pub fn mutation(&self) -> u8 {
        self.terr.mutation() as u8
    }

    /// Sets the mutation, clamped to 1–4.
    pub fn set_mutation(&mut self, mutation: u8) {
        self.terr.set_mutation(usize::from(mutation));
    }

    /// Whether the jars are running.
    pub fn running(&self) -> bool {
        self.running
    }

    /// Run / Pause.
    pub fn set_running(&mut self, running: bool) {
        if running && !self.running {
            self.ticker.reset();
        }
        self.running = running;
    }

    /// Ticks every jar `n` times now (at most [`MAX_TICK_N`]), running or not.
    pub fn tick_n(&mut self, n: u32) {
        for _ in 0..n.min(MAX_TICK_N) {
            self.terr.tick();
        }
        if n > 0 {
            self.pristine = false;
        }
    }

    /// The brood number (1 before any breeding).
    pub fn brood(&self) -> u32 {
        self.terr.brood()
    }

    /// The lineage, newest first (last 8 broods): each brood's number and its parents' codes.
    pub fn lineage(&self) -> Vec<LineageEntry> {
        self.terr.lineage().newest_first().cloned().collect()
    }

    /// The rule code "Plant in the Tray" / "Analyse in Rule lab" carry: the first kept jar's,
    /// or jar 1's (NAV-4).
    pub fn chosen_rule(&self) -> String {
        self.terr.chosen_rule().code()
    }

    /// The note under the breed button ("None kept: all six become parents" / "N kept as
    /// parents").
    pub fn keep_note(&self) -> String {
        self.terr.keep_note()
    }

    /// What Share copies (TERR-4): one line per jar, `Jar 1 · B3/S23 · Life`.
    pub fn share_text(&self) -> String {
        self.terr.share_text()
    }

    /// The engine bench (jars' grids, activity).
    pub fn terrarium(&self) -> &Terrarium {
        &self.terr
    }

    /// The carry-over for NAV-4, from this screen.
    fn carry(&self) -> Carry {
        Carry::rule(self.chosen_rule(), Route::Terrarium)
    }

    /// Plant in the Tray / Analyse in Rule lab.
    fn send_to(&self, ctx: &mut Ctx, route: Route) {
        ctx.go_with(route, self.carry());
    }

    /// Ticks the jars by the frame's time while running.
    fn tick(&mut self, ctx: &Ctx) {
        if !self.running {
            return;
        }
        let dt = ctx.dt();
        for _ in 0..self.ticker.steps(dt, 1.0 / TICK_SECS) {
            self.terr.tick();
        }
        self.bob_secs += f64::from(dt);
        ctx.animate();
    }
}

impl Screen for State {
    fn enter(&mut self, ctx: &mut Ctx) {
        if !self.started {
            self.started = true;
            if self.pristine {
                let mutation = self.terr.mutation();
                self.terr = Terrarium::new(ctx.rng_seed() ^ SEED_SALT);
                self.terr.set_mutation(mutation);
            }
            self.running = ctx.autoplay();
        }
    }

    fn header_actions(&mut self, ctx: &mut Ctx, ui: &mut Ui) {
        let run = if self.running {
            prim("Pause").icon(Icon::Pause)
        } else {
            prim("Run").icon(Icon::Play)
        };
        if run.show(ui).clicked() {
            self.set_running(!self.running);
        }
        if ghost("Fresh soup").show(ui).clicked() {
            self.fresh_soup();
        }
        counter(ui, &format!("brood {}", self.brood()));
        if ghost("Share").show(ui).clicked() {
            ctx.copy(self.share_text());
            ctx.toast(format!(
                "Copied the {} jars' rules",
                number_word(self.terr.jars().len())
            ));
        }
    }

    fn tab_actions(&mut self, _ctx: &mut Ctx, slot: &mut RowSlot) {
        slot.hint("Keep the jars you like, then breed.");
    }

    fn command(&mut self, cmd: Command, _ctx: &mut Ctx) -> bool {
        match cmd {
            Command::RunPause => self.set_running(!self.running),
            Command::Step => {
                self.set_running(false);
                self.tick_n(1);
            }
        }
        true
    }

    fn ui(&mut self, ctx: &mut Ctx, ui: &mut Ui) {
        self.tick(ctx);
        layout::columns(ui, ctx, &COLUMNS, |ui, ctx, i| match i {
            0 => layout::bench(ui, ctx, self, |s, _ctx, stage| s.jar_grid(stage)),
            _ => self.aside(ctx, ui),
        });
    }

    fn describe(&self) -> String {
        let jars: Vec<String> = self
            .jars()
            .iter()
            .map(|j| format!("{} {} ({} alive)", j.code, j.name, j.population))
            .collect();
        format!(
            "Terrarium, brood {}, {}: {}",
            self.brood(),
            if self.running { "running" } else { "paused" },
            jars.join(", ")
        )
    }
}

/// "six" for 6 (the Share toast and the keep note speak in words).
fn number_word(n: usize) -> String {
    const WORDS: [&str; 10] = [
        "zero", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine",
    ];
    WORDS
        .get(n)
        .map_or_else(|| n.to_string(), |w| (*w).to_string())
}

/// The mutation slider's range as `f32`.
fn mutation_range() -> std::ops::RangeInclusive<f32> {
    *MUTATION_RANGE.start() as f32..=*MUTATION_RANGE.end() as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn control_api_round_trips() {
        let mut s = State::default();
        assert_eq!(s.jars().len(), 6);
        assert_eq!(s.jars()[0].code, "B3/S23");
        s.keep(2, true);
        s.keep(2, true);
        s.keep(9, true);
        assert!(s.is_kept(2));
        assert_eq!(s.keep_note(), "1 kept as parents");
        assert_eq!(s.chosen_rule(), "B3678/S34678");
        s.set_mutation(9);
        assert_eq!(s.mutation(), 4);
        s.breed();
        assert_eq!(s.brood(), 2);
        assert_eq!(s.jars()[0].code, "B3678/S34678");
        assert_eq!(s.lineage()[0].parents, ["B3678/S34678"]);
        assert!(!s.pristine);
        assert_eq!(number_word(6), "six");
        assert_eq!(number_word(12), "12");
    }
}
