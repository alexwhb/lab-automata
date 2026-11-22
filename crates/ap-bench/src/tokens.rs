//! Colours the bench draws with.
//!
//! Almost everything comes from [`ap_theme`]. The block marked **local tokens** below holds the
//! few bench-only values `ap-theme` doesn't define yet, each with its mockup CSS source; they
//! are candidates to move into `ap-theme` (listed in the ap-bench report / PROGRESS.md).

use ap_theme::{hex, rgba};
use egui::Color32;

pub use ap_theme::{
    ACCENT, ALIVE, CellLook, ELDER, ELDER_AGE, EMPTY, EMPTY_HIGHLIGHT, EMPTY_HOVER, FLOOR,
    GHOST_CELL, GRAPH, INK, JAR_BODY, JAR_EDGE, JAR_RING, KEY, KEY_ARROW, KEY_DROP_ON, KEY_GLOW,
    LIFT_PX, LINK, LOOM_BOARD, LOOM_HEAD_BORDER, LOOM_HEAD_BOTTOM, LOOM_HEAD_LABEL, LOOM_HEAD_TOP,
    LOOM_SPOKE, LOOM_SPOOL, LOOM_SPOOL_RIM, NEWBORN, TINT, TRAIL, TRAY_BODY, TRAY_CODE, TRAY_EDGE,
    TRAY_LABEL, TRAY_SLAB, WELL, cell_look,
};

// ============================================================================================
// local tokens — bench-only values not (yet) in ap-theme, with their mockup CSS source
// ============================================================================================

/// Tray / Loom: the band over the floor's far edge,
/// `linear-gradient(rgba(11,12,14,0), rgba(11,12,14,.8))` (bottom stop).
pub const FLOOR_BAND: Color32 = rgba(11, 12, 14, 0.8);
/// Tray: the contact shadow under the tray, `background: rgba(0,0,0,.7); filter: blur(16px)`.
pub const TRAY_SHADOW: Color32 = rgba(0, 0, 0, 0.7);
/// Terrarium: the shadow under each jar, `background: rgba(0,0,0,.75); filter: blur(10px)`.
pub const JAR_SHADOW: Color32 = rgba(0, 0, 0, 0.75);
/// Terrarium: the jar's 16 px slab, `box-shadow: 0 16px 0 #0B0C0E`.
pub const JAR_SLAB: Color32 = hex(0x0B0C0E);
/// Terrarium: the glint, `linear-gradient(120deg, rgba(255,255,255,.08), rgba(255,255,255,0))`.
pub const JAR_GLINT: Color32 = rgba(255, 255, 255, 0.08);
/// Loom: the 1 px ring around the board, `box-shadow: 0 0 0 1px #0B0C0E`.
pub const LOOM_BOARD_RING: Color32 = hex(0x0B0C0E);
/// Loom: the board's border, `border: 1px solid #2A2D33`.
pub const LOOM_BOARD_EDGE: Color32 = hex(0x2A2D33);
/// Loom: the fade over the far rows, `linear-gradient(#0B0C0E, rgba(11,12,14,0))` (top stop).
pub const LOOM_FADE: Color32 = hex(0x0B0C0E);
/// Loom: the print head's 12 px drop, `box-shadow: 0 12px 0 #0B0C0E, 0 12px 0 1px #2A2D33`.
pub const LOOM_HEAD_DROP: Color32 = hex(0x0B0C0E);
pub const LOOM_HEAD_DROP_EDGE: Color32 = hex(0x2A2D33);
/// Loom: the print head's cast shadow, `box-shadow: 0 30px 40px rgba(0,0,0,.6)`.
pub const LOOM_HEAD_SHADOW: Color32 = rgba(0, 0, 0, 0.6);
/// Loom: the progress track, `background: #0B0C0E`.
pub const LOOM_TRACK: Color32 = hex(0x0B0C0E);
/// Loom keys: border `#2A2D33`, 7 px drop `#0B0C0E` (`box-shadow: 0 7px 0 #0B0C0E,
/// 0 7px 0 1px #2A2D33`), hover border `#6B2A5C`.
pub const KEY_EDGE: Color32 = hex(0x2A2D33);
pub const KEY_DROP: Color32 = hex(0x0B0C0E);
pub const KEY_HOVER_EDGE: Color32 = ap_theme::EDGE;
/// Loom keys: pattern squares and output cell outline `stroke="#3A3D44"` / `border: 1px solid
/// #3A3D44`, empty squares `#0B0C0E`.
pub const KEY_PATTERN_STROKE: Color32 = hex(0x3A3D44);
pub const KEY_EMPTY: Color32 = hex(0x0B0C0E);
/// Loom keys: the lit output cell's glow, `box-shadow: 0 0 10px rgba(255,170,232,.8)`.
pub const KEY_OUT_GLOW: Color32 = rgba(255, 170, 232, 0.8);
/// Rule lab: the well's border, `border: 1px solid #242424`.
pub const WELL_BORDER: Color32 = ap_theme::BORDER;
/// The keyboard focus ring (`outline: 2px solid` accent, `outline-offset: 2px`).
pub const FOCUS: Color32 = ap_theme::ACCENT;
