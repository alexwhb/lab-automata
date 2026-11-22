//! The app frame: the `.paper` ground, the header, the active screen and toasts; global
//! shortcuts (NAV-5); navigation with carry-over (NAV-4) and the Playground's last-bench
//! memory (NAV-2).

use crate::ctx::{Carry, Command, Ctx, NavRequest, Outbox, Services};
use crate::launcher::{AppId, Launcher};
use crate::router::Route;
use crate::screens::Screens;
use crate::widgets::{Toasts, header::header};
use eframe::egui::{self, Key, Modifiers, Ui};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

/// Start-up options. [`AppOptions::default`] is the real app; [`AppOptions::test`] is
/// deterministic.
#[derive(Debug, Clone)]
pub struct AppOptions {
    /// The screen to open (unless a persisted one is restored).
    pub start: Route,
    /// Seed for every random thing (soups, Surprise me, breeding).
    pub rng_seed: u64,
    /// Screens start running when first shown (TRAY-7 "Runs on start").
    pub autoplay: bool,
    /// Where files are written when there's no save dialog.
    pub out_dir: PathBuf,
    /// Use the native save dialog (off in tests).
    pub save_dialog: bool,
    /// Opens sibling apps (NAV-3).
    pub launcher: Launcher,
    /// Restore the last route and bench from eframe storage.
    pub load_persisted: bool,
    /// Where the live MCP bridge listens (`None`: no bridge; see `control.rs`). The real app
    /// listens on [`crate::control::default_socket_path`]; tests pass their own temp socket.
    pub mcp_socket: Option<PathBuf>,
}

impl Default for AppOptions {
    fn default() -> Self {
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x5EED);
        let out_dir = std::env::var_os("HOME")
            .map(|h| PathBuf::from(h).join("Documents").join(crate::APP_NAME))
            .unwrap_or_else(|| std::env::temp_dir().join("automata-playground"));
        Self {
            start: Route::Tray,
            rng_seed: seed,
            autoplay: true,
            out_dir,
            save_dialog: true,
            launcher: Launcher::system(),
            load_persisted: true,
            mcp_socket: crate::control::default_socket_path(),
        }
    }
}

impl AppOptions {
    /// The fixed seed tests use.
    pub const TEST_SEED: u64 = 7;

    /// Deterministic options for tests: seed 7, paused, no persistence, files into a fresh
    /// temp folder, a recording launcher with nothing installed.
    pub fn test() -> Self {
        static N: AtomicU64 = AtomicU64::new(0);
        let n = N.fetch_add(1, Ordering::Relaxed);
        Self {
            start: Route::Tray,
            rng_seed: Self::TEST_SEED,
            autoplay: false,
            out_dir: std::env::temp_dir().join(format!(
                "automata-playground-test-{}-{n}",
                std::process::id()
            )),
            save_dialog: false,
            launcher: Launcher::recording(&[]),
            load_persisted: false,
            mcp_socket: None,
        }
    }

    /// [`AppOptions::test`] starting on `route`.
    pub fn at(route: Route) -> Self {
        Self {
            start: route,
            ..Self::test()
        }
    }
}

const STORAGE_KEY: &str = "automata-playground";

#[derive(serde::Serialize, serde::Deserialize)]
struct Persisted {
    route: Route,
    last_bench: Route,
}

/// The app.
pub struct AutomataApp {
    route: Route,
    last_bench: Route,
    entered: Option<Route>,
    screens: Screens,
    toasts: Toasts,
    services: Services,
    outbox: Outbox,
    /// A carry-over (NAV-4) and the screen it is for, delivered at the start of the next frame.
    pending_carry: Option<(Route, Carry)>,
    content_height: f32,
    header_height: f32,
    /// The options it was started with (the MCP bridge reads `mcp_socket`).
    pub options: AppOptions,
    /// The live MCP bridge (owned by `control.rs`).
    pub(crate) control: crate::control::Control,
}

impl AutomataApp {
    /// Builds the app from eframe's creation context (installs fonts + style).
    pub fn new(cc: &eframe::CreationContext<'_>, opts: AppOptions) -> Self {
        ap_theme::install(&cc.egui_ctx);
        let mut app = Self::with_options(opts);
        if app.options.load_persisted
            && let Some(storage) = cc.storage
            && let Some(p) = eframe::get_value::<Persisted>(storage, STORAGE_KEY)
        {
            app.route = p.route;
            if p.last_bench.is_bench() {
                app.last_bench = p.last_bench;
            }
        }
        app
    }

    /// Builds the app without an eframe context (call [`ap_theme::install`] yourself).
    pub fn with_options(opts: AppOptions) -> Self {
        let services = Services {
            launcher: opts.launcher.clone(),
            out_dir: opts.out_dir.clone(),
            save_dialog: opts.save_dialog,
            rng_seed: opts.rng_seed,
            autoplay: opts.autoplay,
        };
        let route = opts.start;
        Self {
            route,
            last_bench: if route.is_bench() { route } else { Route::Tray },
            entered: None,
            screens: Screens::default(),
            toasts: Toasts::default(),
            services,
            outbox: Outbox::default(),
            pending_carry: None,
            content_height: 0.0,
            header_height: 0.0,
            control: crate::control::Control::start(opts.mcp_socket.as_deref()),
            options: opts,
        }
    }

    /// The current screen.
    pub fn route(&self) -> Route {
        self.route
    }

    /// The Playground bench "Playground" opens (the last one used; Tray at first).
    pub fn last_bench(&self) -> Route {
        self.last_bench
    }

    /// Go to `route` (what the header does).
    pub fn navigate(&mut self, route: Route) {
        self.apply(NavRequest {
            route,
            carry: None,
            focus_app: None,
        });
    }

    /// Go to `route` handing it a carry-over (NAV-4).
    pub fn navigate_with(&mut self, route: Route, carry: Carry) {
        self.apply(NavRequest {
            route,
            carry: Some(carry),
            focus_app: None,
        });
    }

    /// Open the Series screen with `app` highlighted (what the app switcher does).
    pub fn open_series(&mut self, app: AppId) {
        self.apply(NavRequest {
            route: Route::Series,
            carry: None,
            focus_app: Some(app),
        });
    }

    fn apply(&mut self, req: NavRequest) {
        if req.route.is_bench() {
            self.last_bench = req.route;
        }
        if req.route == Route::Series {
            self.screens.series.highlight = req.focus_app;
        }
        if req.route != self.route {
            self.entered = None;
        }
        self.route = req.route;
        // delivered after `enter`, at the start of the next frame; a navigation without one
        // that comes first (the MCP bridge's, between frames) doesn't drop it
        if let Some(carry) = req.carry {
            self.pending_carry = Some((req.route, carry));
        }
    }

    /// All screens' state.
    pub fn screens(&self) -> &Screens {
        &self.screens
    }

    /// All screens' state, mutably (tests, the MCP bridge).
    pub fn screens_mut(&mut self) -> &mut Screens {
        &mut self.screens
    }

    /// Toasts queued or showing.
    pub fn toasts(&self) -> &Toasts {
        &self.toasts
    }

    /// Queue a toast.
    pub fn toast(&mut self, text: impl Into<String>) {
        self.toasts.push(text);
    }

    /// The last text a screen copied to the clipboard.
    pub fn last_copied(&self) -> Option<&str> {
        self.outbox.last_copied.as_deref()
    }

    /// Files saved so far.
    pub fn saved_files(&self) -> &[PathBuf] {
        &self.outbox.saved
    }

    /// The services (launcher, output folder, seed).
    pub fn services(&self) -> &Services {
        &self.services
    }

    /// The page height as of the last frame: header + body content (screenshot tests size the
    /// window with it).
    pub fn content_height(&self) -> f32 {
        self.content_height
    }

    /// The header's height as of the last frame.
    pub fn header_height(&self) -> f32 {
        self.header_height
    }

    fn ctx<'a>(&'a mut self, egui: &egui::Context, body_height: f32) -> (Ctx<'a>, &'a mut Screens) {
        (
            Ctx {
                route: self.route,
                last_bench: self.last_bench,
                nav: None,
                toasts: &mut self.toasts,
                services: &self.services,
                outbox: &mut self.outbox,
                egui: egui.clone(),
                body_height,
            },
            &mut self.screens,
        )
    }

    /// Runs `route`'s [`Screen::enter`](crate::Screen::enter) now, whether or not it is
    /// showing: the MCP bridge calls it before driving a screen in the background, so the
    /// screen's first-visit set-up doesn't undo the change later. Screens' `enter` is safe to
    /// call again.
    pub(crate) fn enter_screen(&mut self, route: Route, egui_ctx: &egui::Context) {
        let body_h = (egui_ctx.content_rect().height() - self.header_height).max(0.0);
        let (mut ctx, screens) = self.ctx(egui_ctx, body_h);
        screens.get(route).enter(&mut ctx);
    }

    /// One frame of the whole app (also called directly by tests).
    pub fn frame(&mut self, ui: &mut Ui) {
        crate::control::poll(self, ui);
        let egui_ctx = ui.ctx().clone();
        // patterns opened from Finder / the command line (TRAY-9) arm the Tray's stamp
        crate::platform::deliver(self, &egui_ctx);
        let full = ui.max_rect();
        let body_h = (full.height() - self.header_height).max(0.0);
        let route = self.route;

        // ---- enter + carry-over, shortcuts
        let mut nav: Option<NavRequest> = None;
        {
            let entered = self.entered;
            let carry = self.pending_carry.take();
            let (mut ctx, screens) = self.ctx(&egui_ctx, body_h);
            // a carry whose screen was left before it arrived still reaches that screen
            let carry = match carry {
                Some((target, c)) if target != route => {
                    let other = screens.get(target);
                    other.enter(&mut ctx);
                    other.receive(c, &mut ctx);
                    None
                }
                carry => carry.map(|(_, c)| c),
            };
            let screen = screens.get(route);
            if entered != Some(route) {
                screen.enter(&mut ctx);
            }
            if let Some(c) = carry {
                screen.receive(c, &mut ctx);
            }
            if let Some(r) = shortcut_route(&egui_ctx) {
                ctx.go(r);
            }
            for cmd in commands(&egui_ctx) {
                screen.command(cmd, &mut ctx);
            }
            // once per frame: a discarded pass (multi-pass layout) re-runs with the same dt
            if egui_ctx.current_pass_index() == 0 {
                screen.tick(&mut ctx);
            }
            nav = nav.or(ctx.nav.take());
        }
        self.entered = Some(route);

        // ---- the page: ground, header, body (scrolls as a whole when columns stack)
        lab_ui::paint::paint_grid(ui.painter(), full, full.min, &ap_theme::PAPER);
        let mut header_h = self.header_height;
        let mut content_h: f32 = 0.0;
        {
            let (mut ctx, screens) = self.ctx(&egui_ctx, body_h);
            let screen = screens.get(route);
            let out = egui::ScrollArea::vertical()
                .id_salt(("page", route.id()))
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing = egui::Vec2::ZERO;
                    let top = ui.cursor().top();
                    let bar = header(ui, &mut ctx, |ui, ctx| screen.header_actions(ctx, ui));
                    header_h = bar.height();
                    screen.ui(&mut ctx, ui);
                    ui.cursor().top() - top
                });
            content_h = content_h.max(out.inner).max(out.content_size.y);
            nav = ctx.nav.take().or(nav);
        }
        if (header_h - self.header_height).abs() > 0.5 {
            // columns size themselves from the header height: redo the pass with the new one
            egui_ctx.request_discard("header height changed");
        }
        self.header_height = header_h;
        self.content_height = content_h;
        self.toasts.show(&egui_ctx);
        if let Some(req) = nav {
            self.apply(req);
            egui_ctx.request_repaint();
        }
    }
}

/// ⌘1–⌘4 (NAV-5).
fn shortcut_route(ctx: &egui::Context) -> Option<Route> {
    const KEYS: [(Key, u8); 4] = [
        (Key::Num1, 1),
        (Key::Num2, 2),
        (Key::Num3, 3),
        (Key::Num4, 4),
    ];
    ctx.input_mut(|i| {
        KEYS.iter().find_map(|(k, d)| {
            i.consume_key(Modifiers::COMMAND, *k)
                .then(|| {
                    Route::ALL
                        .into_iter()
                        .find(|r| r.shortcut_digit() == Some(*d))
                })
                .flatten()
        })
    })
}

/// Space (no widget focused) → Run / Pause; `S` (no text field focused) → Step.
fn commands(ctx: &egui::Context) -> Vec<Command> {
    let mut out = Vec::new();
    let nothing_focused = ctx.memory(|m| m.focused().is_none());
    let typing = ctx.text_edit_focused();
    ctx.input_mut(|i| {
        if nothing_focused && i.consume_key(Modifiers::NONE, Key::Space) {
            out.push(Command::RunPause);
        }
        if !typing && i.consume_key(Modifiers::NONE, Key::S) {
            out.push(Command::Step);
        }
    });
    out
}

impl eframe::App for AutomataApp {
    /// Runs even while the window is hidden; lets the MCP bridge ask for a frame (`control.rs`).
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        crate::control::logic(self, ctx);
    }

    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        self.frame(ui);
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(
            storage,
            STORAGE_KEY,
            &Persisted {
                route: self.route,
                last_bench: self.last_bench,
            },
        );
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        egui::Color32::to_normalized_gamma_f32(ap_theme::PAPER.base)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_options_are_deterministic() {
        let (a, b) = (AppOptions::test(), AppOptions::test());
        assert_eq!(a.rng_seed, AppOptions::TEST_SEED);
        assert!(!a.autoplay && !a.save_dialog && !a.load_persisted);
        assert_ne!(a.out_dir, b.out_dir, "each test app gets its own folder");
        assert_eq!(AppOptions::at(Route::Loom).start, Route::Loom);
        assert!(AppOptions::default().autoplay);
    }

    #[test]
    fn save_and_copy_go_through_the_outbox() {
        let mut app = AutomataApp::with_options(AppOptions::test());
        let egui_ctx = egui::Context::default();
        let path = {
            let (mut c, _) = app.ctx(&egui_ctx, 600.0);
            c.copy("x = 1, y = 1\no!");
            c.toast("Copied");
            c.save_text("glider.rle", "x = 3, y = 3\nbo$2bo$3o!")
                .expect("write")
                .expect("not cancelled")
        };
        assert_eq!(app.last_copied(), Some("x = 1, y = 1\no!"));
        assert_eq!(app.saved_files(), std::slice::from_ref(&path));
        assert!(path.starts_with(&app.services().out_dir));
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "x = 3, y = 3\nbo$2bo$3o!"
        );
        assert_eq!(app.toasts().items().len(), 1);
        let _ = std::fs::remove_dir_all(&app.services().out_dir);
    }

    #[test]
    fn navigation_rules() {
        let mut app = AutomataApp::with_options(AppOptions::at(Route::RuleLab));
        assert_eq!(app.last_bench(), Route::Tray, "Tray at first start");
        app.navigate(Route::Terrarium);
        app.navigate(Route::RuleLab);
        assert_eq!(app.last_bench(), Route::Terrarium);
        app.open_series(AppId::Chem);
        assert_eq!(app.route(), Route::Series);
        assert_eq!(app.screens().series.highlight, Some(AppId::Chem));
        app.navigate(Route::Series);
        assert_eq!(app.screens().series.highlight, None);
    }
}
