//! Regressions for the independent review's ap-mcp items (each test names its item).

use ap_mcp::app::{AppLink, LinkError};
use ap_mcp::engine::{RunLifeArgs, run_life};
use ap_mcp::{Config, registry};
use lab_mcp::bridge::MAX_REQUEST_BYTES;
use serde_json::{Map, Value, json};
use std::time::Duration;

/// `run_life` refused a pattern by its declared box (`x = 300, y = 1` around one cell: "the
/// pattern is 300×1"); like the Tray it now judges a box too big for the grid by its live
/// cells' bounding box.
#[test]
fn run_life_judges_size_by_the_live_cells() {
    let args = RunLifeArgs {
        pattern: "x = 300, y = 1\no!".into(),
        gens: Some(4),
        ..RunLifeArgs::default()
    };
    let r = run_life(&args, &|| false).unwrap();
    assert_eq!((r.pattern.width, r.pattern.height), (1, 1));
    assert_eq!(r.pattern.population, 1);
    // a declared box wider than a given grid, live cells that fit it
    let args = RunLifeArgs {
        pattern: "x = 50, y = 50\n3o!".into(),
        width: Some(10),
        height: Some(10),
        gens: Some(2),
        ..RunLifeArgs::default()
    };
    let r = run_life(&args, &|| false).unwrap();
    assert_eq!((r.pattern.width, r.pattern.height), (3, 1));
    assert_eq!((r.width, r.height), (10, 10));
    // a box that fits keeps its margins (positions are relative to it)
    let args = RunLifeArgs {
        pattern: "x = 5, y = 5\n$$2bo!".into(),
        gens: Some(1),
        ..RunLifeArgs::default()
    };
    let r = run_life(&args, &|| false).unwrap();
    assert_eq!((r.pattern.width, r.pattern.height), (5, 5));
    // live cells that really are too big still say so
    let wide = "x = 300, y = 1\no298bo!".to_string();
    let err = run_life(
        &RunLifeArgs {
            pattern: wide,
            ..RunLifeArgs::default()
        },
        &|| false,
    )
    .unwrap_err();
    assert!(err.message.contains("300×1"), "{}", err.message);
}

/// The worst case a schema allows: every string at its `maxLength`, of characters JSON
/// escapes to 6 bytes (`\u0001`).
fn worst_args(schema: &Value) -> Map<String, Value> {
    let mut args = Map::new();
    if let Some(props) = schema["properties"].as_object() {
        for (k, p) in props {
            if let Some(n) = p["maxLength"].as_u64() {
                args.insert(k.clone(), Value::String("\u{1}".repeat(n as usize)));
            }
        }
    }
    args
}

/// `tray_load_pattern` documented `maxLength: 1_000_000` for its pattern, but the app's bridge
/// reads at most 1 MiB per request line after JSON escaping, so a valid call could be cut off
/// with a protocol error. Every app tool's string limits now fit one request line.
#[test]
fn app_tool_string_limits_fit_one_bridge_request() {
    let tools = registry(&Config { socket: None }).unwrap();
    let mut checked = 0;
    for tool in tools.tools() {
        let app_tool = ["tray_", "loom_", "terrarium_", "lab_"]
            .iter()
            .any(|p| tool.name.starts_with(p));
        if !app_tool {
            continue;
        }
        let args = worst_args(&tool.input_schema);
        if args.is_empty() {
            continue;
        }
        let line =
            json!({"id": u64::MAX, "method": "terrarium.breed", "params": args}).to_string() + "\n";
        assert!(
            line.len() <= MAX_REQUEST_BYTES,
            "{}: {} bytes at its maxLengths",
            tool.name,
            line.len()
        );
        checked += 1;
    }
    assert!(
        checked >= 3,
        "tray_set_rule, tray_load_pattern, lab_set_script, …"
    );
}

/// Arguments over the bridge's line limit (a client that ignores `maxLength`) get a clear
/// error before anything is sent, not a protocol error from the app.
#[test]
fn oversized_app_calls_are_refused_with_a_clear_message() {
    let link = AppLink::new(Some(std::env::temp_dir().join("ap-review-no-such.sock")));
    let params = json!({"pattern": "o".repeat(MAX_REQUEST_BYTES + 10)});
    let err = link
        .call::<Value>("tray.load_pattern", params, Duration::from_secs(1))
        .unwrap_err();
    match err {
        LinkError::Refused(m) => assert!(m.contains("1 MiB"), "{m}"),
        other => panic!("{other:?}"),
    }
}
