//! # ap-engine
//!
//! The Automata Playground engine, with no UI dependencies (SPEC ENG-1…8). Everything a screen
//! shows is computed here:
//!
//! | Module | For | Key items |
//! |---|---|---|
//! | [`rule`] | all screens | [`LifeRule`], [`Neighbourhood`], [`NeighbourhoodPreset`], [`CountSet`], [`Boundary`] |
//! | [`presets`] | Tray, Terrarium | [`presets::CARTRIDGES`], [`presets::rule_name`], [`presets::starting_garden`] |
//! | [`grid`] | Tray, jars | [`Grid`] (ages + ghost trails), [`Grid::step`] → [`StepStats`], [`Grid::place`] |
//! | [`pattern`] | Tray, Rule lab | [`Pattern`], [`pattern::library`], [`pattern::rle`], [`pattern::cells`], [`pattern::parse_any`] |
//! | [`elementary`] | Loom | [`Elementary`], [`Weave`], [`elementary::FAMOUS`] |
//! | [`analysis`] | Rule lab | [`RunSpec`], [`Seed`], [`run`] → [`Run`] → [`Report`] |
//! | [`mood`] | Tray | [`Mood::classify`], [`History`] |
//! | [`breed`] | Terrarium | [`Terrarium`], [`Jar`], [`breed::breed`], [`breed::child`], [`Lineage`] |
//! | [`script`] | Rule lab | [`RuleScript`] (`parse` / `to_text`), [`ScriptError`] |
//! | [`rng`] | soups, breeding | [`Rng`] (seeded SplitMix64) |
//!
//! Nothing panics on any input: parsers return errors with line numbers, editing methods ignore
//! out-of-range coordinates and counts, and sizes are clamped to documented limits.
//!
//! ```
//! use ap_engine::{run, LifeRule, RunSpec};
//!
//! let report = run(&RunSpec::default()).report();
//! assert_eq!(report.verdict, "Settles into a period-2 cycle at gen 388.");
//!
//! let highlife = LifeRule::parse("B36/S23").unwrap();
//! assert_eq!(ap_engine::presets::rule_name(&highlife), Some("HighLife"));
//! ```

pub mod analysis;
pub mod breed;
pub mod elementary;
pub mod grid;
pub mod mood;
pub mod pattern;
pub mod presets;
pub mod rng;
pub mod rule;
pub mod script;

pub use analysis::{CellChange, Frame, Repeat, Report, Run, RunSpec, Seed, SpecError, run};
pub use breed::{Brood, Jar, Lineage, LineageEntry, Liveliness, Terrarium};
pub use elementary::{Elementary, FamousThread, Weave, WeaveStart};
pub use grid::{Anchor, BBox, Grid, StepStats};
pub use mood::{History, Mood};
pub use pattern::{Pattern, PatternError, PatternFile};
pub use presets::Cartridge;
pub use rng::Rng;
pub use rule::{
    Boundary, CountSet, LifeRule, Neighbourhood, NeighbourhoodError, NeighbourhoodPreset,
    RuleParseError,
};
pub use script::{Probe, ProbeQuantity, RuleScript, ScriptError};
