//! The per-frame screen context: what a screen may ask of the app while it draws.
//!
//! A [`Ctx`] is built by the app each frame and handed to the active screen's hooks. Through it
//! a screen navigates (optionally carrying a rule / pattern to the next screen, NAV-4), shows
//! toasts, copies to the clipboard, saves files, reads the app's RNG seed and frame time, and
//! learns whether a text field has focus.

use crate::launcher::{AppId, Launch, Launcher};
use crate::router::Route;
use crate::widgets::Toasts;
use eframe::egui;
use std::path::PathBuf;

/// A rule (and optionally a pattern) handed from one screen to another (NAV-4): Rule lab
/// "Open in Playground" → Tray, Terrarium "Plant in the Tray" / "Analyse in Rule lab".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Carry {
    /// A rulestring, e.g. `B3/S23` (parse it with the engine).
    pub rule: String,
    /// An RLE pattern (with its `x = …, y = …, rule = …` header), if any.
    pub rle: Option<String>,
    /// Where it came from.
    pub from: Route,
}

impl Carry {
    /// A rule-only carry-over from `from`.
    pub fn rule(rule: impl Into<String>, from: Route) -> Self {
        Self {
            rule: rule.into(),
            rle: None,
            from,
        }
    }

    /// With an RLE pattern.
    pub fn with_rle(mut self, rle: impl Into<String>) -> Self {
        self.rle = Some(rle.into());
        self
    }
}

/// A navigation request made during a frame; the app applies it after the frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NavRequest {
    /// Where to go.
    pub route: Route,
    /// Payload delivered to the target's [`Screen::receive`](crate::screens::Screen::receive).
    pub carry: Option<Carry>,
    /// Series only: the app card to highlight (NAV-3).
    pub focus_app: Option<AppId>,
}

/// Keyboard commands the app routes to the active screen (NAV-5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Command {
    /// Space (when no widget has keyboard focus): Run ↔ Pause (Play ↔ Pause in the Rule lab).
    RunPause,
    /// `S` (when no text field has focus): one step (the Tray steps; others may ignore it).
    Step,
}

/// App services shared by every frame (owned by the app).
#[derive(Debug, Clone)]
pub struct Services {
    /// Opens sibling apps.
    pub launcher: Launcher,
    /// Where files are written when there's no save dialog (tests, headless).
    pub out_dir: PathBuf,
    /// Show the native save dialog (`false` in tests: write straight into `out_dir`).
    pub save_dialog: bool,
    /// The app's RNG seed (fixed in tests, from the clock otherwise).
    pub rng_seed: u64,
    /// Screens start running (Tray / Loom / Terrarium "run on start"); off in tests.
    pub autoplay: bool,
}

/// What the app keeps across frames on behalf of screens (clipboard log, saved files).
#[derive(Debug, Default, Clone)]
pub struct Outbox {
    /// The last text copied with [`Ctx::copy`].
    pub last_copied: Option<String>,
    /// Every file written with [`Ctx::save_text`], in order.
    pub saved: Vec<PathBuf>,
}

/// The screen context for one frame. See the module docs.
pub struct Ctx<'a> {
    pub(crate) route: Route,
    pub(crate) last_bench: Route,
    pub(crate) nav: Option<NavRequest>,
    pub(crate) toasts: &'a mut Toasts,
    pub(crate) services: &'a Services,
    pub(crate) outbox: &'a mut Outbox,
    pub(crate) egui: egui::Context,
    pub(crate) body_height: f32,
}

impl<'a> Ctx<'a> {
    /// The route being drawn.
    pub fn route(&self) -> Route {
        self.route
    }

    /// The Playground bench used last (where "Playground" goes).
    pub fn last_bench(&self) -> Route {
        self.last_bench
    }

    /// Navigate to `route` after this frame.
    pub fn go(&mut self, route: Route) {
        self.nav = Some(NavRequest {
            route,
            carry: None,
            focus_app: None,
        });
    }

    /// Navigate to `route` and hand it `carry` (NAV-4).
    pub fn go_with(&mut self, route: Route, carry: Carry) {
        self.nav = Some(NavRequest {
            route,
            carry: Some(carry),
            focus_app: None,
        });
    }

    /// Open the Series screen with `app`'s card highlighted (NAV-3).
    pub fn open_series(&mut self, app: AppId) {
        self.nav = Some(NavRequest {
            route: Route::Series,
            carry: None,
            focus_app: Some(app),
        });
    }

    /// The navigation requested so far this frame.
    pub fn pending(&self) -> Option<&NavRequest> {
        self.nav.as_ref()
    }

    /// Show a toast.
    pub fn toast(&mut self, text: impl Into<String>) {
        self.toasts.push(text);
    }

    /// Copy `text` to the system clipboard (and remember it: `AutomataApp::last_copied`).
    pub fn copy(&mut self, text: impl Into<String>) {
        let text = text.into();
        self.egui.copy_text(text.clone());
        self.outbox.last_copied = Some(text);
    }

    /// Save `contents` as a file named like `suggested` (e.g. `glider-garden.rle`): the native
    /// save dialog in the app, straight into the output folder in tests. Returns the path
    /// written, `Ok(None)` if the user cancelled.
    pub fn save_text(
        &mut self,
        suggested: &str,
        contents: &str,
    ) -> anyhow::Result<Option<PathBuf>> {
        let path = if self.services.save_dialog {
            let mut dialog = rfd::FileDialog::new().set_file_name(suggested);
            if let Some(ext) = std::path::Path::new(suggested)
                .extension()
                .and_then(|e| e.to_str())
            {
                dialog = dialog.add_filter(ext, &[ext]);
            }
            match dialog.save_file() {
                Some(p) => p,
                None => return Ok(None),
            }
        } else {
            std::fs::create_dir_all(&self.services.out_dir)?;
            self.services.out_dir.join(suggested)
        };
        std::fs::write(&path, contents)?;
        self.outbox.saved.push(path.clone());
        Ok(Some(path))
    }

    /// Open a sibling app; the result says whether it was found.
    pub fn launch(&mut self, app: AppId) -> Launch {
        self.services.launcher.launch(app)
    }

    /// The app's RNG seed (deterministic in tests). Derive per-screen seeds from it.
    pub fn rng_seed(&self) -> u64 {
        self.services.rng_seed
    }

    /// Whether screens should start running when first shown.
    pub fn autoplay(&self) -> bool {
        self.services.autoplay
    }

    /// The frame's time step in seconds (`stable_dt`, capped at 0.1 s so a stall doesn't
    /// jump the simulation). Accumulate it with a [`Ticker`].
    pub fn dt(&self) -> f32 {
        self.egui.input(|i| i.stable_dt).min(0.1)
    }

    /// True while a text field has keyboard focus (don't treat letters as shortcuts).
    pub fn text_focus(&self) -> bool {
        self.egui.text_edit_focused()
    }

    /// The height under the header (what side-by-side columns fill).
    pub fn body_height(&self) -> f32 {
        self.body_height
    }

    /// The egui context (repaints, input, memory).
    pub fn egui(&self) -> &egui::Context {
        &self.egui
    }

    /// Ask for another frame soon (call it every frame while running).
    pub fn animate(&self) {
        self.egui.request_repaint();
    }
}

/// Turns frame time into whole steps at a rate: `let n = ticker.steps(ctx.dt(), speed)`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Ticker {
    acc: f32,
}

impl Ticker {
    /// At most this many steps per frame (a long stall doesn't fast-forward).
    pub const MAX_STEPS: u32 = 8;

    /// Adds `dt` seconds and returns how many steps of `1 / rate` seconds are due.
    pub fn steps(&mut self, dt: f32, rate: f32) -> u32 {
        if rate <= 0.0 || !dt.is_finite() {
            return 0;
        }
        self.acc += dt.max(0.0);
        let period = 1.0 / rate;
        let n = (self.acc / period).floor();
        self.acc -= n * period;
        if n > Self::MAX_STEPS as f32 {
            self.acc = 0.0;
            return Self::MAX_STEPS;
        }
        n as u32
    }

    /// Forget the accumulated time (after a pause).
    pub fn reset(&mut self) {
        self.acc = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ticker_counts_steps() {
        let mut t = Ticker::default();
        assert_eq!(t.steps(0.05, 10.0), 0);
        assert_eq!(t.steps(0.05, 10.0), 1);
        assert_eq!(t.steps(0.25, 10.0), 2);
        assert_eq!(t.steps(0.05, 10.0), 1);
        assert_eq!(t.steps(10.0, 30.0), Ticker::MAX_STEPS);
        assert_eq!(t.steps(0.0, 30.0), 0);
        assert_eq!(t.steps(1.0, 0.0), 0);
    }

    #[test]
    fn carry_builders() {
        let c = Carry::rule("B36/S23", Route::Terrarium).with_rle("x = 1, y = 1\no!");
        assert_eq!(c.rule, "B36/S23");
        assert!(c.rle.is_some());
    }
}
