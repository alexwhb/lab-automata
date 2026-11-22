//! Rule lab (SCR-4) — mockup `docs/design/mockup/RuleLab.dc.html`.
//!
//! **Placeholder** from the app shell (wave 1): page frame (spec aside · run · script and
//! analysis aside, 904 px panels), header actions, keyboard hooks. Wave 2 replaces the body.

use super::Screen;
use crate::ctx::{Command, Ctx};
use crate::layout::{self, Column};
use crate::widgets::ghost;
use eframe::egui::Ui;

/// The page row: spec 300 · run (padding 16, gap 14, scrolls) · script 1 0 340 max 380.
pub const COLUMNS: [Column; 3] = [
    Column::aside("Rule spec", 300.0).border_right(),
    Column::main("Run").pad(16.0, 16.0).gap(14.0).scroll(true),
    Column::readout("Script and analysis")
        .flex(1.0, 0.0, 340.0)
        .max(380.0),
];

/// The Rule lab's state.
#[derive(Debug, Default)]
pub struct State {
    /// Play / Pause of the scrubber.
    pub playing: bool,
}

impl Screen for State {
    fn header_actions(&mut self, ctx: &mut Ctx, ui: &mut Ui) {
        if ghost("Open in Playground").show(ui).clicked() {
            ctx.toast("Open in Playground arrives with the Rule lab");
        }
        if ghost("Export").show(ui).clicked() {
            ctx.toast("Export arrives with the Rule lab");
        }
    }

    fn command(&mut self, cmd: Command, _ctx: &mut Ctx) -> bool {
        match cmd {
            Command::RunPause => {
                self.playing = !self.playing;
                true
            }
            Command::Step => false,
        }
    }

    fn ui(&mut self, ctx: &mut Ctx, ui: &mut Ui) {
        layout::columns(ui, ctx, &COLUMNS, |ui, _ctx, i| match i {
            0 => {
                super::placeholder_section(
                    ui,
                    "Rule spec",
                    "Family, neighbourhood, bits, boundary, seed",
                );
            }
            1 => {
                super::placeholder_section(ui, "Run", "Top-down grid, scrub, population plot");
            }
            _ => {
                super::placeholder_section(ui, "Rule script", "The editable netlist");
                super::placeholder_section(ui, "Analysis", "Peak, final, first repeat, period");
                super::placeholder_section(ui, "Pattern export · RLE", "The scrubbed generation");
            }
        });
    }

    fn describe(&self) -> String {
        "Rule lab placeholder".into()
    }
}
