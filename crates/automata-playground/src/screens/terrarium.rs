//! Terrarium · breed (SCR-3) — mockup `docs/design/mockup/Terrarium.dc.html`.
//!
//! **Placeholder** from the app shell (wave 1): page frame (jars · breeding aside), header and
//! tab-row actions, keyboard hooks. Wave 2 replaces the body.

use super::Screen;
use crate::ctx::{Command, Ctx};
use crate::layout::{self, Column};
use crate::widgets::tabs::RowSlot;
use crate::widgets::{Icon, counter, ghost, prim};
use eframe::egui::Ui;

/// The page row: `main` · `aside` (gap 16).
pub const COLUMNS: [Column; 2] = [Column::main("Jars"), Column::readout("Breeding").gap(16.0)];

/// The Terrarium's state.
#[derive(Debug)]
pub struct State {
    /// Run / Pause.
    pub running: bool,
    /// The brood number (1 before any breeding).
    pub brood: u32,
    started: bool,
}

impl Default for State {
    fn default() -> Self {
        Self {
            running: false,
            brood: 1,
            started: false,
        }
    }
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
            prim("Run").icon(Icon::Play)
        };
        if run.show(ui).clicked() {
            self.running = !self.running;
        }
        let _ = ghost("Fresh soup").show(ui);
        counter(ui, &format!("brood {}", self.brood));
        if ghost("Share").show(ui).clicked() {
            ctx.toast("Sharing arrives with the Terrarium bench");
        }
    }

    fn tab_actions(&mut self, _ctx: &mut Ctx, slot: &mut RowSlot) {
        slot.hint("Keep the jars you like, then breed.");
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
                super::placeholder_stage(stage, "The jars arrive with the Terrarium bench.");
            }),
            _ => {
                super::placeholder_section(ui, "Breeding", "Children mix their parents’ rules");
                super::placeholder_section(ui, "Lineage", "Nothing bred yet.");
                super::placeholder_section(
                    ui,
                    "Use a favourite",
                    "Plant in the Tray · Analyse in Rule lab",
                );
            }
        });
    }

    fn describe(&self) -> String {
        format!("Terrarium placeholder, brood {}", self.brood)
    }
}
