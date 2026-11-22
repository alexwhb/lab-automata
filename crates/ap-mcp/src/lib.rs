//! `automata-mcp`: the Automata Playground MCP server.
//!
//! The tool layer has three parts, registered in this order (which is also `tools/list` order):
//!
//! - [`engine`]: tools that run on `ap-engine` in-process and headless, so they work without
//!   the app: run a Life-like rule on a pattern, analyse a Rule-lab spec, weave an elementary
//!   CA, parse rules / patterns / rule scripts, list the presets.
//! - [`app`]: the lab-series tools on the running app over the live bridge (`app_status`,
//!   `list_screens`, `navigate`, `screenshot`, `describe_screen`).
//! - [`benches`]: the Automata tools on the running app: the Tray, the Loom, the Terrarium and
//!   the Rule lab (read their state, change rules / patterns / scripts, run / pause / step).
//!
//! App tools return a tool error saying how to start the app when it isn't running.
//! [`server`] builds the [`McpServer`]; the `automata-mcp` binary serves it on stdio.

#![warn(missing_docs)]

pub mod app;
pub mod benches;
pub mod engine;

use lab_mcp::mcp::{McpServer, RegistryError, ServerInfo, ToolRegistry};
use serde_json::{Value, json};
use std::path::PathBuf;

/// What the server needs to know.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    /// The running app's bridge socket (`None`: the bridge is disabled).
    pub socket: Option<PathBuf>,
}

impl Config {
    /// `$AUTOMATA_MCP_SOCKET`, else the app's per-user default socket.
    pub fn from_env() -> Self {
        Self {
            socket: ap_shared::bridge_socket_path(),
        }
    }
}

/// Guidance for the model, sent with `initialize` / `server/discover`.
pub const INSTRUCTIONS: &str = "Automata Playground is a desktop cellular-automata lab. \
Playground mode has three benches: the Tray (a 40×26 torus of Life-like cells), the Loom \
(elementary 1D rules, 0–255) and the Terrarium (six jars of rules to breed). Advanced mode is \
the Rule lab (any neighbourhood in 5×5, a 400-generation run, analysis and an editable rule \
script).\n\nEngine tools work without the app: run_life, analyze_rule, elementary, parse_rule, \
parse_pattern, list_presets, rule_script. Rules are written B3/S23 (or a name like HighLife); \
patterns as RLE, plaintext rows of . and O, or a stamp id (glider, lwss, pulsar, penta, rpent, \
acorn, gun, rep).\n\nApp tools act on the running app, so the user must have it open: \
app_status, list_screens, navigate, screenshot, describe_screen, and tray_* / loom_* / \
terrarium_* / lab_* for each screen. Tools that change a screen switch to it by default \
(show: false to stay put). Prefer the *_get tools or describe_screen (text) to screenshot \
when text is enough.";

/// Every tool, in `tools/list` order.
pub fn registry(config: &Config) -> Result<ToolRegistry, RegistryError> {
    let mut tools = ToolRegistry::new();
    engine::register(&mut tools)?;
    let link = app::AppLink::new(config.socket.clone());
    app::register(&mut tools, link.clone())?;
    benches::register(&mut tools, link)?;
    Ok(tools)
}

/// The MCP server for `config`.
pub fn server(config: &Config) -> Result<McpServer, RegistryError> {
    let info = ServerInfo {
        name: "automata-mcp".into(),
        version: env!("CARGO_PKG_VERSION").into(),
        title: Some(ap_shared::APP_NAME.into()),
    };
    Ok(McpServer::new(info, registry(config)?).with_instructions(INSTRUCTIONS))
}

/// JSON Schema for a `screen` argument: one of the screen ids.
pub(crate) fn screen_schema(description: &str) -> Value {
    json!({
        "type": "string",
        "enum": ap_shared::screen_ids(),
        "description": description,
    })
}

/// JSON Schema for the `show` argument of tools that change a screen.
pub(crate) fn show_schema(screen: &str) -> Value {
    json!({
        "type": "boolean",
        "description": format!(
            "Switch the app to the {screen} so the user sees the change (default true). \
             false changes it in the background."
        ),
    })
}
