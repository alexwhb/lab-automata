//! `ap-bench` — Automata Playground's 2.5D bench: drawing and geometry for the tray, the loom,
//! the terrarium jars and the Rule lab's top-down grid.
//!
//! The mockup builds its benches out of CSS 3-D transforms (`perspective(P) rotateX(θ)
//! scale(s)`). [`Projection`] reproduces those transforms exactly (validated against Chrome's
//! own layout, see `tests/chrome_geometry.rs`), including the inverse used for hit-testing
//! through the perspective. [`draw::Batch`] draws on a projection — anti-aliased fills, CSS
//! borders and box-shadows, lines, text whose glyphs lie on the tilted surface — into one egui
//! mesh per scene.
//!
//! The scenes take plain data (no dependency on `ap-engine`): the screens adapt engine state
//! into scene inputs.
//!
//! ```ignore
//! // in the Tray screen, every frame:
//! let rect = ui.available_rect_before_wrap();                 // the `.graph` stage
//! ap_bench::stage(ui.painter(), rect);
//! let cell = |c, r| match grid.age(c, r) {
//!     0 if trails => CellVisual::Trail(grid.trail(c, r)),
//!     0 => CellVisual::Empty,
//!     a => CellVisual::Live(a),
//! };
//! let mut scene = TrayScene::new(40, 26, &cell);
//! scene.tilt = state.tilt.value();                              // anim::Transition + ease
//! scene.zoom = scene.fit_zoom(rect.size());
//! scene.label_left = "PETRI TRAY · 40 × 26 · TORUS";
//! scene.label_right = &code;
//! let geo = scene.geometry(rect);
//! let resp = ui.allocate_rect(rect, egui::Sense::click_and_drag());
//! scene.hover = resp.hover_pos().and_then(|p| geo.cell_at(p));
//! if resp.dragged() { /* paint at geo.cell_near(pointer) */ }
//! scene.cursor = state.focus_cell;
//! scene.paint(ui.painter(), rect);
//! ```

pub mod a11y;
pub mod anim;
pub mod draw;
pub mod jar;
pub mod loom;
pub mod projection;
pub mod stage;
pub mod tokens;
pub mod topdown;
pub mod tray;

pub use anim::{Transition, ease, jar_bob, spool_angle};
pub use draw::Batch;
pub use jar::JarScene;
pub use loom::{KeyVisual, LoomGeometry, LoomScene, build_key, paint_key};
pub use projection::{Projection, Tilt, View};
pub use stage::{floor, stage};
pub use topdown::{TopCell, TopDownGeometry, TopDownGrid};
pub use tray::{CellVisual, Outline, TrayGeometry, TrayScene};
