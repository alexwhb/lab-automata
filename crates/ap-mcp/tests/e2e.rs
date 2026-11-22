//! End to end: the real `automata-mcp` binary (built by Cargo for this package's tests,
//! `CARGO_BIN_EXE_automata-mcp`) speaks MCP on stdin/stdout. Engine tools run with no app;
//! app tools reach the real `AutomataApp` running in an egui_kittest harness over the live
//! bridge on a temp socket, and their effect is checked on the screens' state and through the
//! GUI (AccessKit labels).
//!
//! The test lives in the `ap-mcp` package, not the app's, so Cargo builds the binary for it.
//! The harness is stepped while a tool call waits, standing in for the app's frame loop.
//! Screenshots are real: egui_kittest fulfils `ViewportCommand::Screenshot` by rendering the
//! frame with wgpu, which is what eframe does in the app.
//!
//! `AP_MCP_E2E_DUMP=<dir>` saves the screenshot and the outline there for a look.

mod common;

use automata_playground::Route;
use common::{Mcp, TempSocket, app, data, error, has_label, text};
use serde_json::{Value, json};

const ENGINE_TOOLS: [&str; 7] = [
    "run_life",
    "analyze_rule",
    "elementary",
    "parse_rule",
    "parse_pattern",
    "list_presets",
    "rule_script",
];
const APP_TOOLS: [&str; 17] = [
    "app_status",
    "list_screens",
    "navigate",
    "screenshot",
    "describe_screen",
    "tray_get",
    "tray_set_rule",
    "tray_load_pattern",
    "tray_control",
    "loom_get",
    "loom_set",
    "terrarium_get",
    "terrarium_keep",
    "terrarium_breed",
    "lab_get",
    "lab_set_script",
    "lab_scrub",
];

/// A socket nobody listens on (engine tools don't need one).
fn offline() -> (TempSocket, Mcp) {
    let sock = TempSocket::new();
    let mcp = Mcp::started(&sock.path());
    (sock, mcp)
}

// ---- protocol -------------------------------------------------------------------------------

#[test]
fn initialize_and_list_every_tool() {
    let sock = TempSocket::new();
    let mut mcp = Mcp::spawn(&sock.path());
    let init = mcp.initialize();
    assert_eq!(init["result"]["protocolVersion"], "2025-11-25");
    assert_eq!(init["result"]["serverInfo"]["name"], "automata-mcp");
    assert_eq!(init["result"]["serverInfo"]["title"], "Automata Playground");
    let instructions = init["result"]["instructions"].as_str().unwrap();
    assert!(instructions.contains("run_life"), "{instructions}");

    let list = mcp.call("tools/list", json!({}));
    let tools = list["result"]["tools"].as_array().unwrap();
    let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
    let expected: Vec<&str> = ENGINE_TOOLS.iter().chain(&APP_TOOLS).copied().collect();
    assert_eq!(names, expected, "every tool, in order");
    for t in tools {
        let name = t["name"].as_str().unwrap();
        assert!(t["title"].is_string(), "{name}: title");
        assert!(
            t["description"].as_str().unwrap().len() > 60,
            "{name}: a real description"
        );
        assert_eq!(t["inputSchema"]["type"], "object", "{name}");
        assert_eq!(t["inputSchema"]["additionalProperties"], false, "{name}");
        assert!(t["annotations"]["readOnlyHint"].is_boolean(), "{name}");
    }
    mcp.close();
}

#[test]
fn version_and_help() {
    let exe = env!("CARGO_BIN_EXE_automata-mcp");
    let out = std::process::Command::new(exe)
        .arg("--version")
        .output()
        .unwrap();
    assert!(out.status.success());
    assert_eq!(
        String::from_utf8(out.stdout).unwrap(),
        format!("automata-mcp {}\n", env!("CARGO_PKG_VERSION"))
    );
    let out = std::process::Command::new(exe)
        .arg("--help")
        .output()
        .unwrap();
    assert!(out.status.success());
    let help = String::from_utf8(out.stdout).unwrap();
    assert!(help.contains("--socket <path>"), "{help}");
    assert!(
        help.contains("claude mcp add automata-playground"),
        "{help}"
    );
    assert!(help.contains("AUTOMATA_MCP_SOCKET"), "{help}");
    let out = std::process::Command::new(exe)
        .arg("--frobnicate")
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&out.stderr).contains("unknown argument `--frobnicate`"));
}

// ---- engine tools (no app) ------------------------------------------------------------------

#[test]
fn analyze_rule_reproduces_the_rule_lab() {
    let (_s, mut mcp) = offline();
    let r = mcp.tool(
        "analyze_rule",
        json!({"rule": "B3/S23", "seed": "rpent", "boundary": "torus", "gens": 400}),
    );
    let d = data(&r);
    assert_eq!(d["report"]["peak"], 221);
    assert_eq!(d["report"]["peak_gen"], 148);
    assert_eq!(d["report"]["peak_text"], "221 @ 148");
    assert_eq!(d["report"]["final_population"], 46);
    assert_eq!(d["report"]["period"], 2);
    assert_eq!(d["report"]["first_repeat_text"], "gen 390");
    assert_eq!(
        d["report"]["verdict"],
        "Settles into a period-2 cycle at gen 388."
    );
    assert_eq!(d["spec"]["width"], 64);
    assert_eq!(d["spec"]["height"], 40);
    assert_eq!(d["spec"]["neighbourhood"]["name"], "Moore r1");
    assert_eq!(d["at"]["generation"], 400);
    assert_eq!(d["at"]["population"], 46);
    assert!(text(&r).ends_with("Settles into a period-2 cycle at gen 388."));
    // The script it returns is one the Rule lab (and rule_script) accepts as is.
    let script = d["script"].as_str().unwrap().to_owned();
    let check = mcp.tool("rule_script", json!({"script": script}));
    assert_eq!(data(&check)["changed"], false);
    assert_eq!(data(&check)["report"], d["report"]);

    // Gen 120 is the mockup's 148 alive; a custom neighbourhood and a soup run too.
    let r = mcp.tool("analyze_rule", json!({"generation": 120, "samples": 5}));
    assert_eq!(data(&r)["at"]["population"], 148);
    assert_eq!(data(&r)["series"].as_array().unwrap().len(), 5);
    let r = mcp.tool(
        "analyze_rule",
        json!({"rule": "B2/S", "neighbourhood": [[0,-1],[-1,0],[1,0],[0,1]], "boundary": "dead",
               "seed": "soup", "soup_seed": 11, "density": 0.5, "width": 32, "height": 32, "gens": 50}),
    );
    let d = data(&r);
    assert_eq!(d["spec"]["neighbourhood"]["key"], "vn");
    assert_eq!(d["spec"]["seed"]["soup_seed"], 11);
    assert_eq!(d["spec"]["boundary"], "dead");

    // Errors are tool errors that say what to fix.
    let e = mcp.tool("analyze_rule", json!({"rule": "B9/S23"}));
    assert!(
        error(&e).contains("couldn't read the rule `B9/S23`"),
        "{}",
        text(&e)
    );
    let e = mcp.tool(
        "analyze_rule",
        json!({"rule": "B5/S23", "neighbourhood": "vn"}),
    );
    assert!(
        error(&e).contains("more than this neighbourhood has (4)"),
        "{}",
        text(&e)
    );
    let e = mcp.tool("analyze_rule", json!({"neighbourhood": [[0, 0]]}));
    assert!(error(&e).contains("not a neighbour"), "{}", text(&e));
    let e = mcp.tool("analyze_rule", json!({"seed": "acorn", "density": 0.2}));
    assert!(
        error(&e).contains("only apply to `seed: soup`"),
        "{}",
        text(&e)
    );
    let e = mcp.tool("analyze_rule", json!({"gens": 5000}));
    assert!(error(&e).contains("`gens` must be ≤ 2000"), "{}", text(&e));
    let e = mcp.tool("analyze_rule", json!({"gens": 10, "generation": 11}));
    assert!(
        error(&e).contains("past the run's last generation (10)"),
        "{}",
        text(&e)
    );
    mcp.close();
}

#[test]
fn run_life_on_patterns() {
    let (_s, mut mcp) = offline();
    // A glider by stamp id: a spaceship.
    let r = mcp.tool("run_life", json!({"pattern": "glider", "gens": 40}));
    let d = data(&r);
    assert_eq!(d["pattern"]["source"], "stamp");
    assert_eq!(d["population"]["final_population"], 5);
    assert_eq!(
        d["motion"],
        json!({"start": 0, "period": 4, "dx": 1, "dy": 1})
    );
    assert_eq!(
        d["born_total"], 80,
        "a glider: 2 born and 2 dead every generation"
    );
    assert_eq!(d["died_total"], 80);
    assert_eq!(d["rule_source"], "default");
    assert!(text(&r).contains("A spaceship: moves (1, 1) every 4 gens"));

    // A blinker as plaintext, rendered: period 2.
    let r = mcp.tool(
        "run_life",
        json!({"pattern": ".O.\n.O.\n.O.", "width": 5, "height": 5, "gens": 3, "render": true}),
    );
    let d = data(&r);
    assert_eq!(
        d["first_repeat"],
        json!({"start": 0, "period": 2, "seen_at": 2})
    );
    assert_eq!(d["verdict"], "Settles into a period-2 cycle at gen 0.");
    assert_eq!(
        d["render"]["rows"],
        json!([".....", ".....", ".OOO.", ".....", "....."])
    );
    assert_eq!(d["final_rle"], "x = 3, y = 1, rule = B3/S23\n3o!\n");

    // RLE with a rule header: Seeds explodes; `rule` overrides the header.
    let rle = "x = 2, y = 1, rule = B2/S\n2o!";
    let r = mcp.tool("run_life", json!({"pattern": rle, "gens": 3}));
    assert_eq!(data(&r)["rule"]["code"], "B2/S");
    assert_eq!(data(&r)["rule_source"], "pattern");
    let r = mcp.tool(
        "run_life",
        json!({"pattern": rle, "rule": "Life", "gens": 3}),
    );
    assert_eq!(data(&r)["rule"]["code"], "B3/S23");
    assert_eq!(data(&r)["extinct_gen"], 1);
    assert_eq!(data(&r)["verdict"], "Dies out by gen 1.");

    // The R-pentomino on dead edges, sampled.
    let r = mcp.tool(
        "run_life",
        json!({"pattern": "rpent", "boundary": "dead", "gens": 100, "samples": 11}),
    );
    let series = data(&r)["series"].as_array().unwrap();
    assert_eq!(series.len(), 11);
    assert_eq!(series[0], json!([0, 5]));
    assert_eq!(series[10][0], 100);

    // Errors.
    let e = mcp.tool("run_life", json!({"pattern": "x = 3, y = 3\nbo$2bo$3q!"}));
    assert!(
        error(&e).starts_with("couldn't read the pattern as RLE"),
        "{}",
        text(&e)
    );
    let e = mcp.tool("run_life", json!({"pattern": ""}));
    assert!(error(&e).contains("the pattern is empty"), "{}", text(&e));
    let e = mcp.tool("run_life", json!({"pattern": "glider", "rule": "B3/S23/2"}));
    assert!(error(&e).contains("Generations rule"), "{}", text(&e));
    let e = mcp.tool("run_life", json!({"pattern": "gun", "width": 20}));
    assert!(
        error(&e).contains("doesn't fit a grid width of 20"),
        "{}",
        text(&e)
    );
    let e = mcp.tool(
        "run_life",
        json!({"pattern": "glider", "width": 256, "height": 256, "gens": 10000}),
    );
    assert!(error(&e).contains("the limit is 200000000"), "{}", text(&e));
    let e = mcp.tool(
        "run_life",
        json!({"pattern": "glider", "boundary": "klein"}),
    );
    assert!(
        error(&e).contains("must be one of: torus, dead"),
        "{}",
        text(&e)
    );
    let e = mcp.tool("run_life", json!({"rule": "B3/S23"}));
    assert!(
        error(&e).contains("missing required argument `pattern`"),
        "{}",
        text(&e)
    );
    mcp.close();
}

#[test]
fn elementary_rows() {
    let (_s, mut mcp) = offline();
    let r = mcp.tool("elementary", json!({"rule": 30, "rows": 20}));
    let d = data(&r);
    assert_eq!(d["blurb"], "Chaos · Random-looking");
    assert_eq!(d["binary"], "00011110");
    assert_eq!(d["width"], 121);
    let rows = d["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 20);
    // Rule 30's centre column (OEIS A051023).
    let centre: Vec<u8> = rows
        .iter()
        .map(|r| u8::from(r.as_str().unwrap().as_bytes()[60] == b'#'))
        .collect();
    assert_eq!(
        centre,
        [1, 1, 0, 1, 1, 1, 0, 0, 1, 1, 0, 0, 0, 1, 0, 1, 1, 0, 0, 1]
    );
    assert_eq!(d["counts"][0], 1);

    // Rule 184 conserves cars; a seeded random row is reproducible.
    let a = mcp.tool(
        "elementary",
        json!({"rule": 184, "width": 50, "rows": 30, "start": "random", "seed": 9}),
    );
    let counts = data(&a)["counts"].as_array().unwrap().clone();
    assert!(counts.iter().all(|c| *c == counts[0]), "{counts:?}");
    let b = mcp.tool(
        "elementary",
        json!({"rule": 184, "width": 50, "rows": 30, "seed": 9}),
    );
    assert_eq!(data(&a)["rows"], data(&b)["rows"]);
    assert_eq!(data(&b)["start"], "random");

    let e = mcp.tool("elementary", json!({"rule": 256}));
    assert!(error(&e).contains("`rule` must be ≤ 255"), "{}", text(&e));
    let e = mcp.tool(
        "elementary",
        json!({"rule": 30, "width": 1000, "rows": 1000}),
    );
    assert!(
        error(&e).contains("the limit is 65536 cells"),
        "{}",
        text(&e)
    );
    let e = mcp.tool(
        "elementary",
        json!({"rule": 30, "start": "single", "seed": 1}),
    );
    assert!(error(&e).contains("`seed` only applies"), "{}", text(&e));
    mcp.close();
}

#[test]
fn parse_rules_patterns_and_scripts() {
    let (_s, mut mcp) = offline();
    let r = mcp.tool("parse_rule", json!({"rule": "b36s23"}));
    let d = data(&r);
    assert_eq!(d["code"], "B36/S23");
    assert_eq!(d["name"], "HighLife");
    assert_eq!(d["birth"], json!([3, 6]));
    assert_eq!(d["survive"], json!([2, 3]));
    assert_eq!(d["cartridge"]["id"], "high");
    assert_eq!(text(&r), "b36s23 = B36/S23 (HighLife; Moore r1).");
    assert_eq!(
        data(&mcp.tool("parse_rule", json!({"rule": "23/3"})))["code"],
        "B3/S23"
    );
    let r = mcp.tool("parse_rule", json!({"rule": "Day & Night"}));
    assert_eq!(data(&r)["code"], "B3678/S34678");
    let r = mcp.tool(
        "parse_rule",
        json!({"rule": "B3,10/S2,3", "neighbourhood": "moore2"}),
    );
    assert_eq!(data(&r)["code"], "B3,10/S2,3");
    assert_eq!(data(&r)["name"], Value::Null);
    let e = mcp.tool("parse_rule", json!({"rule": "B3/X23"}));
    assert!(error(&e).contains("couldn't read the rule"), "{}", text(&e));

    let r = mcp.tool(
        "parse_pattern",
        json!({"pattern": "#N Glider\n#O Richard K. Guy\nx = 3, y = 3, rule = B3/S23\nbo$2bo$3o!"}),
    );
    let d = data(&r);
    assert_eq!(d["source"], "rle");
    assert_eq!(d["name"], "Glider");
    assert_eq!(d["author"], "Richard K. Guy");
    assert_eq!(
        (d["width"].as_u64(), d["height"].as_u64()),
        (Some(3), Some(3))
    );
    assert_eq!(d["population"], 5);
    assert_eq!(d["cells"], json!([".O.", "..O", "OOO"]));
    assert_eq!(d["rule_code"], "B3/S23");
    let r = mcp.tool("parse_pattern", json!({"pattern": "!Name: blinker\nOOO"}));
    assert_eq!(data(&r)["source"], "plaintext");
    assert_eq!(data(&r)["rle"], "#N blinker\nx = 3, y = 1\n3o!\n");
    let r = mcp.tool("parse_pattern", json!({"pattern": "Glider gun"}));
    assert_eq!(data(&r)["stamp"], "gun");
    assert_eq!(data(&r)["population"], 36);
    let e = mcp.tool(
        "parse_pattern",
        json!({"pattern": "x = 3, y = 3\nbo$2bo$3q!"}),
    );
    assert!(error(&e).contains("line 2"), "{}", text(&e));

    // rule_script: canonical text, or every error with its line.
    let messy = "* mine\n.rule life_like b36s23\n.neigh moore n=8\n.grid 64x40 boundary=torus\n\
                 .seed acorn\n.run gens=200\n";
    let r = mcp.tool("rule_script", json!({"script": messy}));
    let d = data(&r);
    assert_eq!(d["valid"], true);
    assert_eq!(d["changed"], true);
    assert_eq!(d["spec"]["rule"]["code"], "B36/S23");
    assert_eq!(d["spec"]["seed"]["kind"], "acorn");
    assert_eq!(d["spec"]["gens"], 200);
    assert!(d["report"]["verdict"].is_string());
    let r = mcp.tool("rule_script", json!({"script": messy, "analyze": false}));
    assert_eq!(data(&r)["report"], Value::Null);
    let bad = ".rule life_like B3/S23\n.neigh moore n=8\n.grid 0x40 boundary=moon\n.run gens=x\n";
    let r = mcp.tool("rule_script", json!({"script": bad}));
    let d = data(&r);
    assert_eq!(d["valid"], false);
    let lines: Vec<u64> = d["errors"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["line"].as_u64().unwrap())
        .collect();
    assert!(lines.contains(&3) && lines.contains(&4), "{lines:?}");
    assert!(text(&r).starts_with("The script has"), "{}", text(&r));
    assert!(text(&r).contains("line 3:"), "{}", text(&r));
    mcp.close();
}

#[test]
fn list_presets_and_screens_need_no_app() {
    let (_s, mut mcp) = offline();
    let r = mcp.tool("list_presets", json!({}));
    let d = data(&r);
    let ids = |k: &str, f: &str| -> Vec<String> {
        d[k].as_array()
            .unwrap()
            .iter()
            .map(|x| x[f].to_string().trim_matches('"').to_owned())
            .collect()
    };
    assert_eq!(
        ids("cartridges", "id"),
        ["life", "high", "seeds", "dn", "maze", "coral"]
    );
    assert_eq!(
        ids("stamps", "id"),
        [
            "glider", "lwss", "pulsar", "penta", "rpent", "acorn", "gun", "rep"
        ]
    );
    assert_eq!(
        ids("famous_threads", "rule"),
        ["30", "90", "110", "184", "54", "150"]
    );
    assert_eq!(
        ids("neighbourhoods", "key"),
        ["moore", "vn", "hex", "moore2"]
    );
    assert_eq!(d["terrarium_start"].as_array().unwrap().len(), 6);

    let r = mcp.tool("list_screens", json!({}));
    let d = data(&r);
    let screens = d["screens"].as_array().unwrap();
    assert_eq!(screens.len(), Route::ALL.len());
    for (s, route) in screens.iter().zip(Route::ALL) {
        assert_eq!(s["id"], route.id());
        assert_eq!(s["label"], route.title());
        assert_eq!(s["mockup"], route.mockup());
    }
    assert_eq!(d["current"], Value::Null, "nothing is running");
    assert_eq!(d["modes"].as_array().unwrap().len(), 2);
    mcp.close();
}

// ---- app tools ------------------------------------------------------------------------------

#[test]
fn drive_the_running_app_over_mcp() {
    let sock = TempSocket::new();
    let mut app = app(&sock.path());
    let mut mcp = Mcp::started(&sock.path());

    // navigate → the app's route changes
    assert_eq!(app.state().route(), Route::Tray);
    let r = mcp.call_tool(&mut app, "navigate", json!({"screen": "rule-lab"}));
    assert_eq!(text(&r), "Now showing Rule lab (Advanced mode).");
    assert_eq!(app.state().route(), Route::RuleLab);

    // app_status / list_screens
    let r = mcp.call_tool(&mut app, "app_status", json!({}));
    let s = data(&r);
    assert_eq!(s["running"], true);
    assert_eq!(s["screen"]["id"], "rule-lab");
    assert_eq!(s["screen"]["mode"]["id"], "advanced");
    assert_eq!(s["pid"], std::process::id());
    assert_eq!(s["window"]["width"], 1440.0);
    let r = mcp.call_tool(&mut app, "list_screens", json!({}));
    assert_eq!(data(&r)["current"], "rule-lab");
    assert!(
        text(&r).contains("rule-lab (Advanced, showing)"),
        "{}",
        text(&r)
    );

    // screenshot (navigating first) → a PNG of the rendered frame
    let r = mcp.call_tool(&mut app, "screenshot", json!({"screen": "tray"}));
    assert_eq!(r["isError"], false, "{r}");
    assert_eq!(app.state().route(), Route::Tray);
    assert_eq!(text(&r), "Tray · 2D (Playground mode) — 1440×900 px.");
    let img = &r["content"][1];
    assert_eq!(img["type"], "image");
    assert_eq!(img["mimeType"], "image/png");
    let png = lab_mcp::base64_decode(img["data"].as_str().unwrap()).unwrap();
    let png = image::load_from_memory(&png).unwrap().to_rgba8();
    assert_eq!(png.dimensions(), (1440, 900));
    let colours: std::collections::HashSet<[u8; 4]> = png.pixels().map(|p| p.0).collect();
    assert!(
        colours.len() > 50,
        "a blank frame ({} colours)",
        colours.len()
    );
    let dump = std::env::var_os("AP_MCP_E2E_DUMP").map(std::path::PathBuf::from);
    if let Some(dir) = &dump {
        png.save(dir.join("screenshot.png")).unwrap();
    }

    // describe_screen → the accessibility outline with known labels
    let r = mcp.call_tool(&mut app, "describe_screen", json!({"screen": "loom"}));
    assert_eq!(app.state().route(), Route::Loom);
    let outline = text(&r);
    if let Some(dir) = &dump {
        std::fs::write(dir.join("outline.txt"), outline).unwrap();
    }
    assert!(
        outline.starts_with("Loom · 1D (Playground mode)"),
        "{outline}"
    );
    for label in [
        "\"Automata Playground\"",
        "\"Playground\"",
        "\"Advanced\"",
        "\"Weave\"",
    ] {
        assert!(outline.contains(label), "{label} missing from:\n{outline}");
    }
    assert!(outline.contains("Woven pattern, rule 30"), "{outline}");
    assert!(r["structuredContent"]["nodes"].as_array().unwrap().len() > 20);

    // Bad arguments come back as tool errors the model can act on; nothing moves.
    let r = mcp.call_tool(&mut app, "navigate", json!({"screen": "lab"}));
    assert!(error(&r).contains("must be one of"), "{}", text(&r));
    assert_eq!(app.state().route(), Route::Loom);

    // A modern (2026-07-28, stateless) request on the same process works too.
    let id = mcp.request(
        "tools/call",
        json!({
            "name": "app_status",
            "_meta": {
                "io.modelcontextprotocol/protocolVersion": "2026-07-28",
                "io.modelcontextprotocol/clientCapabilities": {}
            }
        }),
    );
    let r = mcp.response_with(id, || app.step());
    assert_eq!(r["result"]["resultType"], "complete");
    assert_eq!(r["result"]["structuredContent"]["screen"]["id"], "loom");
    mcp.close();
}

#[test]
fn tray_tools_change_what_the_gui_shows() {
    let sock = TempSocket::new();
    let mut app = app(&sock.path());
    let mut mcp = Mcp::started(&sock.path());
    mcp.call_tool(&mut app, "navigate", json!({"screen": "series"}));

    // tray_set_rule → the Tray shows, with HighLife lit.
    let r = mcp.call_tool(&mut app, "tray_set_rule", json!({"rule": "HighLife"}));
    assert_eq!(data(&r)["rule"], "B36/S23");
    assert!(
        text(&r).starts_with("Tray: B36/S23 (HighLife), gen 0"),
        "{}",
        text(&r)
    );
    assert_eq!(app.state().route(), Route::Tray);
    assert_eq!(app.state().screens().tray.rule().code(), "B36/S23");
    assert!(has_label(&app, "Rule: HighLife"));

    // tray_control step → generations and counters advance on screen.
    let before = app.state().screens().tray.stats();
    let r = mcp.call_tool(&mut app, "tray_control", json!({"action": "step", "n": 3}));
    let d = data(&r);
    assert_eq!(d["generation"], before.generation + 3);
    assert_eq!(d["running"], false);
    let after = app.state().screens().tray.stats();
    assert_eq!(after.generation, before.generation + 3);
    assert_eq!(d["population"], after.population);
    assert_eq!(
        (&d["born"], &d["died"]),
        (&json!(after.born), &json!(after.died))
    );
    assert!(has_label(&app, &format!("gen {:04}", after.generation)));
    assert!(has_label(&app, &format!("Alive: {}", after.population)));

    // tray_load_pattern with a glider (RLE), then 4 steps: it moved (+1, +1).
    let r = mcp.call_tool(
        &mut app,
        "tray_load_pattern",
        json!({"pattern": "x = 3, y = 3, rule = B3/S23\nbo$2bo$3o!"}),
    );
    let d = data(&r);
    assert_eq!(d["population"], 5);
    assert_eq!(d["generation"], 0);
    assert_eq!(d["rule"], "B3/S23", "the RLE's rule = applies");
    assert!(has_label(&app, "Alive: 5"));
    assert!(has_label(&app, "Rule: Life"));
    let b0 = app.state().screens().tray.grid().bbox().unwrap();
    let r = mcp.call_tool(&mut app, "tray_control", json!({"action": "step", "n": 4}));
    assert_eq!(data(&r)["population"], 5);
    let b1 = app.state().screens().tray.grid().bbox().unwrap();
    assert_eq!((b1.x, b1.y, b1.w, b1.h), (b0.x + 1, b0.y + 1, b0.w, b0.h));

    // tray_get reads the same thing the screen shows, RLE included.
    let r = mcp.call_tool(&mut app, "tray_get", json!({}));
    assert_eq!(data(&r)["rle"], app.state().screens().tray.rle());
    assert_eq!(
        data(&r)["mood"]["title"],
        app.state().screens().tray.stats().mood_title
    );
    assert!(has_label(&app, data(&r)["mood"]["title"].as_str().unwrap()));

    // run / pause, and the app's error for a bad rule.
    let r = mcp.call_tool(&mut app, "tray_control", json!({"action": "run"}));
    assert_eq!(data(&r)["running"], true);
    assert!(has_label(&app, "Pause"));
    mcp.call_tool(&mut app, "tray_control", json!({"action": "pause"}));
    assert!(!app.state().screens().tray.running());
    let r = mcp.call_tool(&mut app, "tray_set_rule", json!({"rule": "B9/S23"}));
    assert!(
        error(&r).contains("couldn't read the rule `B9/S23`"),
        "{}",
        text(&r)
    );
    mcp.close();
}

#[test]
fn loom_terrarium_and_rule_lab_tools() {
    let sock = TempSocket::new();
    let mut app = app(&sock.path());
    let mut mcp = Mcp::started(&sock.path());

    // Loom
    let r = mcp.call_tool(
        &mut app,
        "loom_set",
        json!({"rule": 110, "running": false, "step": 9, "rows": 10}),
    );
    let d = data(&r);
    assert_eq!(d["rule"], 110);
    assert_eq!(d["famous"]["name"], "Universal");
    assert_eq!(d["row"], 10);
    assert_eq!(d["rows"].as_array().unwrap().len(), 10);
    assert_eq!(app.state().route(), Route::Loom);
    assert_eq!(app.state().screens().loom.rule(), 110);
    assert!(has_label(&app, "row 0010"));
    let r = mcp.call_tool(&mut app, "loom_get", json!({"rows": 3}));
    assert_eq!(data(&r)["rows"].as_array().unwrap().len(), 3);
    assert!(
        text(&r).starts_with("Loom: rule 110 (01101110"),
        "{}",
        text(&r)
    );

    // Terrarium
    let r = mcp.call_tool(&mut app, "terrarium_keep", json!({"jars": [2]}));
    assert_eq!(data(&r)["jars"][1]["kept"], true);
    assert_eq!(app.state().route(), Route::Terrarium);
    assert!(app.state().screens().terrarium.is_kept(1));
    assert!(has_label(&app, "1 kept as parents"));
    let r = mcp.call_tool(&mut app, "terrarium_breed", json!({"ticks": 12}));
    let d = data(&r);
    assert_eq!(d["brood"], 2);
    assert_eq!(
        d["jars"][0]["rule"], "B36/S23",
        "the kept HighLife jar carries over"
    );
    assert_eq!(d["lineage"][0]["parents"], json!(["B36/S23"]));
    assert!(has_label(&app, "brood 2"));
    let r = mcp.call_tool(&mut app, "terrarium_get", json!({}));
    assert_eq!(data(&r)["brood"], 2);
    let r = mcp.call_tool(&mut app, "terrarium_keep", json!({"jars": [7]}));
    assert!(error(&r).contains("`jars[0]` must be ≤ 6"), "{}", text(&r));

    // Rule lab
    let r = mcp.call_tool(&mut app, "lab_get", json!({}));
    let d = data(&r);
    assert_eq!(
        d["report"]["verdict"],
        "Settles into a period-2 cycle at gen 388."
    );
    assert_eq!(
        app.state().route(),
        Route::Terrarium,
        "reading doesn't switch screens"
    );
    let script = d["script"]
        .as_str()
        .unwrap()
        .replace("r_pentomino", "acorn");
    let r = mcp.call_tool(&mut app, "lab_set_script", json!({"script": script}));
    let d = data(&r);
    assert_eq!(d["spec"]["seed"]["kind"], "acorn");
    assert_eq!(app.state().route(), Route::RuleLab);
    assert_eq!(app.state().screens().rule_lab.script_text(), script);
    let r = mcp.call_tool(&mut app, "lab_scrub", json!({"generation": 50}));
    assert_eq!(data(&r)["generation"], 50);
    assert_eq!(app.state().screens().rule_lab.generation(), 50);
    assert_eq!(data(&r)["alive"], app.state().screens().rule_lab.alive());
    let r = mcp.call_tool(
        &mut app,
        "lab_set_script",
        json!({"script": ".rule life_like B3/S23\n.grid 64x40 boundary=moon\n"}),
    );
    assert!(error(&r).starts_with("the script has"), "{}", text(&r));
    assert_eq!(
        app.state().screens().rule_lab.script_text(),
        script,
        "unchanged"
    );
    mcp.close();
}

#[test]
fn the_app_can_restart_under_a_running_server() {
    let sock = TempSocket::new();
    let mut mcp = Mcp::started(&sock.path());
    {
        let mut first = app(&sock.path());
        let r = mcp.call_tool(&mut first, "navigate", json!({"screen": "terrarium"}));
        assert_eq!(r["isError"], false, "{r}");
    } // the app quits (its socket goes away)
    let r = mcp.tool("navigate", json!({"screen": "loom"}));
    assert!(error(&r).starts_with("Automata Playground isn't running"));
    // …and comes back.
    let mut second = app(&sock.path());
    let r = mcp.call_tool(&mut second, "app_status", json!({}));
    assert_eq!(data(&r)["screen"]["id"], "tray");
    mcp.close();
}

#[test]
fn app_tools_say_clearly_when_the_app_is_not_running() {
    let (_s, mut mcp) = offline();
    let calls = [
        ("navigate", json!({"screen": "loom"})),
        ("screenshot", json!({})),
        ("describe_screen", json!({"screen": "tray"})),
        ("tray_get", json!({})),
        ("tray_set_rule", json!({"rule": "B3/S23"})),
        ("tray_load_pattern", json!({"pattern": "glider"})),
        ("tray_control", json!({"action": "step"})),
        ("loom_get", json!({})),
        ("loom_set", json!({"rule": 30})),
        ("terrarium_get", json!({})),
        ("terrarium_keep", json!({"jars": [1]})),
        ("terrarium_breed", json!({})),
        ("lab_get", json!({})),
        ("lab_set_script", json!({"script": ".end"})),
        ("lab_scrub", json!({"generation": 1})),
    ];
    for (tool, args) in calls {
        let r = mcp.tool(tool, args);
        let msg = error(&r);
        assert!(
            msg.starts_with("Automata Playground isn't running — open Automata Playground"),
            "{tool}: {msg}"
        );
        assert!(
            msg.contains("cargo run --release -p automata-playground"),
            "{tool}: {msg}"
        );
    }
    // app_status reports it as a normal result.
    let r = mcp.tool("app_status", json!({}));
    assert_eq!(data(&r)["running"], false);
    assert!(text(&r).starts_with("Automata Playground isn't running"));
    mcp.close();
}
