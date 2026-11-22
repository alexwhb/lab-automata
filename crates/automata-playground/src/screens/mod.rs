//! The five screens. Each lives in its own module as a `State` (with `Default`) that
//! implements [`Screen`]; the app keeps them all alive in [`Screens`] so their state survives
//! navigation. See `docs/ARCHITECTURE.md` › "How a screen plugs in".

use crate::ctx::{Carry, Command, Ctx};
use crate::router::Route;
use crate::widgets::tabs::RowSlot;
use eframe::egui::Ui;

pub mod loom;
pub mod rule_lab;
pub mod series;
pub mod terrarium;
pub mod tray;

/// What a screen implements. Only [`Screen::ui`] is required.
///
/// Call order each frame for the active screen: [`Screen::enter`] (only when the route has
/// just become active, before anything else), [`Screen::command`] (for Space / `S`),
/// [`Screen::header_actions`] (inside the header), then [`Screen::ui`] (the body under the
/// header; benches call [`crate::layout::bench`], which calls [`Screen::tab_actions`]). After
/// a navigation with a carry-over, the target gets `enter` then [`Screen::receive`].
pub trait Screen {
    /// The body under the header. Lay it out with [`crate::layout::columns`].
    fn ui(&mut self, ctx: &mut Ctx, ui: &mut Ui);

    /// The header's right-hand actions (Run / Pause, Step, `gen 0000`, Share, …), drawn left
    /// to right with 8 px gaps; give each item a 32 px height so they centre.
    fn header_actions(&mut self, _ctx: &mut Ctx, _ui: &mut Ui) {}

    /// The bench tab row's right-hand items (Playground benches only): call `slot.add(|ui|
    /// …)` once per item (each may wrap onto its own line), or `slot.hint("…")`.
    fn tab_actions(&mut self, _ctx: &mut Ctx, _slot: &mut RowSlot) {}

    /// A keyboard command (NAV-5). Return `true` if handled.
    fn command(&mut self, _cmd: Command, _ctx: &mut Ctx) -> bool {
        false
    }

    /// The route just became active (also the very first frame): seed from
    /// [`Ctx::rng_seed`], start running if [`Ctx::autoplay`], …
    fn enter(&mut self, _ctx: &mut Ctx) {}

    /// A rule / pattern carried over from another screen (NAV-4).
    fn receive(&mut self, _carry: Carry, _ctx: &mut Ctx) {}

    /// A one-line description of what's on screen (A11Y-4 canvases, the MCP `describe` tool).
    fn describe(&self) -> String {
        String::new()
    }
}

/// Placeholder content (wave 1): a `.lbl` title over a muted note, as a section of an aside.
#[doc(hidden)]
pub fn placeholder_section(ui: &mut Ui, title: &str, note: &str) {
    crate::widgets::lbl(ui, title);
    crate::widgets::text::text_block(
        ui,
        note,
        lab_theme::font::body(12.0),
        ap_theme::TEXT_FAINT,
        None,
    );
}

/// Placeholder content (wave 1): a centred note on the `.graph` stage.
#[doc(hidden)]
pub fn placeholder_stage(ui: &mut Ui, note: &str) {
    let rect = ui.max_rect();
    crate::widgets::Chip::new(note).show_at(ui, rect.center(), eframe::egui::Align2::CENTER_CENTER);
}

/// Every screen's state.
#[derive(Default)]
pub struct Screens {
    /// SCR-1.
    pub tray: tray::State,
    /// SCR-2.
    pub loom: loom::State,
    /// SCR-3.
    pub terrarium: terrarium::State,
    /// SCR-4.
    pub rule_lab: rule_lab::State,
    /// SCR-5.
    pub series: series::State,
}

impl Screens {
    /// The screen for `route`.
    pub fn get(&mut self, route: Route) -> &mut dyn Screen {
        match route {
            Route::Tray => &mut self.tray,
            Route::Loom => &mut self.loom,
            Route::Terrarium => &mut self.terrarium,
            Route::RuleLab => &mut self.rule_lab,
            Route::Series => &mut self.series,
        }
    }

    /// Read-only access for `describe` and tests.
    pub fn get_ref(&self, route: Route) -> &dyn Screen {
        match route {
            Route::Tray => &self.tray,
            Route::Loom => &self.loom,
            Route::Terrarium => &self.terrarium,
            Route::RuleLab => &self.rule_lab,
            Route::Series => &self.series,
        }
    }
}
