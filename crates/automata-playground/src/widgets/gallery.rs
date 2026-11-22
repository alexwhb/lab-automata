//! A gallery of every widget in its states, on the `.paper` ground. The widget tests click
//! through it and snapshot it; it's also the quickest way to eyeball a change.

use super::*;
use crate::layout::Column;
use ap_theme as t;
use eframe::egui::{self, Align2, Rect, Ui, vec2};

/// The gallery's interactive state.
#[derive(Debug, Clone)]
pub struct Gallery {
    /// Clicks per button label.
    pub clicks: std::collections::BTreeMap<String, u32>,
    /// Selected tool.
    pub tool: usize,
    /// Selected `.mini` (fill group).
    pub start: usize,
    /// Selected family.
    pub family: usize,
    /// `.tog` states.
    pub trails: bool,
    /// Second `.tog`.
    pub flat: bool,
    /// `.keep`.
    pub kept: bool,
    /// Born-with bits.
    pub born: u32,
    /// Rule lab bits.
    pub lab_bits: u32,
    /// Neighbourhood.
    pub nb: u32,
    /// The highlighted cartridge.
    pub cart: usize,
    /// The highlighted stamp.
    pub stamp: usize,
    /// The highlighted preset.
    pub preset: usize,
    /// Slider value.
    pub speed: f32,
}

impl Default for Gallery {
    fn default() -> Self {
        let moore: u32 = [
            (-1, -1),
            (0, -1),
            (1, -1),
            (-1, 0),
            (1, 0),
            (-1, 1),
            (0, 1),
            (1, 1),
        ]
        .iter()
        .map(|&(x, y)| NbGrid::bit(x, y))
        .sum();
        Self {
            clicks: Default::default(),
            tool: 0,
            start: 0,
            family: 0,
            trails: true,
            flat: false,
            kept: false,
            born: 1 << 3,
            lab_bits: (1 << 2) | (1 << 3),
            nb: moore,
            cart: 0,
            stamp: 0,
            preset: 0,
            speed: 10.0,
        }
    }
}

/// The glider, for previews.
pub const GLIDER: [(i32, i32); 5] = [(1, 0), (2, 1), (0, 2), (1, 2), (2, 2)];
/// A lightweight spaceship.
pub const LWSS: [(i32, i32); 9] = [
    (1, 0),
    (4, 0),
    (0, 1),
    (0, 2),
    (4, 2),
    (0, 3),
    (1, 3),
    (2, 3),
    (3, 3),
];

impl Gallery {
    fn click(&mut self, label: &str) {
        *self.clicks.entry(label.to_string()).or_default() += 1;
    }

    /// Draws the gallery into `ui` (meant for a 1440 × 900 window).
    pub fn ui(&mut self, ui: &mut Ui) {
        let full = ui.max_rect();
        lab_ui::paint::paint_grid(ui.painter(), full, full.min, &t::PAPER);
        let cols = [
            Column::aside("Buttons", 360.0).flex(1.0, 1.0, 360.0),
            Column::aside("Lists", 300.0).flex(1.0, 1.0, 300.0),
            Column::aside("Readouts", 360.0).flex(1.0, 1.0, 360.0),
            Column::aside("Icons", 300.0).flex(1.0, 1.0, 300.0),
        ];
        let lines = crate::layout::resolve(full.width(), &cols);
        for p in lines.iter().flatten() {
            let rect = Rect::from_min_size(full.min + vec2(p.x, 0.0), vec2(p.width, full.height()))
                .shrink(16.0);
            let mut col = ui.new_child(
                egui::UiBuilder::new()
                    .id_salt(("gallery", p.index))
                    .max_rect(rect)
                    .layout(egui::Layout::top_down(egui::Align::Min)),
            );
            col.spacing_mut().item_spacing = vec2(10.0, 12.0);
            match p.index {
                0 => self.buttons(&mut col),
                1 => self.lists(&mut col),
                2 => self.readouts(&mut col),
                _ => self.icons(&mut col),
            }
        }
    }

    fn buttons(&mut self, ui: &mut Ui) {
        lbl(ui, "Buttons");
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            if prim("Pause").icon(Icon::Pause).show(ui).clicked() {
                self.click("Pause");
            }
            if ghost("Step").icon(Icon::Step).show(ui).clicked() {
                self.click("Step");
            }
            if ghost("Restart").icon(Icon::Restart).show(ui).clicked() {
                self.click("Restart");
            }
            counter(ui, "gen 0030");
        });
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            let _ = ghost("Disabled").enabled(false).show(ui);
            if ghost("Random soup")
                .icon(Icon::Dice)
                .height(30.0)
                .show(ui)
                .clicked()
            {
                self.click("Random soup");
            }
        });
        if prim("Breed brood 2")
            .icon(Icon::Breed)
            .big()
            .show(ui)
            .clicked()
        {
            self.click("Breed brood 2");
        }
        if ghost("Plant in the Tray")
            .full_width()
            .link()
            .show(ui)
            .clicked()
        {
            self.click("Plant in the Tray");
        }
        lbl(ui, "Segmented");
        let tools = [
            SegItem::new("Draw"),
            SegItem::new("Erase"),
            SegItem::new("Stamp"),
        ];
        ui.horizontal(|ui| {
            if let Some(i) = SegGroup::new("Tool", &tools, Some(self.tool))
                .style(SegStyle::Tool)
                .show(ui)
                .clicked
            {
                self.tool = i;
            }
        });
        let starts = [SegItem::new("Single cell"), SegItem::new("Random")];
        if let Some(i) = SegGroup::new("First row", &starts, Some(self.start))
            .style(SegStyle::Mini { fill: true })
            .show(ui)
            .clicked
        {
            self.start = i;
        }
        let fams = [
            SegItem::new("Life-like"),
            SegItem::new("Generations").enabled(false),
            SegItem::new("Larger than Life").enabled(false),
            SegItem::new("Lenia").enabled(false),
        ];
        ui.scope(|ui| {
            ui.set_max_width(268.0);
            if let Some(i) = SegGroup::new("Rule family", &fams, Some(self.family))
                .show(ui)
                .clicked
            {
                self.family = i;
            }
        });
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            if Toggle::new("Ghost trails", self.trails).show(ui).clicked() {
                self.trails = !self.trails;
            }
            if Toggle::new("Top-down view", self.flat).show(ui).clicked() {
                self.flat = !self.flat;
            }
            if Keep::new(self.kept)
                .accessible_name("Keep jar 1")
                .show(ui)
                .clicked()
            {
                self.kept = !self.kept;
            }
        });
        lbl(ui, "Bits");
        if let Some(n) = BitRow::new("Born with", 9, self.born).show(ui) {
            self.born ^= 1 << n;
        }
        ui.scope(|ui| {
            ui.set_max_width(240.0);
            if let Some(n) = BitRow::new("Survives with", 13, self.lab_bits)
                .lab()
                .show(ui)
            {
                self.lab_bits ^= 1 << n;
            }
        });
        ui.horizontal(|ui| {
            if let Some((x, y)) = NbGrid::new(self.nb).show(ui) {
                self.nb ^= NbGrid::bit(x, y);
            }
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 4.0;
                let _ = MiniButton::new("Moore r1", true).show(ui);
                let _ = MiniButton::new("von Neumann", false).width(70.0).show(ui);
            });
        });
    }

    fn lists(&mut self, ui: &mut Ui) {
        lbl(ui, "Rules");
        let carts = [
            ("Life", "The classic", "B3/S23"),
            ("HighLife", "Has a replicator", "B36/S23"),
            ("Day & Night", "Light–dark symmetric", "B3678/S34678"),
        ];
        ui.scope(|ui| {
            ui.spacing_mut().item_spacing.y = 6.0;
            for (i, (n, note, code)) in carts.iter().enumerate() {
                if Cartridge::new(n, note, code, t::CART_BODIES[i * 2 % 6])
                    .on(self.cart == i)
                    .show(ui)
                    .clicked()
                {
                    self.cart = i;
                }
            }
        });
        lbl(ui, "Stamps");
        lab_ui::layout::grid_columns(ui, 2, lab_ui::layout::Gap::same(8.0), 2, |ui, i| {
            let (name, cells): (&str, &[(i32, i32)]) = if i == 0 {
                ("Glider", &GLIDER)
            } else {
                ("Spaceship", &LWSS)
            };
            if StampTile::new(name, cells)
                .on(self.stamp == i)
                .show(ui)
                .clicked()
            {
                self.stamp = i;
            }
        });
        lbl(ui, "Famous threads");
        ui.scope(|ui| {
            ui.spacing_mut().item_spacing.y = 6.0;
            for (i, (n, name, note)) in [
                ("30", "Chaos", "Random-looking"),
                ("90", "Sierpiński", "Fractal triangles"),
            ]
            .iter()
            .enumerate()
            {
                if Preset::new(n, name, note)
                    .on(self.preset == i)
                    .show(ui)
                    .clicked()
                {
                    self.preset = i;
                }
            }
        });
    }

    fn readouts(&mut self, ui: &mut Ui) {
        lbl(ui, "Readouts");
        let text = format!("{} gen/s", self.speed as i32);
        let _ = Slider::new("Speed", &mut self.speed, 1.0..=30.0)
            .step(1.0)
            .value_text(text)
            .show(ui);
        let hist: Vec<f32> = (0..60)
            .map(|i| 90.0 + 30.0 * ((i as f32) * 0.7).sin())
            .collect();
        Plot::sparkline(&hist, 120).show(ui);
        lab_ui::layout::grid_columns(ui, 2, lab_ui::layout::Gap::same(8.0), 4, |ui, i| {
            let (k, v, c) = [
                ("Generation", "30", t::TEXT),
                ("Alive", "90", t::INK),
                ("Born", "27", t::NEWBORN.face),
                ("Died", "57", t::TEXT_MUTED),
            ][i];
            StatTile::new(k, v).color(c).show(ui);
        });
        Callout::new("Churning", "Births and deaths are balanced.").show(ui);
        let pops: Vec<f32> = (0..=400)
            .map(|i| 40.0 + 180.0 * (-(i as f32 - 150.0).powi(2) / 4000.0).exp())
            .collect();
        Plot::series(&pops)
            .stable_from(Some(390))
            .playhead(Some(120))
            .show(ui);
        verdict(ui, "Settles into a period-2 cycle at gen 388.");
        Card::panel().fill(t::INSET).show(ui, |ui| {
            CodeBlock::new(
                "* automata netlist · glider-garden.rule\n.rule   life_like  B3/S23\n.end",
            )
            .show(ui);
        });
    }

    fn icons(&mut self, ui: &mut Ui) {
        lbl(ui, "Icons");
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(10.0, 10.0);
            for icon in Icon::ALL {
                let (rect, _) = ui.allocate_exact_size(vec2(32.0, 32.0), egui::Sense::hover());
                ui.painter().rect_filled(rect, 6, t::CARD);
                let s = icon.css_size();
                icon.paint_with(
                    ui.painter(),
                    Rect::from_center_size(rect.center(), s),
                    t::TEXT_DIM,
                    (icon == Icon::Heart).then_some(t::ACCENT),
                );
            }
        });
        lbl_colored(ui, "Cell glyphs", t::INK);
        ui.horizontal(|ui| {
            for (i, g) in CellGlyph::FAMILY.iter().enumerate() {
                g.show(ui, 27.0, &format!("glyph {i}"));
            }
            CellGlyph::MOORE.show(ui, 40.0, "Moore neighbourhood");
        });
        let (rect, _) =
            ui.allocate_exact_size(vec2(ui.available_width(), 40.0), egui::Sense::hover());
        Chip::new("gen 0030 · ").part("90 alive", t::INK).show_at(
            ui,
            rect.left_top(),
            Align2::LEFT_TOP,
        );
    }
}
