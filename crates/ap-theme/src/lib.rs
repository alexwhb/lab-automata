//! `ap-theme` — Automata Playground's visual tokens.
//!
//! The lab series shares one look (labkit's [`lab_theme`]: fonts, the egui style builder,
//! contrast helpers). Automata Playground, like Breadbox, uses the neutral greys
//! (`#0A0A0A` paper, `#242424` lines) and adds **one magenta ramp**, a **cell-age scale** and
//! cell-glyph icons (mockup `docs/design/mockup/Series.dc.html`, "Same family, one new accent").
//!
//! Every value names its CSS source in `docs/design/mockup/*.dc.html`. Screens never hard-code
//! a colour: they use the constants here (or [`lab_theme::pal`] for the generic roles).
//!
//! ```ignore
//! ap_theme::install(&cc.egui_ctx); // fonts + egui style + the palette below
//! ```

use egui::Color32;
pub use lab_theme::{self, Grid, font, hex, mix, rgba};

/// Installs the series fonts and the egui style built from [`PALETTE`].
pub fn install(ctx: &egui::Context) {
    lab_theme::install(ctx, PALETTE);
}

/// CSS `shade()` from the Tray mockup: each RGB channel × `k`, rounded (cartridge sides use
/// `k = 0.45`).
pub fn shade(c: Color32, k: f32) -> Color32 {
    let [r, g, b, a] = c.to_srgba_unmultiplied();
    let f = |v: u8| (v as f32 * k).round().clamp(0.0, 255.0) as u8;
    Color32::from_rgba_unmultiplied(f(r), f(g), f(b), a)
}

// ============================================================================================
// Grounds (graph paper)

/// `.paper` — the window ground: `#0A0A0A`, 60 px lines at 4 % white, 12 px at 1.6 %.
pub const PAPER: Grid = Grid {
    base: hex(0x0A0A0A),
    major: rgba(255, 255, 255, 0.04),
    major_px: 60.0,
    minor: rgba(255, 255, 255, 0.016),
    minor_px: 12.0,
};
/// `.graph` — the bench stage behind the tray / loom / jars: `#0B0C0E`, 120 px at 5 %, 24 px at 2 %.
pub const GRAPH: Grid = Grid {
    base: hex(0x0B0C0E),
    major: rgba(255, 255, 255, 0.05),
    major_px: 120.0,
    minor: rgba(255, 255, 255, 0.02),
    minor_px: 24.0,
};
/// `.floor` — the tilted floor plane: `#121316`, 120 px at 9 %, 24 px at 3.5 %
/// (drawn with `perspective(900px) rotateX(62deg)`, origin top centre).
pub const FLOOR: Grid = Grid {
    base: hex(0x121316),
    major: rgba(255, 255, 255, 0.09),
    major_px: 120.0,
    minor: rgba(255, 255, 255, 0.035),
    minor_px: 24.0,
};
/// `.well` — the Rule lab's top-down grid view: `#060606` with 12 px lines at 6 %.
pub const WELL: Grid = Grid {
    base: hex(0x060606),
    major: rgba(255, 255, 255, 0.06),
    major_px: 12.0,
    minor: rgba(255, 255, 255, 0.06),
    minor_px: 12.0,
};

// ============================================================================================
// Neutrals (shared with Breadbox)

/// Header and tab-row fill `rgba(12,12,12,0.8)`.
pub const BAR: Color32 = rgba(12, 12, 12, 0.8);
/// Side panels (`aside`) `rgba(17,17,17,0.84)`.
pub const SIDE: Color32 = rgba(17, 17, 17, 0.84);
/// Cards, stat tiles, presets `#111111`.
pub const CARD: Color32 = hex(0x111111);
/// Wells behind segmented groups, code blocks, the mode / app-switcher wells
/// `#060606` (`rgba(6,6,6,0.88)` for the header wells).
pub const INSET: Color32 = hex(0x060606);
/// Header wells (mode switcher, app switcher, tool group) `rgba(6,6,6,0.88)`.
pub const INSET_BAR: Color32 = rgba(6, 6, 6, 0.88);
/// `.cart` / `.stamp` fill `rgba(18,18,18,0.92)`.
pub const TILE: Color32 = rgba(18, 18, 18, 0.92);
/// `.ghost` button fill `#171717`.
pub const GHOST: Color32 = hex(0x171717);
/// `.ghost:hover` `#1F1F1F`.
pub const GHOST_HOVER: Color32 = hex(0x1F1F1F);
/// `.bit`, `.nb`, `.tog`, `.keep` fill `#141414`.
pub const CONTROL: Color32 = hex(0x141414);
/// `.sw:hover` (app switcher) `#1C1C1C`.
pub const SW_HOVER: Color32 = hex(0x1C1C1C);
/// Segmented "on" fill (`.seg-on`, `.mini-on`, `.sw-on`) `#2A2A2A`.
pub const SEG_ON: Color32 = hex(0x2A2A2A);
/// Panel separators, tiles, card borders `#242424`.
pub const BORDER: Color32 = hex(0x242424);
/// Button / chip / bit borders `#2E2E2E`.
pub const BORDER_STRONG: Color32 = hex(0x2E2E2E);
/// Primary text `#EAEAEA`.
pub const TEXT: Color32 = hex(0xEAEAEA);
/// Emphasis text (selected segment, active tab) `#F5F5F5`.
pub const TEXT_STRONG: Color32 = hex(0xF5F5F5);
/// Secondary labels, ghost-button text `#D9D9D9`.
pub const TEXT_DIM: Color32 = hex(0xD9D9D9);
/// Captions, section eyebrows (`.lbl`), inactive tabs `#A3A3A3`.
pub const TEXT_MUTED: Color32 = hex(0xA3A3A3);
/// Axis numbers, lineage brood labels, "Nothing bred yet." `#858585`.
pub const TEXT_FAINT: Color32 = hex(0x858585);
/// Disabled controls (`.mini-off`) `#5E5E5E` — disabled text is exempt from 4.5:1.
pub const TEXT_DISABLED: Color32 = hex(0x5E5E5E);
/// The `/` between brand and document name `#4A4A4A` (decorative).
pub const SLASH: Color32 = hex(0x4A4A4A);
/// Links and info values `#4CC3F0` (hover `#8AD8F5`).
pub const LINK: Color32 = hex(0x4CC3F0);
pub const LINK_HOVER: Color32 = hex(0x8AD8F5);

// ============================================================================================
// The magenta ramp (Automata's one accent)

/// Accent `#F06BD0`: Run, selection, active tab underline, sliders, live cells.
pub const ACCENT: Color32 = hex(0xF06BD0);
/// Ink `#F9B3E8`: text on tinted fills, rule codes, big numbers.
pub const INK: Color32 = hex(0xF9B3E8);
/// Tint `#22101D`: primary button / selected tile fill.
pub const TINT: Color32 = hex(0x22101D);
/// Edge `#6B2A5C`: primary button / hovered tile border.
pub const EDGE: Color32 = hex(0x6B2A5C);
/// `.prim:hover` `#2E1427`.
pub const TINT_HOVER: Color32 = hex(0x2E1427);
/// `.cart:hover` / `.stamp:hover` fill `#1A1018`.
pub const TILE_HOVER: Color32 = hex(0x1A1018);
/// "What is happening" callout and lineage chips `#1A0E17`.
pub const CALLOUT: Color32 = hex(0x1A0E17);
/// `.bit-on` 3 px drop `#8A2F76` (same as the alive cell's side).
pub const BIT_ON_DROP: Color32 = hex(0x8A2F76);
/// Text on accent fills (`.bit-on`) `#1A0A16`.
pub const ON_ACCENT: Color32 = hex(0x1A0A16);
/// Neighbourhood editor: neighbour on `#6B2A5C` fill + accent border; this cell = accent.
pub const NB_ON: Color32 = EDGE;

// ============================================================================================
// Info (cyan) — analysis values, stable band, verdict box

/// Rule lab verdict box border `#1D4A5C`.
pub const INFO_BORDER: Color32 = hex(0x1D4A5C);
/// Rule lab verdict box fill `#0C161B`.
pub const INFO_BG: Color32 = hex(0x0C161B);
/// Population plot "stable" band `rgba(76,195,240,.08)`.
pub const STABLE_BAND: Color32 = rgba(76, 195, 240, 0.08);
/// Plot fill under the population line `rgba(240,107,208,.14)`.
pub const PLOT_FILL: Color32 = rgba(240, 107, 208, 0.14);
/// Plot guide lines `rgba(255,255,255,.05)`.
pub const PLOT_GUIDE: Color32 = rgba(255, 255, 255, 0.05);
/// Plot background `#0B0C0E`.
pub const PLOT_BG: Color32 = hex(0x0B0C0E);
/// Plot playhead (Rule lab `head`, `stroke="#4CC3F0" stroke-width="1.5"`) = [`LINK`].
pub const PLAYHEAD: Color32 = LINK;

// ============================================================================================
// Inputs and glyphs (added by the app shell, wave 1)

/// `input[type=range]` with `accent-color: #F06BD0` as Chrome draws it in the mockup
/// (measured from `screenshots/Tray-1440.png`): an 8 px track, radius 4, unfilled part
/// `#3B3B3B` with a 1 px `#858585` border, the filled part and the 16 px thumb in [`ACCENT`].
pub const RANGE_TRACK: Color32 = hex(0x3B3B3B);
/// Border of the unfilled range track `#858585`.
pub const RANGE_BORDER: Color32 = hex(0x858585);
/// Series "Icon family · cell glyphs": empty cells of the 3 × 3 glyphs `fill="#3A3A3A"`.
pub const GLYPH_OFF: Color32 = hex(0x3A3A3A);
/// Tray "8 neighbours · edges wrap" glyph: the neighbour squares `fill="#2A1A26"`.
pub const NB_GLYPH_OFF: Color32 = hex(0x2A1A26);
/// Series identity swatch frame `background: #171717` (= [`GHOST`]) and the logo's body.
pub const LOGO_BODY: Color32 = GHOST;
/// Lineage code chips (Terrarium) `border: 1px solid #6B2A5C; background: #1A0E17` =
/// [`EDGE`] on [`CALLOUT`]; the dashed "Nothing bred yet." box border `#2E2E2E`.
pub const DASHED: Color32 = BORDER_STRONG;

// ============================================================================================
// Cell-age scale (automata only) — "reads in greyscale too; live cells lift 6 px off the tray"

/// One step of the cell-age scale: the top face, the 6 px side drop and an optional glow.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CellLook {
    pub face: Color32,
    pub side: Option<Color32>,
    pub glow: Option<Color32>,
    /// Glow blur radius in px (CSS `0 0 <r>px`).
    pub glow_px: f32,
}

/// `.cn` newborn (age 1): `#FFE6F7`, side `#B34E9C`, glow `0 0 16px rgba(255,170,232,.75)`.
pub const NEWBORN: CellLook = CellLook {
    face: hex(0xFFE6F7),
    side: Some(hex(0xB34E9C)),
    glow: Some(rgba(255, 170, 232, 0.75)),
    glow_px: 16.0,
};
/// `.ca` alive (age 2–10): `#F06BD0`, side `#8A2F76`, glow `0 0 10px rgba(240,107,208,.42)`.
pub const ALIVE: CellLook = CellLook {
    face: hex(0xF06BD0),
    side: Some(hex(0x8A2F76)),
    glow: Some(rgba(240, 107, 208, 0.42)),
    glow_px: 10.0,
};
/// `.co` elder (age > 10): `#B24C9C`, side `#561C49`, no glow.
pub const ELDER: CellLook = CellLook {
    face: hex(0xB24C9C),
    side: Some(hex(0x561C49)),
    glow: None,
    glow_px: 0.0,
};
/// Ghost trail after death, fading 3 → 2 → 1: `.t3 #3B1734`, `.t2 #2C1428`, `.t1 #22131F`.
pub const TRAIL: [Color32; 3] = [hex(0x22131F), hex(0x2C1428), hex(0x3B1734)];
/// Empty cell `#1B1C21` with an inset 1 px top highlight `rgba(255,255,255,.05)`.
pub const EMPTY: Color32 = hex(0x1B1C21);
pub const EMPTY_HIGHLIGHT: Color32 = rgba(255, 255, 255, 0.05);
/// Hovered empty cell `#2A2B33`; hovered live cell `#FFFFFF`.
pub const EMPTY_HOVER: Color32 = hex(0x2A2B33);
/// Rule lab "died this generation" ghost `#2C1428`.
pub const GHOST_CELL: Color32 = hex(0x2C1428);
/// How many px a live cell lifts off the tray (`translateY(-6px)` + `0 6px 0` side).
pub const LIFT_PX: f32 = 6.0;
/// The age at which a live cell becomes an elder (`a > 10`).
pub const ELDER_AGE: u16 = 11;

/// The look of a live cell of `age` (1 = newborn).
pub fn cell_look(age: u16) -> CellLook {
    match age {
        0 | 1 => NEWBORN,
        a if a >= ELDER_AGE => ELDER,
        _ => ALIVE,
    }
}

// ============================================================================================
// Bench objects

/// The Petri tray (Tray screen): body `#15171A`, 1.5 px edge `#2A2D33`, 22 px slab `#0B0C0E`,
/// radius 20.
pub const TRAY_BODY: Color32 = hex(0x15171A);
pub const TRAY_EDGE: Color32 = hex(0x2A2D33);
pub const TRAY_SLAB: Color32 = hex(0x0B0C0E);
/// Engraved tray labels (decorative; the same facts are in accessible text elsewhere):
/// "PETRI TRAY · 40 × 26 · TORUS" `#5A5E66`, the rule code `#8A4E7C`.
pub const TRAY_LABEL: Color32 = hex(0x5A5E66);
pub const TRAY_CODE: Color32 = hex(0x8A4E7C);
/// Pinned measurement chip `.chip`: `rgba(11,12,14,.9)`, border `#2A2D33`, text `#ECEAE6`.
pub const CHIP: Color32 = rgba(11, 12, 14, 0.9);
pub const CHIP_BORDER: Color32 = hex(0x2A2D33);
pub const CHIP_TEXT: Color32 = hex(0xECEAE6);

/// Loom: fabric board `#0F1013` (border `#2A2D33`), print head gradient `#26282E → #17191D`
/// (border `#34373E`), spools `#101114` / `#3A3D44` / spokes `#4A4D55`, head label `#8A8D94`.
pub const LOOM_BOARD: Color32 = hex(0x0F1013);
pub const LOOM_HEAD_TOP: Color32 = hex(0x26282E);
pub const LOOM_HEAD_BOTTOM: Color32 = hex(0x17191D);
pub const LOOM_HEAD_BORDER: Color32 = hex(0x34373E);
pub const LOOM_SPOOL: Color32 = hex(0x101114);
pub const LOOM_SPOOL_RIM: Color32 = hex(0x3A3D44);
pub const LOOM_SPOKE: Color32 = hex(0x4A4D55);
pub const LOOM_HEAD_LABEL: Color32 = hex(0x8A8D94);
/// Loom rule keys `.key`: `#16181C`, border `#2A2D33`, 7 px drop `#0B0C0E`; pressed: tint,
/// accent border, 3 px drop `#561C49`, glow `0 0 18px rgba(240,107,208,.25)`.
pub const KEY: Color32 = hex(0x16181C);
pub const KEY_DROP_ON: Color32 = hex(0x561C49);
pub const KEY_GLOW: Color32 = rgba(240, 107, 208, 0.25);
/// Arrow on the keys `#858585`; empty pattern squares `#0B0C0E`.
pub const KEY_ARROW: Color32 = hex(0x858585);

/// Terrarium jar: body `#121318`, 2 px edge `#2E3138`, inner ring `#17181D`, 16 px slab
/// `#0B0C0E`; card `.jarcard` border `#1E1F24` fill `rgba(14,15,18,.6)`, kept
/// `rgba(34,16,29,.45)` with the edge colour.
pub const JAR_BODY: Color32 = hex(0x121318);
pub const JAR_EDGE: Color32 = hex(0x2E3138);
pub const JAR_RING: Color32 = hex(0x17181D);
pub const JAR_CARD_BORDER: Color32 = hex(0x1E1F24);
pub const JAR_CARD: Color32 = rgba(14, 15, 18, 0.6);
pub const JAR_CARD_KEPT: Color32 = rgba(34, 16, 29, 0.45);
/// Liveliness meter: track `#1B1C21`; quiet (< 8 %) `#5E5E5E`; frantic (> 80 %) `#FF8E7A`.
pub const METER_TRACK: Color32 = hex(0x1B1C21);
pub const METER_QUIET: Color32 = hex(0x5E5E5E);
pub const METER_FRANTIC: Color32 = hex(0xFF8E7A);

/// Rule cartridge body colours (Tray "Rules" list), in the mockup's order: Life, HighLife,
/// Seeds, Day & Night, Maze, Coral. The side is [`shade`]`(body, 0.45)`.
pub const CART_BODIES: [Color32; 6] = [
    hex(0xF06BD0),
    hex(0xF3DC6B),
    hex(0xFF8E7A),
    hex(0xB79CFF),
    hex(0x4BE3A0),
    hex(0x6CCBF5),
];
/// Terrarium jar inks, by jar index (Terrarium mockup `inks`: same six hues, different order).
pub const JAR_INKS: [Color32; 6] = [
    hex(0xF06BD0),
    hex(0xF3DC6B),
    hex(0xB79CFF),
    hex(0x4BE3A0),
    hex(0xFF8E7A),
    hex(0x6CCBF5),
];

// ============================================================================================
// App identities (Series screen, app switcher)

/// One app's four-colour ramp.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Identity {
    pub name: &'static str,
    pub domain: &'static str,
    pub accent: Color32,
    pub ink: Color32,
    pub tint: Color32,
    pub edge: Color32,
}

pub const BREADBOX: Identity = Identity {
    name: "Breadbox",
    domain: "Circuits · SPICE",
    accent: hex(0xF2A33A),
    ink: hex(0xF7C77A),
    tint: hex(0x1D1810),
    edge: hex(0x6B4A16),
};
pub const CHEM: Identity = Identity {
    name: "Chem Playground",
    domain: "Reactions · kinetics",
    accent: hex(0x86A2FF),
    ink: hex(0xB3C4FF),
    tint: hex(0x161C2E),
    edge: hex(0x3B4C7A),
};
pub const BIO: Identity = Identity {
    name: "Bio Playground",
    domain: "Gene circuits · populations",
    accent: hex(0x4BE3A0),
    ink: hex(0xA6F5CF),
    tint: hex(0x0E2419),
    edge: hex(0x1E5C42),
};
pub const AUTOMATA: Identity = Identity {
    name: "Automata Playground",
    domain: "Cellular automata · rules",
    accent: ACCENT,
    ink: INK,
    tint: TINT,
    edge: EDGE,
};
/// The series in switcher order.
pub const SERIES: [Identity; 4] = [BREADBOX, CHEM, BIO, AUTOMATA];

// ============================================================================================
// Sizes (points; 1 pt = 1 CSS px at 1440 × 900)

pub mod size {
    /// Header min-height 56.
    pub const HEADER: f32 = 56.0;
    /// Bench tab row min-height 44 (tabs are 44 tall).
    pub const TAB_ROW: f32 = 44.0;
    /// `.ghost` / `.prim` / `.seg` height 32 (`.big` 44).
    pub const BUTTON: f32 = 32.0;
    pub const BUTTON_BIG: f32 = 44.0;
    /// `.mini` 28, `.tog` 28, `.keep` 30.
    pub const MINI: f32 = 28.0;
    pub const KEEP: f32 = 30.0;
    /// App switcher squares 32 × 32.
    pub const SWITCH: f32 = 32.0;
    /// Tray B/S bits 28 × 30; Rule lab bits 26 × 26 (min-width); neighbourhood cells 30 × 30.
    pub const BIT_W: f32 = 28.0;
    pub const BIT_H: f32 = 30.0;
    pub const LAB_BIT: f32 = 26.0;
    pub const NB_CELL: f32 = 30.0;
    /// Smallest pointer target (WCAG 2.5.5). Controls drawn smaller than this extend their hit
    /// area into the surrounding padding where it exists.
    pub const MIN_TARGET: f32 = 44.0;
    /// Side panels: Tray rules 248; right readouts 320–360; Rule lab spec 300, script 340–380.
    pub const RULES_W: f32 = 248.0;
    pub const READOUT_MIN: f32 = 320.0;
    pub const READOUT_MAX: f32 = 360.0;
    pub const SPEC_W: f32 = 300.0;
    pub const SCRIPT_MIN: f32 = 340.0;
    pub const SCRIPT_MAX: f32 = 380.0;
    /// Below this width the panels stack (the mockup's `flex-wrap: wrap` rows).
    pub const NARROW: f32 = 720.0;
}

pub mod radius {
    /// Buttons, segmented pills, chips 6; `.mini` 5; wells 7–8; cards / tiles 10; jar cards 14;
    /// tray 20; toggles 999.
    pub const BUTTON: u8 = 6;
    pub const MINI: u8 = 5;
    pub const WELL: u8 = 8;
    pub const SEG_WELL: u8 = 7;
    pub const CARD: u8 = 10;
    pub const JAR_CARD: u8 = 14;
    pub const TRAY: u8 = 20;
    pub const PILL: u8 = 255;
    pub const CELL: u8 = 4;
}

// ============================================================================================
// The palette installed into egui (generic roles for labkit helpers and egui widgets)

/// [`lab_theme::LAB_DARK`] with Automata's greys and the magenta ramp in the accent roles
/// (`amber*` fields are the series' "action / selection" accent).
pub const PALETTE: lab_theme::Palette = lab_theme::Palette {
    ground: PAPER.base,
    ground_grid: PAPER,
    stage_grid: GRAPH,
    floor_grid: FLOOR,
    bar: BAR,
    panel: SIDE,
    panel_soft: SIDE,
    surface: GHOST,
    tile: CARD,
    tag: CONTROL,
    seg_on: SEG_ON,
    chip: CHIP,
    mode_on: SEG_ON,
    border: BORDER,
    border_hover: BORDER_STRONG,
    axis: TEXT_FAINT,
    text: TEXT,
    text_secondary: TEXT_DIM,
    text_mode: TEXT_MUTED,
    text_muted: TEXT_MUTED,
    text_code: TEXT_DIM,
    text_faint: TEXT_FAINT,
    on_light: ON_ACCENT,
    amber: ACCENT,
    amber_hover: INK,
    amber_soft: INK,
    amber_bg: TINT,
    amber_border: EDGE,
    amber_text: INK,
    cyan: LINK,
    ..lab_theme::LAB_DARK
};

#[cfg(test)]
mod tests {
    use super::*;
    use lab_theme::contrast::{over, ratio};

    #[test]
    fn values_match_the_mockup_css() {
        assert_eq!(ACCENT, Color32::from_rgb(0xF0, 0x6B, 0xD0));
        assert_eq!(SIDE.to_srgba_unmultiplied(), [17, 17, 17, 214]);
        assert_eq!(PAPER.major.a(), 10);
        // shade() of the Life cartridge body (#F06BD0 × .45) is #6C305E
        assert_eq!(
            shade(CART_BODIES[0], 0.45),
            Color32::from_rgb(0x6C, 0x30, 0x5E)
        );
    }

    #[test]
    fn cell_age_scale() {
        assert_eq!(cell_look(1), NEWBORN);
        assert_eq!(cell_look(2), ALIVE);
        assert_eq!(cell_look(10), ALIVE);
        assert_eq!(cell_look(11), ELDER);
    }

    /// Body text reaches 4.5:1 on every surface it sits on (WCAG AA).
    #[test]
    fn text_contrast_is_at_least_4_5() {
        let paper = PAPER.base;
        let surfaces = [
            ("paper", paper),
            ("bar", over(BAR, paper)),
            ("side", over(SIDE, paper)),
            ("card", CARD),
            ("inset", INSET),
            ("ghost", GHOST),
            ("control", CONTROL),
            ("tile", over(TILE, paper)),
            ("graph", GRAPH.base),
            ("chip", over(CHIP, GRAPH.base)),
            ("plot", PLOT_BG),
        ];
        let texts = [
            ("text", TEXT),
            ("text_dim", TEXT_DIM),
            ("text_muted", TEXT_MUTED),
            ("text_faint", TEXT_FAINT),
            ("accent", ACCENT),
            ("ink", INK),
            ("link", LINK),
        ];
        for (sn, s) in surfaces {
            for (tn, t) in texts {
                let r = ratio(t, s);
                assert!(r >= 4.5, "{tn} on {sn}: {r:.2}:1");
            }
        }
        let pairs = [
            ("ink on tint", INK, TINT),
            ("ink on callout", INK, CALLOUT),
            ("text_dim on callout", TEXT_DIM, CALLOUT),
            ("text_strong on seg_on", TEXT_STRONG, SEG_ON),
            ("on_accent on accent", ON_ACCENT, ACCENT),
            ("text_dim on info_bg", TEXT_DIM, INFO_BG),
            ("ink on tile_hover", INK, TILE_HOVER),
        ];
        for (n, t, s) in pairs {
            let r = ratio(t, s);
            assert!(r >= 4.5, "{n}: {r:.2}:1");
        }
        for id in SERIES {
            let r = ratio(id.ink, id.tint);
            assert!(r >= 4.5, "{} ink on tint: {r:.2}:1", id.name);
        }
    }

    /// Non-text graphics (live cells, the tray edge against the stage) reach 3:1.
    #[test]
    fn live_cells_reach_3_to_1() {
        for look in [NEWBORN, ALIVE, ELDER] {
            let r = ratio(look.face, EMPTY);
            assert!(r >= 3.0, "{:?} on empty: {r:.2}:1", look.face);
        }
    }
}
