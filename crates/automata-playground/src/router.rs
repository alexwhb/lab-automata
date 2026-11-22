//! Navigation model (NAV-1/2): two modes and five routes.
//!
//! Playground holds the three benches (Tray, Loom, Terrarium) under a tab row; Advanced is the
//! Rule lab; the Series screen is reached from the app switcher (NAV-3) and belongs to no
//! mode. Each route knows its mockup (`docs/design/mockup/<file>`), the mockup's board size
//! and the fixed panel height its columns use when they stack.

use serde::{Deserialize, Serialize};

/// The header's mode switcher.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Mode {
    /// Tray · Loom · Terrarium (the benches).
    Playground,
    /// The Rule lab.
    Advanced,
}

impl Mode {
    /// In switcher order.
    pub const ALL: [Mode; 2] = [Mode::Playground, Mode::Advanced];

    /// The switcher label (and accessible name).
    pub fn label(self) -> &'static str {
        match self {
            Mode::Playground => "Playground",
            Mode::Advanced => "Advanced",
        }
    }
}

/// A screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Route {
    /// SCR-1 Playground › Tray · 2D.
    Tray,
    /// SCR-2 Playground › Loom · 1D.
    Loom,
    /// SCR-3 Playground › Terrarium · breed.
    Terrarium,
    /// SCR-4 Advanced › Rule lab.
    RuleLab,
    /// SCR-5 the series screen (app switcher).
    Series,
}

impl Route {
    /// Every route, in shortcut order (⌘1 … ⌘4, then Series).
    pub const ALL: [Route; 5] = [
        Route::Tray,
        Route::Loom,
        Route::Terrarium,
        Route::RuleLab,
        Route::Series,
    ];

    /// The Playground benches, in tab order.
    pub const BENCHES: [Route; 3] = [Route::Tray, Route::Loom, Route::Terrarium];

    /// The mode this route belongs to (`None` for the Series screen, which highlights no mode).
    pub fn mode(self) -> Option<Mode> {
        match self {
            Route::Tray | Route::Loom | Route::Terrarium => Some(Mode::Playground),
            Route::RuleLab => Some(Mode::Advanced),
            Route::Series => None,
        }
    }

    /// True for the three Playground benches (they have the bench tab row).
    pub fn is_bench(self) -> bool {
        self.mode() == Some(Mode::Playground)
    }

    /// The bench tab label (`None` off the Playground).
    pub fn tab_label(self) -> Option<&'static str> {
        match self {
            Route::Tray => Some("Tray · 2D"),
            Route::Loom => Some("Loom · 1D"),
            Route::Terrarium => Some("Terrarium · breed"),
            Route::RuleLab | Route::Series => None,
        }
    }

    /// A human title (the tab label for benches).
    pub fn title(self) -> &'static str {
        match self {
            Route::RuleLab => "Rule lab",
            Route::Series => "Series",
            r => r.tab_label().unwrap_or_default(),
        }
    }

    /// The document name shown after the brand in the header (NAV-1).
    pub fn document(self) -> &'static str {
        match self {
            Route::Tray => "glider-garden",
            Route::Loom => "loom",
            Route::Terrarium => "terrarium",
            Route::RuleLab => "glider-garden.rule",
            Route::Series => "series",
        }
    }

    /// The mockup file in `docs/design/mockup/`.
    pub fn mockup(self) -> &'static str {
        match self {
            Route::Tray => "Tray.dc.html",
            Route::Loom => "Loom.dc.html",
            Route::Terrarium => "Terrarium.dc.html",
            Route::RuleLab => "RuleLab.dc.html",
            Route::Series => "Series.dc.html",
        }
    }

    /// The mockup's board size in points (`$preview` width × height).
    pub fn board_size(self) -> (f32, f32) {
        match self {
            Route::Tray | Route::Loom | Route::Terrarium => (1440.0, 900.0),
            Route::RuleLab => (1440.0, 960.0),
            Route::Series => (1440.0, 640.0),
        }
    }

    /// The fixed height of the screen's columns (`height: 844px` / `904px` in the mockup):
    /// used when the columns stack at narrow widths. Side by side they fill the window under
    /// the header instead. The Series screen has no columns (0).
    pub fn panel_height(self) -> f32 {
        match self {
            Route::Tray | Route::Loom | Route::Terrarium => 844.0,
            Route::RuleLab => 904.0,
            Route::Series => 0.0,
        }
    }

    /// The ⌘-digit shortcut (NAV-5): ⌘1–⌘3 benches, ⌘4 Rule lab.
    pub fn shortcut_digit(self) -> Option<u8> {
        match self {
            Route::Tray => Some(1),
            Route::Loom => Some(2),
            Route::Terrarium => Some(3),
            Route::RuleLab => Some(4),
            Route::Series => None,
        }
    }

    /// A stable lowercase id (persistence, MCP `navigate`, test filters).
    pub fn id(self) -> &'static str {
        match self {
            Route::Tray => "tray",
            Route::Loom => "loom",
            Route::Terrarium => "terrarium",
            Route::RuleLab => "rule-lab",
            Route::Series => "series",
        }
    }

    /// Parses [`Route::id`] (also accepts the mockup name, any case, and `rule_lab`).
    pub fn from_id(id: &str) -> Option<Route> {
        let id = id.trim().to_ascii_lowercase().replace('_', "-");
        Route::ALL.into_iter().find(|r| {
            r.id() == id
                || r.mockup().trim_end_matches(".dc.html").to_ascii_lowercase()
                    == id.replace('-', "")
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modes_and_benches() {
        assert_eq!(
            Route::BENCHES.map(Route::mode),
            [Some(Mode::Playground); 3],
            "benches are Playground"
        );
        assert_eq!(Route::RuleLab.mode(), Some(Mode::Advanced));
        assert_eq!(Route::Series.mode(), None);
        assert!(Route::Tray.is_bench() && !Route::RuleLab.is_bench());
        assert_eq!(Mode::ALL.map(Mode::label), ["Playground", "Advanced"]);
    }

    #[test]
    fn labels_match_the_mockup() {
        assert_eq!(
            Route::BENCHES.map(|r| r.tab_label().unwrap()),
            ["Tray · 2D", "Loom · 1D", "Terrarium · breed"]
        );
        assert_eq!(Route::RuleLab.tab_label(), None);
        assert_eq!(
            Route::ALL.map(Route::document),
            [
                "glider-garden",
                "loom",
                "terrarium",
                "glider-garden.rule",
                "series"
            ]
        );
        assert_eq!(Route::RuleLab.title(), "Rule lab");
    }

    #[test]
    fn boards_and_panels() {
        assert_eq!(Route::Tray.board_size(), (1440.0, 900.0));
        assert_eq!(Route::RuleLab.board_size(), (1440.0, 960.0));
        assert_eq!(Route::Series.board_size(), (1440.0, 640.0));
        // board height − 56 px header (the mockup's `height: 844px` / `904px` columns)
        for r in [Route::Tray, Route::Loom, Route::Terrarium, Route::RuleLab] {
            assert_eq!(r.panel_height(), r.board_size().1 - 56.0, "{r:?}");
        }
    }

    #[test]
    fn mockups_exist() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/design");
        for r in Route::ALL {
            assert!(dir.join("mockup").join(r.mockup()).is_file(), "{r:?}");
            let stem = r.mockup().trim_end_matches(".dc.html");
            assert!(
                dir.join(format!("screenshots/{stem}-1440.png")).is_file(),
                "{r:?}"
            );
        }
    }

    #[test]
    fn ids_round_trip() {
        for r in Route::ALL {
            assert_eq!(Route::from_id(r.id()), Some(r));
        }
        assert_eq!(Route::from_id("RuleLab"), Some(Route::RuleLab));
        assert_eq!(Route::from_id("rule_lab"), Some(Route::RuleLab));
        assert_eq!(Route::from_id("nope"), None);
    }

    #[test]
    fn shortcuts() {
        assert_eq!(
            Route::ALL.map(Route::shortcut_digit),
            [Some(1), Some(2), Some(3), Some(4), None]
        );
    }
}
