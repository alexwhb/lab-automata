//! Tray · 2D (SCR-1) — mockup `docs/design/mockup/Tray.dc.html`.
//!
//! **Placeholder** from the app shell (wave 1): the mockup's page frame (rules aside · bench
//! with the tab row and `.graph` stage · readouts aside), the header and tab-row actions and
//! the keyboard hooks, so navigation and layout are testable. Wave 2 replaces the body with
//! the real tray on `ap-engine` + `ap-bench`.

use super::Screen;
use crate::ctx::{Command, Ctx};
use crate::layout::{self, Column};
use crate::widgets::tabs::RowSlot;
use crate::widgets::{Icon, SegGroup, SegItem, SegStyle, counter, ghost, prim};
use eframe::egui::Ui;

/// The page row: `aside` 248 (padding 14 12, gap 14) · `main` · readouts (gap 18).
pub const COLUMNS: [Column; 3] = [
    Column::aside("Cartridges and stamps", 248.0)
        .pad(14.0, 12.0)
        .border_right(),
    Column::main("Bench"),
    Column::readout("Rule and readouts").gap(18.0),
];

/// The drawing tools.
pub const TOOLS: [SegItem<'static>; 3] = [
    SegItem::new("Draw"),
    SegItem::new("Erase"),
    SegItem::new("Stamp"),
];

/// The Tray's state.
#[derive(Debug, Default)]
pub struct State {
    /// Run / Pause.
    pub running: bool,
    /// Generations stepped.
    pub generation: u64,
    /// Selected tool (index into [`TOOLS`]).
    pub tool: usize,
    started: bool,
}

impl State {
    fn step(&mut self) {
        self.running = false;
        self.generation += 1;
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
        if ghost("Step").icon(Icon::Step).show(ui).clicked() {
            self.step();
        }
        if ghost("Restart").icon(Icon::Restart).show(ui).clicked() {
            self.generation = 0;
        }
        counter(ui, &format!("gen {:04}", self.generation));
        if ghost("Share").show(ui).clicked() {
            ctx.toast("Sharing arrives with the Tray bench");
        }
    }

    fn tab_actions(&mut self, _ctx: &mut Ctx, slot: &mut RowSlot) {
        slot.add(|ui| {
            if let Some(i) = SegGroup::new("Tool", &TOOLS, Some(self.tool))
                .style(SegStyle::Tool)
                .show(ui)
                .clicked
            {
                self.tool = i;
            }
        });
        slot.add(|ui| ghost("Random soup").icon(Icon::Dice).height(30.0).show(ui));
        slot.add(|ui| ghost("Clear").height(30.0).show(ui));
    }

    fn command(&mut self, cmd: Command, _ctx: &mut Ctx) -> bool {
        match cmd {
            Command::RunPause => self.running = !self.running,
            Command::Step => self.step(),
        }
        true
    }

    fn ui(&mut self, ctx: &mut Ctx, ui: &mut Ui) {
        layout::columns(ui, ctx, &COLUMNS, |ui, ctx, i| match i {
            0 => {
                super::placeholder_section(ui, "Rules", "Six rule cartridges");
                super::placeholder_section(ui, "Stamps", "Eight stamps, click to place");
            }
            1 => layout::bench(ui, ctx, self, |_, _, stage| {
                super::placeholder_stage(stage, "The Petri tray arrives with the Tray bench.");
            }),
            _ => {
                super::placeholder_section(ui, "Rule", "Born with / Survives with");
                super::placeholder_section(ui, "Speed", "1–30 gen/s");
                super::placeholder_section(ui, "Population", "last 120 gens");
            }
        });
    }

    fn describe(&self) -> String {
        format!(
            "Tray placeholder, generation {}, {}",
            self.generation,
            if self.running { "running" } else { "paused" }
        )
    }
}
