//! External control of the running app: the app side of the live MCP bridge (SPEC DONE-5).
//!
//! The app listens on a user-only Unix socket ([`lab_mcp::bridge::BridgeServer`]) at
//! [`AppOptions::mcp_socket`](crate::AppOptions::mcp_socket); `automata-mcp` connects to it to
//! act on the running app. [`poll`] runs at the start of every frame on the UI thread: it takes
//! the requests that arrived, applies them and sends the replies.
//!
//! | method | params | reply |
//! |---|---|---|
//! | `status` | none | screen, window size, pid, version |
//! | `navigate` | `{"screen": id}` | status after navigating |
//! | `screenshot` | `{"screen"?: id}` | PNG of the next rendered frame (base64) |
//! | `describe` | `{"screen"?: id}` | outline of the AccessKit tree of the next frame + the screen's one-line `summary` |
//! | `tray.*`, `loom.*`, `terrarium.*`, `lab.*` | see [`ap_shared::proto`] | the screen's state after the call |
//!
//! The screen methods drive the screens' control APIs (`docs/ARCHITECTURE.md` › "Screen
//! control API") through [`AutomataApp::screens_mut`]. Their params are checked before anything
//! changes; with `show` (the default) the app switches to that screen. A screen that has never
//! been shown is entered first ([`AutomataApp::enter_screen`]), so its first-visit set-up (run
//! state from autoplay, the Terrarium's seeded soups) happens before the change, not after it.
//!
//! Screenshots and descriptions are answered a frame or more later. After navigating, the app
//! lets [`SETTLE_FRAMES`] frames pass first so the new screen's layout has settled. A screenshot
//! comes from `ViewportCommand::Screenshot` (eframe reads back the rendered frame; egui_kittest
//! renders it with wgpu) and is PNG-encoded on a worker thread. The description comes from the
//! AccessKit tree egui builds at the end of a frame, caught by a small egui plugin; the first
//! `describe` turns AccessKit on for the rest of the session (a screen reader would do the same).
//!
//! Backgrounded windows: every arriving request wakes the app (`ctx.request_repaint()` from the
//! bridge thread). While its window is minimized or covered, eframe skips `App::ui` (so [`poll`]
//! doesn't run) and calls only `App::logic`. That calls [`logic`], which queues the requests and
//! sends `ViewportCommand::RequestPaintWhileHidden` plus a repaint while anything is pending, so
//! the next frame runs the UI and paints, and a hidden window still answers.
//!
//! Mirrors `../chem-playground/crates/chem-playground/src/control.rs`; the AccessKit capture and
//! outline are candidates for `lab-mcp`.

use crate::app::AutomataApp;
use crate::router::Route;
use crate::screens::{loom, rule_lab, terrarium, tray};
use ap_engine::{Liveliness, Neighbourhood, RuleScript, WeaveStart, presets};
use ap_shared::input;
use ap_shared::proto::{self as ap, MAX_STEPS};
use eframe::egui::{self, Context, Ui, accesskit};
use lab_mcp::bridge::proto::{
    self, AccessNode, AppStatus, Description, ImagePayload, ScreenParams, ScreenRef, WindowInfo,
};
use lab_mcp::bridge::{BridgeServer, PendingRequest, RemoteError, code};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Frames to let pass after navigating before a screenshot or description is taken.
pub const SETTLE_FRAMES: u8 = 2;

/// Bridge state owned by the app.
#[derive(Default)]
pub struct Control {
    server: Option<BridgeServer>,
    waker_installed: bool,
    /// Requests taken from the bridge by [`logic`], for the next [`poll`].
    inbox: Vec<PendingRequest>,
    /// Screenshots and descriptions waiting for their frame.
    waiting: Vec<Waiting>,
    /// Screens already entered for the bridge.
    prepared: Vec<Route>,
    /// Soups seeded through the bridge without a seed (varies the default seed).
    soups: u64,
}

struct Waiting {
    req: PendingRequest,
    kind: Kind,
    /// Frames still to let pass before acting.
    settle: u8,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Screenshot,
    /// Waiting to be captured by the AccessKit plugin.
    Describe,
}

impl Control {
    /// Starts listening on `socket` (or does nothing for `None`). Never fails the app start-up:
    /// problems (another instance owns the socket, a bad path) are logged and the app runs
    /// without a bridge.
    pub fn start(socket: Option<&Path>) -> Self {
        let Some(path) = socket else {
            return Self::default();
        };
        match BridgeServer::bind(path) {
            Ok(server) => Self {
                server: Some(server),
                ..Self::default()
            },
            Err(e) => {
                log::warn!("MCP bridge off: {e}");
                Self::default()
            }
        }
    }

    /// Where the bridge listens, if it is on.
    pub fn socket(&self) -> Option<&Path> {
        self.server.as_ref().map(BridgeServer::path)
    }
}

impl std::fmt::Debug for Control {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Control")
            .field("socket", &self.socket())
            .field("inbox", &self.inbox.len())
            .field("waiting", &self.waiting.len())
            .finish_non_exhaustive()
    }
}

/// The socket the real app listens on: `$AUTOMATA_MCP_SOCKET`, else the per-user path (`None`
/// when the variable is empty or `off`). See [`ap_shared::bridge_socket_path`].
pub fn default_socket_path() -> Option<PathBuf> {
    ap_shared::bridge_socket_path()
}

/// Called from `App::logic`: every frame before `App::ui`, and also while the window is hidden
/// (minimized or covered), when eframe runs no UI pass. Pending requests ask for a painted frame
/// so that [`poll`] runs and a screenshot has pixels.
pub fn logic(app: &mut AutomataApp, ctx: &Context) {
    let control = &mut app.control;
    let Some(server) = control.server.as_ref() else {
        return;
    };
    install_waker(server, &mut control.waker_installed, ctx);
    control.inbox.extend(server.poll());
    if !control.inbox.is_empty() || !control.waiting.is_empty() {
        ctx.send_viewport_cmd(egui::ViewportCommand::RequestPaintWhileHidden);
        ctx.request_repaint();
    }
}

/// Called at the start of every frame (before the screen is entered and drawn), so a
/// navigation or a change takes effect in the same frame.
pub fn poll(app: &mut AutomataApp, ui: &mut Ui) {
    let ctx = ui.ctx().clone();
    let control = &mut app.control;
    let Some(server) = control.server.as_ref() else {
        return;
    };
    install_waker(server, &mut control.waker_installed, &ctx);
    let mut requests = std::mem::take(&mut control.inbox);
    requests.extend(server.poll());
    let any = !requests.is_empty();
    for req in requests {
        handle(app, &ctx, req);
    }
    advance_waiting(app, &ctx);
    if any || !app.control.waiting.is_empty() {
        ctx.request_repaint();
    }
}

/// Requests received but not yet answered (queued, or waiting for their frame).
pub fn pending(app: &AutomataApp) -> usize {
    app.control.inbox.len() + app.control.waiting.len()
}

fn install_waker(server: &BridgeServer, installed: &mut bool, ctx: &Context) {
    if !*installed {
        let waker = ctx.clone();
        server.set_waker(move || waker.request_repaint());
        *installed = true;
    }
}

fn handle(app: &mut AutomataApp, ctx: &Context, req: PendingRequest) {
    match req.method() {
        proto::STATUS => {
            let reply = to_value(&status(app, ctx));
            req.reply(reply);
        }
        proto::NAVIGATE => match screen_param(&req) {
            Ok(Some(route)) => {
                app.navigate(route);
                let reply = to_value(&status(app, ctx));
                req.reply(reply);
            }
            Ok(None) => req.reply(Err(RemoteError::invalid_params(format!(
                "`screen` is required: one of {}",
                ap_shared::screen_ids().join(", ")
            )))),
            Err(e) => req.reply(Err(e)),
        },
        method @ (proto::SCREENSHOT | proto::DESCRIBE) => {
            let kind = if method == proto::SCREENSHOT {
                Kind::Screenshot
            } else {
                Kind::Describe
            };
            let settle = match screen_param(&req) {
                Ok(Some(route)) if route != app.route() => {
                    app.navigate(route);
                    SETTLE_FRAMES
                }
                Ok(_) => 0,
                Err(e) => return req.reply(Err(e)),
            };
            if kind == Kind::Describe {
                ctx.enable_accesskit();
                ctx.add_plugin(AccessCapture::default()); // ignored if already added
            }
            app.control.waiting.push(Waiting { req, kind, settle });
        }
        method if ap::METHODS.contains(&method) => {
            let reply = screen_method(app, ctx, &req);
            req.reply(reply);
        }
        other => {
            let err = RemoteError::method_not_found(other);
            req.reply(Err(err));
        }
    }
}

fn advance_waiting(app: &mut AutomataApp, ctx: &Context) {
    let route = app.route();
    let screen = screen_ref(route);
    let summary = app.screens().get_ref(route).describe();
    let captured = ctx
        .with_plugin::<AccessCapture, _>(|p| p.captured.take())
        .flatten();
    let mut still_waiting = Vec::new();
    let mut want_capture = false;
    for mut w in std::mem::take(&mut app.control.waiting) {
        if w.req.is_abandoned() {
            continue; // the client left; nobody wants the answer
        }
        if w.settle > 0 {
            w.settle -= 1;
            want_capture |= w.kind == Kind::Describe;
            still_waiting.push(w);
            continue;
        }
        match w.kind {
            Kind::Screenshot => request_screenshot(ctx, w.req, screen.clone()),
            Kind::Describe => match &captured {
                Some(nodes) => {
                    let d = Description {
                        screen: screen.clone(),
                        outline: AccessNode::outline(nodes),
                        nodes: nodes.clone(),
                    };
                    let reply = to_value(&d).map(|mut v| {
                        if let Value::Object(m) = &mut v {
                            m.insert("summary".into(), Value::String(summary.clone()));
                        }
                        v
                    });
                    w.req.reply(reply);
                }
                None => {
                    want_capture = true;
                    still_waiting.push(w);
                }
            },
        }
    }
    app.control.waiting = still_waiting;
    if want_capture {
        // Capture at the end of this frame (or the next, if AccessKit just came on).
        ctx.with_plugin::<AccessCapture, _>(|p| p.armed = true);
    }
}

/// Asks the integration for this frame's pixels; the PNG is encoded off the UI thread.
fn request_screenshot(ctx: &Context, req: PendingRequest, screen: ScreenRef) {
    ctx.request_screenshot(move |image: Arc<egui::ColorImage>| {
        let job = move || {
            let [w, h] = image.size;
            let rgba: Vec<u8> = image
                .pixels
                .iter()
                .flat_map(|c| c.to_srgba_unmultiplied())
                .collect();
            let reply = u32::try_from(w)
                .and_then(|w| u32::try_from(h).map(|h| (w, h)))
                .map_err(|_| RemoteError::new(code::INTERNAL_ERROR, "screenshot is too large"))
                .and_then(|(w, h)| {
                    ImagePayload::png_from_rgba(w, h, &rgba, screen).map_err(|e| {
                        RemoteError::new(code::INTERNAL_ERROR, format!("PNG encoding failed: {e}"))
                    })
                })
                .and_then(|img| to_value(&img));
            req.reply(reply);
        };
        // If no thread can be spawned the request drops, which answers it with an error.
        let _ = std::thread::Builder::new()
            .name("automata-screenshot-png".into())
            .spawn(job);
    });
}

fn status(app: &AutomataApp, ctx: &Context) -> AppStatus {
    let rect = ctx.content_rect();
    AppStatus {
        app: crate::APP_NAME.to_owned(),
        version: env!("CARGO_PKG_VERSION").to_owned(),
        pid: std::process::id(),
        screen: screen_ref(app.route()),
        window: Some(WindowInfo {
            width: rect.width(),
            height: rect.height(),
            pixels_per_point: ctx.pixels_per_point(),
        }),
    }
}

/// A route as the bridge reports it.
pub fn screen_ref(route: Route) -> ScreenRef {
    match ap_shared::screen(route.id()) {
        Some(s) => s.screen_ref(),
        None => ScreenRef {
            id: route.id().to_owned(),
            label: route.title().to_owned(),
            mode: None,
        },
    }
}

/// The optional `screen` param as a route (`Err` for an unknown id).
fn screen_param(req: &PendingRequest) -> Result<Option<Route>, RemoteError> {
    let p: ScreenParams = req.parse_params()?;
    match p.screen {
        None => Ok(None),
        Some(id) => Route::from_id(&id).map(Some).ok_or_else(|| {
            RemoteError::invalid_params(format!(
                "unknown screen `{id}`; screens are {}",
                ap_shared::screen_ids().join(", ")
            ))
        }),
    }
}

fn to_value(v: &impl serde::Serialize) -> Result<Value, RemoteError> {
    serde_json::to_value(v).map_err(|e| RemoteError::new(code::INTERNAL_ERROR, e.to_string()))
}

fn invalid(message: impl Into<String>) -> RemoteError {
    RemoteError::invalid_params(message)
}

// ---- the screens' methods -------------------------------------------------------------------

/// Enters `route` once (if the bridge hasn't yet) and, with `show`, switches to it.
fn prepare(app: &mut AutomataApp, ctx: &Context, route: Route, show: bool) {
    if !app.control.prepared.contains(&route) {
        app.enter_screen(route, ctx);
        app.control.prepared.push(route);
    }
    if show && app.route() != route {
        app.navigate(route);
    }
}

fn steps(n: Option<u32>, what: &str) -> Result<Option<u32>, RemoteError> {
    match n {
        Some(n) if n == 0 || n > MAX_STEPS => Err(invalid(format!(
            "`{what}` must be 1 to {MAX_STEPS}, not {n}"
        ))),
        n => Ok(n),
    }
}

fn speed(v: Option<f32>, range: std::ops::RangeInclusive<f32>) -> Result<Option<f32>, RemoteError> {
    match v {
        Some(s) if !s.is_finite() || !range.contains(&s) => Err(invalid(format!(
            "`speed` must be {} to {}, not {s}",
            range.start(),
            range.end()
        ))),
        s => Ok(s),
    }
}

fn samples(n: Option<usize>) -> usize {
    n.unwrap_or(ap::DEFAULT_SAMPLES).clamp(2, 2001)
}

fn screen_method(
    app: &mut AutomataApp,
    ctx: &Context,
    req: &PendingRequest,
) -> Result<Value, RemoteError> {
    match req.method() {
        ap::TRAY_GET => {
            prepare(app, ctx, Route::Tray, false);
            to_value(&tray_state(&app.screens().tray, None))
        }
        ap::TRAY_SET_RULE => {
            let p: ap::TraySetRule = req.parse_params()?;
            let rule = input::rule_input(&p.rule, Neighbourhood::MOORE).map_err(invalid)?;
            prepare(app, ctx, Route::Tray, p.show);
            app.screens_mut().tray.set_rule(rule);
            to_value(&tray_state(&app.screens().tray, None))
        }
        ap::TRAY_LOAD_PATTERN => {
            let p: ap::TrayLoadPattern = req.parse_params()?;
            let input = input::pattern_input(&p.pattern).map_err(invalid)?;
            let mut notes = Vec::new();
            let rule = match (&p.rule, &input.file.rule) {
                (Some(r), _) => Some(input::rule_input(r, Neighbourhood::MOORE).map_err(invalid)?),
                (None, Some(header)) => match input::rule_input(header, Neighbourhood::MOORE) {
                    Ok(rule) => {
                        notes.push(format!("rule {} from the pattern", rule.code()));
                        Some(rule)
                    }
                    Err(e) => {
                        notes.push(format!(
                            "ignored the pattern's `rule = {header}` ({e}); the rule is unchanged"
                        ));
                        None
                    }
                },
                (None, None) => None,
            };
            prepare(app, ctx, Route::Tray, p.show);
            let tray = &mut app.screens_mut().tray;
            if let Some(rule) = rule {
                tray.set_rule(rule);
            }
            let pat = &input.file.pattern;
            let cropped = tray.load_pattern(pat);
            let name = input.file.name.as_deref().unwrap_or("the pattern");
            let mut note = format!(
                "Loaded {name} ({}×{}, {} cells)",
                pat.width(),
                pat.height(),
                pat.population()
            );
            if cropped {
                note.push_str(&format!(
                    ", cropped to {} × {} around its centre",
                    presets::TRAY_W,
                    presets::TRAY_H
                ));
            }
            for n in notes {
                note.push_str("; ");
                note.push_str(&n);
            }
            note.push('.');
            to_value(&tray_state(&app.screens().tray, Some(note)))
        }
        ap::TRAY_CONTROL => {
            let p: ap::TrayControl = req.parse_params()?;
            let n = steps(p.n, "n")?;
            let speed = speed(p.speed, tray::SPEED_RANGE)?;
            if n.is_some() && p.action != ap::TrayAction::Step {
                return Err(invalid("`n` only applies to action `step`"));
            }
            if p.seed.is_some() && p.action != ap::TrayAction::Randomize {
                return Err(invalid("`seed` only applies to action `randomize`"));
            }
            let seed = p.seed.unwrap_or_else(|| {
                app.control.soups += 1;
                app.services().rng_seed ^ app.control.soups.wrapping_mul(0x9E37_79B9_7F4A_7C15)
            });
            prepare(app, ctx, Route::Tray, p.show);
            let tray = &mut app.screens_mut().tray;
            if let Some(s) = speed {
                tray.set_speed(s);
            }
            let mut note = None;
            match p.action {
                ap::TrayAction::Run => tray.set_running(true),
                ap::TrayAction::Pause => tray.set_running(false),
                ap::TrayAction::Step => {
                    tray.set_running(false);
                    tray.step_n(n.unwrap_or(1));
                }
                ap::TrayAction::Clear => tray.clear(),
                ap::TrayAction::Restart => tray.restart(),
                ap::TrayAction::Randomize => {
                    tray.randomize(seed);
                    note = Some(format!("Random soup from seed {seed}."));
                }
            }
            to_value(&tray_state(&app.screens().tray, note))
        }
        ap::LOOM_GET => {
            let p: ap::LoomGet = req.parse_params()?;
            prepare(app, ctx, Route::Loom, false);
            to_value(&loom_state(&app.screens().loom, p.rows, None))
        }
        ap::LOOM_SET => {
            let p: ap::LoomSet = req.parse_params()?;
            let step = steps(p.step, "step")?;
            let speed = speed(p.speed, loom::SPEED_RANGE)?;
            if p.seed.is_some() && p.start == Some(ap::FirstRow::Single) {
                return Err(invalid("`seed` only applies to `start: random`"));
            }
            prepare(app, ctx, Route::Loom, p.show);
            let l = &mut app.screens_mut().loom;
            if let Some(rule) = p.rule {
                l.set_rule(rule);
            }
            match (p.start, p.seed) {
                (_, Some(seed)) => l.set_start(WeaveStart::Random(seed)),
                (Some(ap::FirstRow::Single), None) => l.set_start(WeaveStart::Single),
                (Some(ap::FirstRow::Random), None) => l.pick_first_row(loom::FirstRow::Random),
                (None, None) => {}
            }
            if p.restart {
                l.restart();
            }
            if let Some(s) = speed {
                l.set_speed(s);
            }
            if let Some(r) = p.running {
                l.set_running(r);
            }
            if let Some(n) = step {
                l.step_n(n);
            }
            to_value(&loom_state(&app.screens().loom, p.rows, None))
        }
        ap::TERRARIUM_GET => {
            prepare(app, ctx, Route::Terrarium, false);
            to_value(&terrarium_state(&app.screens().terrarium, None))
        }
        ap::TERRARIUM_KEEP => {
            let p: ap::TerrariumKeep = req.parse_params()?;
            let count = app.screens().terrarium.jars().len();
            if p.jars.is_empty() {
                return Err(invalid(format!("`jars` lists jar numbers, 1 to {count}")));
            }
            if let Some(bad) = p.jars.iter().find(|&&j| j == 0 || j > count) {
                return Err(invalid(format!(
                    "there is no jar {bad}; jars are numbered 1 to {count}"
                )));
            }
            prepare(app, ctx, Route::Terrarium, p.show);
            let t = &mut app.screens_mut().terrarium;
            for &j in &p.jars {
                t.keep(j - 1, p.kept);
            }
            to_value(&terrarium_state(&app.screens().terrarium, None))
        }
        ap::TERRARIUM_BREED => {
            let p: ap::TerrariumBreed = req.parse_params()?;
            if let Some(m) = p.mutation
                && !(1..=4).contains(&m)
            {
                return Err(invalid(format!("`mutation` must be 1 to 4, not {m}")));
            }
            let ticks = match p.ticks {
                Some(0) | None => None,
                t => steps(t, "ticks")?,
            };
            prepare(app, ctx, Route::Terrarium, p.show);
            let t = &mut app.screens_mut().terrarium;
            if let Some(m) = p.mutation {
                t.set_mutation(m);
            }
            t.breed();
            if let Some(n) = ticks {
                t.tick_n(n);
            }
            let parents = t
                .lineage()
                .first()
                .map(|e| e.parents.join(", "))
                .unwrap_or_default();
            let note = format!(
                "Bred brood {} from {parents}{}.",
                t.brood(),
                ticks.map_or(String::new(), |n| format!(", then ran {n} ticks"))
            );
            to_value(&terrarium_state(&app.screens().terrarium, Some(note)))
        }
        ap::LAB_GET => {
            let p: ap::LabGet = req.parse_params()?;
            prepare(app, ctx, Route::RuleLab, false);
            // a run typed into the editor may still compute: answer with the spec's own
            app.screens_mut().rule_lab.finish();
            to_value(&lab_state(
                &app.screens().rule_lab,
                samples(p.samples),
                None,
            ))
        }
        ap::LAB_SET_SCRIPT => {
            let p: ap::LabSetScript = req.parse_params()?;
            // check first: a script with errors changes nothing, not even the screen
            let checked = RuleScript::parse(&p.script).map(|_| ());
            if checked.is_ok() {
                prepare(app, ctx, Route::RuleLab, p.show);
            }
            let lab = &mut app.screens_mut().rule_lab;
            lab.finish();
            if let Err(errors) = checked.and_then(|()| lab.set_script(&p.script)) {
                let lines: Vec<String> = errors
                    .iter()
                    .map(|e| format!("line {}: {}", e.line, e.message))
                    .collect();
                return Err(invalid(format!(
                    "the script has {} error{}, so the Rule lab kept its script:\n{}",
                    errors.len(),
                    if errors.len() == 1 { "" } else { "s" },
                    lines.join("\n")
                )));
            }
            to_value(&lab_state(lab, samples(p.samples), None))
        }
        ap::LAB_SCRUB => {
            let p: ap::LabScrub = req.parse_params()?;
            prepare(app, ctx, Route::RuleLab, p.show);
            let lab = &mut app.screens_mut().rule_lab;
            lab.finish(); // `scrub` clamps to the spec's run, not the one on screen
            lab.scrub(p.generation);
            let note = (lab.generation() != p.generation).then(|| {
                format!(
                    "Generation {} is past the run; scrubbed to its last, {}.",
                    p.generation,
                    lab.generation()
                )
            });
            to_value(&lab_state(lab, samples(p.samples), note))
        }
        other => Err(RemoteError::method_not_found(other)),
    }
}

fn tray_state(t: &tray::State, note: Option<String>) -> ap::TrayState {
    let s = t.stats();
    let cartridge = presets::matching_cartridge(t.rule());
    ap::TrayState {
        rule: t.rule().code(),
        rule_name: cartridge.map_or("Custom rule", |c| c.name).to_owned(),
        cartridge: cartridge.map(|c| c.id.to_owned()),
        generation: s.generation,
        population: s.population,
        born: s.born,
        died: s.died,
        mood: ap::MoodInfo {
            title: s.mood_title.to_owned(),
            text: s.mood_text.to_owned(),
        },
        running: t.running(),
        speed: t.speed(),
        width: presets::TRAY_W,
        height: presets::TRAY_H,
        history: t.history().to_vec(),
        rle: t.rle(),
        description: t.description(),
        note,
    }
}

/// The default number of Loom rows in a reply.
const LOOM_ROWS: usize = 24;

fn loom_state(l: &loom::State, rows: Option<usize>, note: Option<String>) -> ap::LoomState {
    let e = l.elementary();
    let weave = l.weave();
    let keep = rows.unwrap_or(LOOM_ROWS).min(weave.len());
    let text = weave.rows_text('#', '.');
    let all: Vec<&str> = text.lines().collect();
    let (start, seed) = match l.start() {
        WeaveStart::Single => (ap::FirstRow::Single, None),
        WeaveStart::Random(s) => (ap::FirstRow::Random, Some(s)),
    };
    ap::LoomState {
        rule: l.rule(),
        binary: e.binary(),
        blurb: e.blurb(),
        famous: e.famous().map(|f| ap::FamousInfo {
            name: f.name.to_owned(),
            note: f.note.to_owned(),
        }),
        start,
        seed,
        running: l.running(),
        speed: l.speed(),
        row: l.row_count(),
        width: weave.width(),
        visible_rows: weave.len(),
        progress: weave.progress(),
        rows: all[all.len().saturating_sub(keep)..]
            .iter()
            .map(|s| (*s).to_owned())
            .collect(),
        description: l.description(),
        note,
    }
}

fn terrarium_state(t: &terrarium::State, note: Option<String>) -> ap::TerrariumState {
    ap::TerrariumState {
        brood: t.brood(),
        mutation: t.mutation(),
        running: t.running(),
        jars: t
            .jars()
            .into_iter()
            .enumerate()
            .map(|(i, j)| ap::JarState {
                number: i + 1,
                rule: j.code,
                name: j.name.to_owned(),
                population: j.population,
                liveliness: j.liveliness,
                level: match Liveliness::of(j.liveliness) {
                    Liveliness::Quiet => "quiet",
                    Liveliness::Lively => "lively",
                    Liveliness::Frantic => "frantic",
                }
                .to_owned(),
                kept: j.kept,
            })
            .collect(),
        keep_note: t.keep_note(),
        chosen_rule: t.chosen_rule(),
        lineage: t
            .lineage()
            .into_iter()
            .map(|e| ap::LineageInfo {
                brood: e.brood,
                parents: e.parents,
            })
            .collect(),
        note,
    }
}

fn lab_state(l: &rule_lab::State, samples: usize, note: Option<String>) -> ap::LabState {
    use crate::screens::Screen;
    ap::LabState {
        script: l.script_text(),
        errors: l.errors().iter().map(ap::ScriptErrorInfo::from).collect(),
        spec: ap::SpecInfo::from(&l.spec()),
        generation: l.generation(),
        alive: l.alive(),
        playing: l.playing,
        report: ap::ReportInfo::from(&l.report()),
        series: ap::sample_series(l.populations(), samples),
        rle: l.rle(),
        description: l.describe(),
        note,
    }
}

// ---- AccessKit ------------------------------------------------------------------------------

/// Catches the AccessKit tree egui produces at the end of a frame, when armed.
#[derive(Default)]
struct AccessCapture {
    armed: bool,
    captured: Option<Vec<AccessNode>>,
}

impl egui::Plugin for AccessCapture {
    fn debug_name(&self) -> &'static str {
        "automata-playground-access-capture"
    }

    fn output_hook(&mut self, _ctx: &Context, output: &mut egui::FullOutput) {
        if !self.armed {
            return;
        }
        if let Some(update) = &output.platform_output.accesskit_update {
            self.captured = Some(outline_nodes(update));
            self.armed = false;
        }
    }
}

/// Flattens an AccessKit tree into outline nodes: depth-first from the root, skipping hidden
/// subtrees and unnamed generic containers (their children move up a level).
pub fn outline_nodes(update: &accesskit::TreeUpdate) -> Vec<AccessNode> {
    use std::collections::{HashMap, HashSet};
    let by_id: HashMap<accesskit::NodeId, &accesskit::Node> =
        update.nodes.iter().map(|(id, n)| (*id, n)).collect();
    let Some(root) = update.tree.as_ref().map(|t| t.root) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    // (node, depth); a stack, children pushed in reverse to keep document order.
    let mut stack = vec![(root, 0u32)];
    let mut seen = HashSet::new();
    while let Some((id, depth)) = stack.pop() {
        if !seen.insert(id) {
            continue; // a malformed tree with a cycle
        }
        let Some(node) = by_id.get(&id) else {
            continue;
        };
        if node.is_hidden() {
            continue;
        }
        let name = node.label().map(str::to_owned).filter(|s| !s.is_empty());
        let value = node
            .value()
            .map(str::to_owned)
            .or_else(|| node.numeric_value().map(|v| format!("{v}")))
            .filter(|s| !s.is_empty());
        let role = node.role();
        let skip = name.is_none()
            && value.is_none()
            && matches!(
                role,
                accesskit::Role::GenericContainer | accesskit::Role::Unknown
            );
        let child_depth = if skip { depth } else { depth + 1 };
        if !skip {
            let mut states = Vec::new();
            if node.is_selected() == Some(true) {
                states.push("selected".to_owned());
            }
            match node.toggled() {
                Some(accesskit::Toggled::True) => states.push("checked".to_owned()),
                Some(accesskit::Toggled::Mixed) => states.push("mixed".to_owned()),
                _ => {}
            }
            if node.is_disabled() {
                states.push("disabled".to_owned());
            }
            if update.focus == id && id != root {
                states.push("focused".to_owned());
            }
            out.push(AccessNode {
                depth,
                role: format!("{role:?}"),
                name,
                value,
                states,
            });
        }
        for child in node.children().iter().rev() {
            stack.push((*child, child_depth));
        }
    }
    out
}
