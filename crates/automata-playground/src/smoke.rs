//! `automata-playground --smoke <seconds> [--headless]` — a measurable start-up + run path for
//! `scripts/test-all.sh` and `scripts/check-bundle.sh`: the real [`AutomataApp`] opens on the
//! Tray with autoplay on, draws the frames it asks for during `<seconds>` of wall time (it
//! paces itself like the real app — a frame per generation or per pulse step, vsync while
//! visible — so `frames=` is its real frame count, not a spin) and prints one line
//!
//! ```text
//! SMOKE route=tray mode=window first_frame_ms=412 frames=180 gens=30 frame_ms_avg=1.9 frame_ms_max=7.2 status=ok
//! ```
//!
//! then exits 0 (1 on an error or when the window closes early, 2 on bad arguments). With
//! `--headless` or `AUTOMATA_HEADLESS=1` — or when no window can be opened — the same app draws
//! into an egui context without a window (layout + tessellation, paced by the repaint delay the
//! app asks for, no GPU).
//!
//! A smoke run never touches the user's state: no persistence (a temp storage folder), no
//! save dialogs, a launcher that opens nothing, a fixed seed. It reads nothing from the
//! checkout at run time (fonts are compiled in), so it runs from a sandboxed bundle copy.

use crate::app::{AppOptions, AutomataApp};
use crate::launcher::Launcher;
use crate::router::Route;
use eframe::egui;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Parsed `--smoke` arguments.
#[derive(Debug, Clone, PartialEq)]
pub struct SmokeArgs {
    /// Wall seconds to run after the first frame.
    pub seconds: f64,
    /// Draw without a window.
    pub headless: bool,
}

impl SmokeArgs {
    /// `None` when `--smoke` is not among the arguments.
    pub fn parse(args: &[String]) -> Option<Result<SmokeArgs, String>> {
        let i = args.iter().position(|a| a == "--smoke")?;
        Some(Self::parse_after(&args[i + 1..]))
    }

    fn parse_after(rest: &[String]) -> Result<SmokeArgs, String> {
        let mut a = SmokeArgs {
            seconds: 3.0,
            headless: false,
        };
        let mut it = rest.iter().peekable();
        if let Some(s) = it.peek()
            && !s.starts_with("--")
        {
            a.seconds = s
                .parse::<f64>()
                .map_err(|_| format!("not a number of seconds: {s}"))?;
            it.next();
        }
        for flag in it {
            match flag.as_str() {
                "--headless" => a.headless = true,
                other => {
                    return Err(format!(
                        "unknown smoke option {other} (usage: automata-playground --smoke <seconds> [--headless])"
                    ));
                }
            }
        }
        if !(a.seconds > 0.0 && a.seconds.is_finite()) {
            return Err("the smoke run needs a positive number of seconds".into());
        }
        Ok(a)
    }
}

/// What a smoke run measured.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SmokeReport {
    /// The route that ran.
    pub route: String,
    /// `window` or `headless`.
    pub mode: String,
    /// Process start → first frame.
    pub first_frame_ms: f64,
    /// Frames drawn after the first.
    pub frames: u64,
    /// Generations the Tray advanced.
    pub gens: u64,
    /// Mean frame time (the app's `frame`, ms).
    pub frame_ms_avg: f64,
    /// Longest frame (ms).
    pub frame_ms_max: f64,
    /// What went wrong.
    pub errors: Vec<String>,
}

impl SmokeReport {
    /// No errors.
    pub fn ok(&self) -> bool {
        self.errors.is_empty()
    }

    /// The one-line summary the scripts parse.
    pub fn line(&self) -> String {
        format!(
            "SMOKE route={} mode={} first_frame_ms={:.0} frames={} gens={} frame_ms_avg={:.2} frame_ms_max={:.2} status={}",
            self.route,
            self.mode,
            self.first_frame_ms,
            self.frames,
            self.gens,
            self.frame_ms_avg,
            self.frame_ms_max,
            if self.ok() {
                "ok".to_string()
            } else {
                format!("error: {}", self.errors.join("; "))
            }
        )
    }
}

/// The options of a smoke run's app.
pub fn app_options() -> AppOptions {
    AppOptions {
        start: Route::Tray,
        rng_seed: AppOptions::TEST_SEED,
        autoplay: true,
        save_dialog: false,
        load_persisted: false,
        launcher: Launcher::recording(&[]),
        out_dir: std::env::temp_dir().join(format!("automata-smoke-{}", std::process::id())),
        mcp_socket: None,
    }
}

/// Drives one run frame by frame.
struct Driver {
    seconds: f64,
    started: Instant,
    first: Option<Instant>,
    deadline: Instant,
    frame_sum: f64,
    report: SmokeReport,
    done: bool,
}

impl Driver {
    fn new(args: &SmokeArgs, mode: &str) -> Self {
        let now = Instant::now();
        Self {
            seconds: args.seconds,
            started: now,
            first: None,
            // generous for debug builds and slow machines
            deadline: now + Duration::from_secs_f64(30.0 + args.seconds * 5.0),
            frame_sum: 0.0,
            report: SmokeReport {
                route: Route::Tray.id().into(),
                mode: mode.into(),
                ..Default::default()
            },
            done: false,
        }
    }

    /// Wall time left until the run is due to end (from now).
    fn until_end(&self) -> Duration {
        let end = match self.first {
            Some(first) => first + Duration::from_secs_f64(self.seconds),
            None => self.deadline,
        };
        end.saturating_duration_since(Instant::now())
            .max(Duration::from_millis(1))
    }

    /// After the app drew a frame that took `dt`.
    fn after(&mut self, app: &AutomataApp, dt: Duration) {
        if self.done {
            return;
        }
        let now = Instant::now();
        match self.first {
            None => {
                self.first = Some(now);
                self.report.first_frame_ms = crate::STARTED
                    .get()
                    .unwrap_or(&self.started)
                    .elapsed()
                    .as_secs_f64()
                    * 1e3;
            }
            Some(first) => {
                let ms = dt.as_secs_f64() * 1e3;
                self.report.frames += 1;
                self.frame_sum += ms;
                self.report.frame_ms_max = self.report.frame_ms_max.max(ms);
                if now.duration_since(first).as_secs_f64() >= self.seconds {
                    self.finish(app);
                }
            }
        }
        if !self.done && now > self.deadline {
            self.report.errors.push("timed out".into());
            self.finish(app);
        }
    }

    fn finish(&mut self, app: &AutomataApp) {
        let r = &mut self.report;
        r.frame_ms_avg = self.frame_sum / r.frames.max(1) as f64;
        r.gens = app.screens().tray.generation;
        r.route = app.route().id().into();
        if app.route() != Route::Tray {
            r.errors
                .push(format!("left the Tray for {}", app.route().id()));
        }
        if !app.screens().tray.running {
            r.errors.push("the Tray wasn't running".into());
        }
        if r.frames == 0 {
            r.errors.push("no frames drawn".into());
        }
        self.done = true;
    }
}

/// The window around the app for a smoke run.
struct SmokeApp {
    app: AutomataApp,
    driver: Driver,
    out: Arc<Mutex<Option<SmokeReport>>>,
}

impl eframe::App for SmokeApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let t0 = Instant::now();
        self.app.frame(ui);
        self.driver.after(&self.app, t0.elapsed());
        if self.driver.done {
            if let Ok(mut o) = self.out.lock() {
                *o = Some(self.driver.report.clone());
            }
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        } else {
            // the app asks for its own frames; this only makes sure the run ends on time
            ui.ctx().request_repaint_after(self.driver.until_end());
        }
    }

    // never write the user's saved state
    fn save(&mut self, _storage: &mut dyn eframe::Storage) {}

    fn persist_egui_memory(&self) -> bool {
        false
    }
}

/// Runs in a real window (wgpu); `None` if no window could be opened.
pub fn run_windowed(args: &SmokeArgs) -> Option<SmokeReport> {
    crate::STARTED.get_or_init(Instant::now);
    let out = Arc::new(Mutex::new(None));
    let out2 = out.clone();
    let mut native = crate::native_options();
    native.persist_window = false;
    native.persistence_path = Some(std::env::temp_dir().join("automata-smoke-state"));
    let driver = Driver::new(args, "window");
    let r = eframe::run_native(
        "Automata Playground smoke",
        native,
        Box::new(move |cc| {
            Ok(Box::new(SmokeApp {
                app: AutomataApp::new(cc, app_options()),
                driver,
                out: out2,
            }))
        }),
    );
    if let Err(e) = r {
        log::warn!("smoke: no window ({e}); running headless");
        return None;
    }
    let report = out.lock().ok().and_then(|mut o| o.take());
    Some(report.unwrap_or_else(|| SmokeReport {
        route: Route::Tray.id().into(),
        mode: "window".into(),
        errors: vec!["the window closed before the run finished".into()],
        ..Default::default()
    }))
}

/// Runs without a window: the app draws into an egui context (layout and tessellation on the
/// CPU), sleeping between frames for the repaint delay the app asks for.
pub fn run_headless(args: &SmokeArgs) -> SmokeReport {
    crate::STARTED.get_or_init(Instant::now);
    let ctx = egui::Context::default();
    ap_theme::install(&ctx);
    let mut app = AutomataApp::with_options(app_options());
    let mut d = Driver::new(args, "headless");
    let t_start = Instant::now();
    while !d.done {
        let t0 = Instant::now();
        let raw = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1440.0, 900.0),
            )),
            time: Some(t_start.elapsed().as_secs_f64()),
            ..Default::default()
        };
        let mut out = ctx.run_ui(raw, |ui| app.frame(ui));
        let _ = ctx.tessellate(std::mem::take(&mut out.shapes), out.pixels_per_point);
        out.textures_delta.clear();
        let dt = t0.elapsed();
        d.after(&app, dt);
        // what the app asked for (`Duration::MAX` when idle), at least one 60 Hz frame
        let asked = out
            .viewport_output
            .get(&egui::ViewportId::ROOT)
            .map_or(Duration::MAX, |v| v.repaint_delay);
        let wait = asked
            .max(Duration::from_millis(16).saturating_sub(dt))
            .min(d.until_end());
        std::thread::sleep(wait);
    }
    d.report
}

/// `main`'s entry: runs, prints the report line, returns the exit code.
pub fn main(args: SmokeArgs) -> i32 {
    let headless = args.headless
        || std::env::var_os("AUTOMATA_HEADLESS").is_some_and(|v| !v.is_empty() && v != "0");
    let report = if headless {
        run_headless(&args)
    } else {
        run_windowed(&args).unwrap_or_else(|| run_headless(&args))
    };
    println!("{}", report.line());
    if report.ok() { 0 } else { 1 }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn parses_arguments() {
        assert_eq!(SmokeArgs::parse(&s(&["automata-playground"])), None);
        let a = SmokeArgs::parse(&s(&["x", "--smoke", "3"]))
            .unwrap()
            .unwrap();
        assert_eq!((a.seconds, a.headless), (3.0, false));
        let a = SmokeArgs::parse(&s(&["x", "--smoke", "0.5", "--headless"]))
            .unwrap()
            .unwrap();
        assert_eq!((a.seconds, a.headless), (0.5, true));
        let a = SmokeArgs::parse(&s(&["x", "--smoke"])).unwrap().unwrap();
        assert_eq!(a.seconds, 3.0, "default");
        for bad in [
            &["x", "--smoke", "abc"][..],
            &["x", "--smoke", "-1"],
            &["x", "--smoke", "2", "--nope"],
        ] {
            assert!(SmokeArgs::parse(&s(bad)).unwrap().is_err(), "{bad:?}");
        }
    }

    /// A short headless run draws the frames the app asks for, the Tray runs and advances, and
    /// the line has the fields the scripts parse.
    #[test]
    fn headless_smoke_runs_the_tray() {
        let r = run_headless(&SmokeArgs {
            seconds: 0.4,
            headless: true,
        });
        assert!(r.ok(), "{}", r.line());
        assert!(r.frames > 5, "{}", r.line());
        // paced like the app: ~30 frames a second (the callout's pulse), not 60 or a spin
        assert!(r.frames <= 16, "{}", r.line());
        assert!(r.gens > 0, "{}", r.line());
        let line = r.line();
        assert!(
            line.starts_with("SMOKE route=tray mode=headless first_frame_ms="),
            "{line}"
        );
        assert!(
            line.contains(" frames=") && line.ends_with("status=ok"),
            "{line}"
        );
    }
}
