//! Automata Playground — the egui app: the header (brand, mode switcher, app switcher, the
//! screen's actions), the bench tab row, the widget kit drawn to the mockup, and the five
//! screens (Tray, Loom, Terrarium, Rule lab, Series).
//!
//! Screens are specified by `docs/design/mockup/*.dc.html` ([`Route::mockup`]). How a screen
//! plugs in (state, `ui`, header / tab-row actions, keyboard commands, carry-over, toasts) is
//! documented on [`screens::Screen`] and in `docs/ARCHITECTURE.md`.

pub mod app;
pub mod control;
pub mod ctx;
pub mod launcher;
pub mod layout;
pub mod router;
pub mod screens;
pub mod smoke;
pub mod widgets;

pub use app::{AppOptions, AutomataApp};
pub use ctx::{Carry, Command, Ctx, Ticker};
pub use launcher::{AppId, Launch, Launcher};
pub use router::{Mode, Route};
pub use screens::Screen;

/// The app's name (window title, brand).
pub const APP_NAME: &str = "Automata Playground";
/// The macOS bundle id.
pub const APP_ID: &str = "dev.labseries.AutomataPlayground";

/// When the process started (set first thing in `main`; the smoke run's `first_frame_ms`).
pub static STARTED: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();

/// This checkout's crate folder, except in bundle builds (`AUTOMATA_BUNDLE=1`, set by
/// `scripts/bundle-macos.sh`) so the release binary carries no paths of the build machine.
pub const MANIFEST_DIR: Option<&str> = match option_env!("AUTOMATA_BUNDLE") {
    Some(_) => None,
    None => Some(env!("CARGO_MANIFEST_DIR")),
};

/// Window options: 1440 × 900 (the mockup's board), at least 390 × 600, wgpu, the window's
/// size and position persisted.
pub fn native_options() -> eframe::NativeOptions {
    eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title(APP_NAME)
            .with_app_id(APP_ID)
            .with_inner_size([1440.0, 900.0])
            .with_min_inner_size([390.0, 600.0]),
        persist_window: true,
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    }
}

/// Runs the app (with the MCP bridge on its default socket).
pub fn run() -> eframe::Result<()> {
    run_with(AppOptions::default())
}

/// Runs the app with `opts`.
pub fn run_with(opts: AppOptions) -> eframe::Result<()> {
    eframe::run_native(
        APP_NAME,
        native_options(),
        Box::new(|cc| Ok(Box::new(AutomataApp::new(cc, opts)))),
    )
}
