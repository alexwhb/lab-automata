//! Loom · 1D (SCR-2) — mockup `docs/design/mockup/Loom.dc.html`.
//!
//! **Placeholder** from the app shell (wave 1): page frame (bench · rule aside), header and
//! tab-row actions, keyboard hooks. Wave 2 replaces the body.

use super::Screen;
use crate::ctx::{Command, Ctx};
use crate::layout::{self, Column};
use crate::widgets::tabs::RowSlot;
use crate::widgets::{Icon, counter, ghost, prim};
use eframe::egui::Ui;

/// The page row: `main` · `aside` (gap 16).
pub const COLUMNS: [Column; 2] = [
    Column::main("Loom"),
    Column::readout("Rule and presets").gap(16.0),
];

/// The Loom's state.
#[derive(Debug, Default)]
pub struct State {
    /// Weave / Pause.
    pub running: bool,
    /// Rows woven.
    pub row: u64,
    started: bool,
}

impl Screen for State {
    fn enter(&mut self, ctx: &mut Ctx) {
        if !self.started {
            self.started = true;
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
            self.running = !self.running;
        }
        if ghost("New thread").icon(Icon::Restart).show(ui).clicked() {
            self.row = 0;
        }
        counter(ui, &format!("row {:04}", self.row));
        if ghost("Share").show(ui).clicked() {
            ctx.toast("Sharing arrives with the Loom bench");
        }
    }

    fn tab_actions(&mut self, _ctx: &mut Ctx, slot: &mut RowSlot) {
        slot.hint("Each row is woven from the one above.");
    }

    fn command(&mut self, cmd: Command, _ctx: &mut Ctx) -> bool {
        match cmd {
            Command::RunPause => {
                self.running = !self.running;
                true
            }
            Command::Step => false,
        }
    }

    fn ui(&mut self, ctx: &mut Ctx, ui: &mut Ui) {
        layout::columns(ui, ctx, &COLUMNS, |ui, ctx, i| match i {
            0 => layout::bench(ui, ctx, self, |_, _, stage| {
                super::placeholder_stage(stage, "The loom arrives with the Loom bench.");
            }),
            _ => {
                super::placeholder_section(ui, "Rule", "Rule number, −1 / +1, Surprise me");
                super::placeholder_section(ui, "Famous threads", "30 · 90 · 110 · 184 · 54 · 150");
                super::placeholder_section(ui, "First row", "Single cell or random");
            }
        });
    }

    fn describe(&self) -> String {
        format!("Loom placeholder, row {}", self.row)
    }
}
