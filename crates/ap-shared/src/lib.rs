//! UI-free vocabulary shared by the Automata Playground app and its MCP server
//! (`automata-mcp`, crate `ap-mcp`):
//!
//! - the app's identity ([`APP_NAME`], [`APP_ID`]) and where the live MCP bridge listens
//!   ([`SOCKET_ENV`], [`bridge_socket_path`]);
//! - the screens and modes as the bridge names them ([`SCREENS`], [`MODES`]; the app's
//!   `automata_playground::Route` is the source of truth and a test keeps the two equal);
//! - the Automata bridge methods and their payloads ([`proto`]), on top of the four every
//!   lab-series app answers (`lab_mcp::bridge::proto`);
//! - how a pattern or rule typed by a person or a model is read ([`input`]), so the app and the
//!   server accept exactly the same text.
//!
//! No egui here: `ap-mcp` uses it without pulling in a GUI toolkit.

#![warn(missing_docs)]

pub mod input;
pub mod proto;

use lab_mcp::bridge::proto::{ModeRef, ScreenRef};
use serde::Serialize;
use std::ffi::OsString;
use std::path::PathBuf;

/// The app's display name (matches `automata_playground::APP_NAME`).
pub const APP_NAME: &str = "Automata Playground";
/// The app's reverse-DNS id, also its macOS bundle id (matches `automata_playground::APP_ID`).
pub const APP_ID: &str = "dev.labseries.AutomataPlayground";

/// Overrides where the live bridge listens, for both the app and `automata-mcp`. Empty or
/// `off` disables the bridge.
pub const SOCKET_ENV: &str = "AUTOMATA_MCP_SOCKET";

/// How to start the app, for "not running" messages.
pub const HOW_TO_START: &str = "open Automata Playground (dist/Automata Playground.app, or \
`cargo run --release -p automata-playground` in the automata-playground checkout)";

/// The bridge socket: `$AUTOMATA_MCP_SOCKET` if set (`None` when it is empty or `off`), else
/// the per-user default (`~/Library/Application Support/dev.labseries.AutomataPlayground/mcp.sock`
/// on macOS, or a shorter fallback when that path is too long for a Unix socket).
pub fn bridge_socket_path() -> Option<PathBuf> {
    socket_from(std::env::var_os(SOCKET_ENV))
}

/// [`bridge_socket_path`] for a given value of [`SOCKET_ENV`] (`None`: unset).
pub fn socket_from(value: Option<OsString>) -> Option<PathBuf> {
    match value {
        Some(v) if v.is_empty() || v == "off" => None,
        Some(v) => Some(PathBuf::from(v)),
        None => Some(lab_mcp::bridge::default_socket_path(APP_ID)),
    }
}

/// A mode of the header's mode switcher.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ModeInfo {
    /// Stable id: `playground`, `advanced`.
    pub id: &'static str,
    /// The switcher label.
    pub label: &'static str,
    /// Where the mode link goes (a screen id).
    pub home: &'static str,
}

impl ModeInfo {
    /// The mode as the bridge reports it.
    pub fn mode_ref(&self) -> ModeRef {
        ModeRef {
            id: self.id.to_owned(),
            label: self.label.to_owned(),
        }
    }
}

/// Playground: the three benches (Tray, Loom, Terrarium).
pub const PLAYGROUND: ModeInfo = ModeInfo {
    id: "playground",
    label: "Playground",
    home: "tray",
};
/// Advanced: the Rule lab.
pub const ADVANCED: ModeInfo = ModeInfo {
    id: "advanced",
    label: "Advanced",
    home: "rule-lab",
};
/// Both modes, in switcher order.
pub const MODES: [ModeInfo; 2] = [PLAYGROUND, ADVANCED];

/// A mockup board size in points.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Board {
    /// Width.
    pub width: f32,
    /// Height.
    pub height: f32,
}

/// A screen as the bridge and `list_screens` name it.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct ScreenInfo {
    /// Stable id (`navigate` takes it): `tray`, `loom`, `terrarium`, `rule-lab`, `series`.
    pub id: &'static str,
    /// The tab label or title ("Tray · 2D", "Rule lab").
    pub label: &'static str,
    /// The mode it belongs to (`None`: the Series screen).
    pub mode: Option<ModeInfo>,
    /// The document name in the header (`glider-garden`).
    pub document: &'static str,
    /// The keyboard shortcut (`⌘1`…), if any.
    pub shortcut: Option<&'static str>,
    /// What is on it, for the model.
    pub about: &'static str,
    /// The mockup that specifies it (`docs/design/mockup/`).
    pub mockup: &'static str,
    /// The mockup board size at 1440 px.
    pub board: Board,
}

impl ScreenInfo {
    /// The screen as the bridge reports it.
    pub fn screen_ref(&self) -> ScreenRef {
        ScreenRef {
            id: self.id.to_owned(),
            label: self.label.to_owned(),
            mode: self.mode.map(|m| m.mode_ref()),
        }
    }
}

/// Every screen, in shortcut order (⌘1–⌘4, then Series).
pub const SCREENS: [ScreenInfo; 5] = [
    ScreenInfo {
        id: "tray",
        label: "Tray · 2D",
        mode: Some(PLAYGROUND),
        document: "glider-garden",
        shortcut: Some("⌘1"),
        about: "A 40 × 26 torus of Life-like cells: rule cartridges, stamps, draw / erase, \
                run / step, population, born / died and the 'what is happening' mood.",
        mockup: "Tray.dc.html",
        board: Board {
            width: 1440.0,
            height: 900.0,
        },
    },
    ScreenInfo {
        id: "loom",
        label: "Loom · 1D",
        mode: Some(PLAYGROUND),
        document: "loom",
        shortcut: Some("⌘2"),
        about: "An elementary (Wolfram) CA, 121 cells wide, woven row by row: rule keys, \
                famous threads, single-cell or random first row.",
        mockup: "Loom.dc.html",
        board: Board {
            width: 1440.0,
            height: 900.0,
        },
    },
    ScreenInfo {
        id: "terrarium",
        label: "Terrarium · breed",
        mode: Some(PLAYGROUND),
        document: "terrarium",
        shortcut: Some("⌘3"),
        about: "Six jars, each a 32 × 32 torus with its own Life-like rule: keep favourites, \
                breed a new brood from them (bit-flip mutations), lineage.",
        mockup: "Terrarium.dc.html",
        board: Board {
            width: 1440.0,
            height: 900.0,
        },
    },
    ScreenInfo {
        id: "rule-lab",
        label: "Rule lab",
        mode: Some(ADVANCED),
        document: "glider-garden.rule",
        shortcut: Some("⌘4"),
        about: "Rule spec (neighbourhood, born / survives, boundary, seed), a 400-generation \
                run with scrubber and population plot, the analysis (peak, final, first \
                repeat, period, verdict), the editable rule script and RLE export.",
        mockup: "RuleLab.dc.html",
        board: Board {
            width: 1440.0,
            height: 960.0,
        },
    },
    ScreenInfo {
        id: "series",
        label: "Series",
        mode: None,
        document: "series",
        shortcut: None,
        about: "The lab series: the four apps' identity cards and Open buttons.",
        mockup: "Series.dc.html",
        board: Board {
            width: 1440.0,
            height: 640.0,
        },
    },
];

/// The screen with this id (exact, lowercase).
pub fn screen(id: &str) -> Option<&'static ScreenInfo> {
    SCREENS.iter().find(|s| s.id == id)
}

/// The screen ids, in order (for tool schemas and error messages).
pub fn screen_ids() -> Vec<&'static str> {
    SCREENS.iter().map(|s| s.id).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn screens_and_modes() {
        assert_eq!(
            screen_ids(),
            ["tray", "loom", "terrarium", "rule-lab", "series"]
        );
        let lab = screen("rule-lab").unwrap().screen_ref();
        assert_eq!(lab.label, "Rule lab");
        assert_eq!(lab.mode.unwrap().id, "advanced");
        assert!(screen("series").unwrap().mode.is_none());
        for m in MODES {
            assert_eq!(screen(m.home).unwrap().mode, Some(m));
        }
        assert!(screen("lab").is_none());
    }

    #[test]
    fn socket_paths() {
        assert_eq!(socket_from(Some("off".into())), None);
        assert_eq!(socket_from(Some("".into())), None);
        assert_eq!(
            socket_from(Some("/tmp/x.sock".into())),
            Some(PathBuf::from("/tmp/x.sock"))
        );
        let p = socket_from(None).unwrap();
        assert!(p.as_os_str().len() <= lab_mcp::bridge::MAX_SOCKET_PATH);
        assert!(p.to_string_lossy().contains("AutomataPlayground"));
    }
}
