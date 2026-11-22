//! The app side of the live MCP bridge (`control.rs`): a real `AutomataApp` in an egui_kittest
//! harness listens on a temp socket, and a `lab_mcp` bridge client drives it. The harness is
//! stepped while the client waits, as frames would run in the real app. Changes are checked
//! both on the screens' state and through the GUI (AccessKit labels).
//!
//! The full stack (the real `automata-mcp` binary speaking MCP) is tested in
//! `crates/ap-mcp/tests/e2e.rs`.

use automata_playground::{AppOptions, AutomataApp, Route};
use eframe::egui;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use lab_mcp::bridge::proto::{AppStatus, Description, ImagePayload};
use lab_mcp::bridge::{BridgeClient, BridgeError, code};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::thread;
use std::time::{Duration, Instant};

type App = Harness<'static, AutomataApp>;

/// A private temp dir with a socket path in it, removed on drop.
struct TempSocket {
    dir: PathBuf,
}

impl TempSocket {
    fn new() -> Self {
        static N: AtomicU32 = AtomicU32::new(0);
        let dir = std::env::temp_dir().join(format!(
            "ap-bridge-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        Self { dir }
    }

    fn path(&self) -> PathBuf {
        self.dir.join("app.sock")
    }
}

impl Drop for TempSocket {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn harness_with(socket: &Path, opts: AppOptions) -> App {
    let opts = AppOptions {
        mcp_socket: Some(socket.to_path_buf()),
        ..opts
    };
    let mut h = Harness::builder()
        .with_size(egui::vec2(1440.0, 900.0))
        .with_pixels_per_point(1.0)
        .with_max_steps(64)
        .wgpu()
        .build_eframe(move |cc| AutomataApp::new(cc, opts));
    h.run_steps(3);
    h
}

fn harness(socket: &Path) -> App {
    harness_with(socket, AppOptions::test())
}

/// Calls the app from another thread, stepping frames until the reply arrives.
fn call(h: &mut App, socket: &Path, method: &str, params: Value) -> Result<Value, BridgeError> {
    let (socket, method) = (socket.to_path_buf(), method.to_owned());
    let client = thread::spawn(move || {
        BridgeClient::connect(&socket)?.call(&method, params, Duration::from_secs(15))
    });
    let t0 = Instant::now();
    while !client.is_finished() {
        assert!(t0.elapsed() < Duration::from_secs(20), "no reply");
        h.step();
        thread::sleep(Duration::from_millis(2));
    }
    let r = client.join().unwrap();
    h.run_steps(2); // let the GUI draw the change
    r
}

fn ok(h: &mut App, socket: &Path, method: &str, params: Value) -> Value {
    call(h, socket, method, params).unwrap_or_else(|e| panic!("{method}: {e}"))
}

fn refused(r: Result<Value, BridgeError>) -> (i64, String) {
    match r {
        Err(BridgeError::Remote { code, message, .. }) => (code, message),
        other => panic!("expected an error reply, got {other:?}"),
    }
}

fn has_label(h: &App, label: &str) -> bool {
    h.query_all_by_label(label).next().is_some()
}

#[test]
fn status_and_navigate() {
    let sock = TempSocket::new();
    let mut h = harness(&sock.path());

    let s: AppStatus =
        serde_json::from_value(ok(&mut h, &sock.path(), "status", Value::Null)).unwrap();
    assert_eq!(s.app, "Automata Playground");
    assert_eq!(s.pid, std::process::id());
    assert_eq!(s.screen.id, "tray");
    assert_eq!(s.screen.label, "Tray · 2D");
    assert_eq!(s.screen.mode.as_ref().unwrap().id, "playground");
    let w = s.window.unwrap();
    assert_eq!(
        (w.width, w.height, w.pixels_per_point),
        (1440.0, 900.0, 1.0)
    );

    let r = ok(
        &mut h,
        &sock.path(),
        "navigate",
        json!({"screen": "rule-lab"}),
    );
    let s: AppStatus = serde_json::from_value(r).unwrap();
    assert_eq!(s.screen.id, "rule-lab");
    assert_eq!(s.screen.mode.unwrap().id, "advanced");
    assert_eq!(h.state().route(), Route::RuleLab);

    // The Series screen belongs to no mode.
    let r = ok(
        &mut h,
        &sock.path(),
        "navigate",
        json!({"screen": "series"}),
    );
    assert_eq!(r["screen"]["mode"], Value::Null);
    assert_eq!(h.state().route(), Route::Series);
}

#[test]
fn bad_requests_get_error_replies_and_change_nothing() {
    let sock = TempSocket::new();
    let mut h = harness(&sock.path());
    let p = sock.path();
    let before = h.state().screens().tray.rle();
    let invalid = |r| refused(r).0;
    assert_eq!(
        invalid(call(&mut h, &p, "navigate", json!({"screen": "lab"}))),
        code::INVALID_PARAMS
    );
    assert_eq!(
        invalid(call(&mut h, &p, "navigate", json!({}))),
        code::INVALID_PARAMS
    );
    assert_eq!(
        invalid(call(&mut h, &p, "screenshot", json!({"screen": "nope"}))),
        code::INVALID_PARAMS
    );
    assert_eq!(
        invalid(call(&mut h, &p, "launch_rockets", Value::Null)),
        code::METHOD_NOT_FOUND
    );
    let (c, m) = refused(call(&mut h, &p, "tray.set_rule", json!({"rule": "B9/S23"})));
    assert_eq!(c, code::INVALID_PARAMS);
    assert!(m.contains("couldn't read the rule `B9/S23`"), "{m}");
    let (_, m) = refused(call(
        &mut h,
        &p,
        "tray.load_pattern",
        json!({"pattern": "x = 3, y = 3\nbo$2bo$3q!"}),
    ));
    assert!(m.starts_with("couldn't read the pattern as RLE"), "{m}");
    let (_, m) = refused(call(
        &mut h,
        &p,
        "tray.control",
        json!({"action": "run", "n": 3}),
    ));
    assert!(m.contains("`n` only applies to action `step`"), "{m}");
    let (_, m) = refused(call(
        &mut h,
        &p,
        "tray.control",
        json!({"action": "step", "n": 0}),
    ));
    assert!(m.contains("1 to 10000"), "{m}");
    let (_, m) = refused(call(&mut h, &p, "tray.control", json!({"action": "fly"})));
    assert!(m.contains("bad params for `tray.control`"), "{m}");
    let (_, m) = refused(call(
        &mut h,
        &p,
        "tray.set_rule",
        json!({"rule": "B3/S23", "colour": "red"}),
    ));
    assert!(m.contains("unknown field `colour`"), "{m}");
    let (_, m) = refused(call(&mut h, &p, "terrarium.keep", json!({"jars": [2, 9]})));
    assert!(m.contains("there is no jar 9"), "{m}");
    let (_, m) = refused(call(&mut h, &p, "terrarium.breed", json!({"mutation": 7})));
    assert!(m.contains("1 to 4"), "{m}");
    let (_, m) = refused(call(
        &mut h,
        &p,
        "lab.set_script",
        json!({"script": ".rule life_like B3/S23\n.grid 0x0 boundary=moon\n"}),
    ));
    assert!(m.starts_with("the script has"), "{m}");
    assert!(m.contains("line 2:"), "{m}");
    let (_, m) = refused(call(
        &mut h,
        &p,
        "loom.set",
        json!({"start": "single", "seed": 3}),
    ));
    assert!(m.contains("`seed` only applies"), "{m}");

    assert_eq!(h.state().route(), Route::Tray, "nothing moved");
    assert_eq!(h.state().screens().tray.rle(), before, "nothing changed");
    assert!(h.state().screens().terrarium.jars().iter().all(|j| !j.kept));
    assert!(h.state().screens().rule_lab.errors().is_empty());
}

#[test]
fn screenshot_is_the_rendered_frame() {
    let sock = TempSocket::new();
    let mut h = harness(&sock.path());
    let r = ok(
        &mut h,
        &sock.path(),
        "screenshot",
        json!({"screen": "loom"}),
    );
    let img: ImagePayload = serde_json::from_value(r).unwrap();
    assert_eq!(img.mime_type, "image/png");
    assert_eq!(img.screen.id, "loom");
    assert_eq!(h.state().route(), Route::Loom);
    let png = img.decode().unwrap();
    assert_eq!(png.dimensions(), (1440, 900));
    assert_eq!((img.width, img.height), (1440, 900));
    let colours: std::collections::HashSet<[u8; 4]> = png.pixels().map(|p| p.0).collect();
    assert!(colours.len() > 4, "only {} colours", colours.len());

    // Without `screen` it shows the current screen.
    let r = ok(&mut h, &sock.path(), "screenshot", Value::Null);
    assert_eq!(r["screen"]["id"], "loom");
}

#[test]
fn describe_outlines_the_accesskit_tree() {
    let sock = TempSocket::new();
    let mut h = harness(&sock.path());
    let r = ok(
        &mut h,
        &sock.path(),
        "describe",
        json!({"screen": "terrarium"}),
    );
    let summary = r["summary"].as_str().unwrap().to_owned();
    let d: Description = serde_json::from_value(r).unwrap();
    assert_eq!(d.screen.id, "terrarium");
    assert_eq!(h.state().route(), Route::Terrarium);
    assert_eq!(d.nodes[0].role, "Window");
    for label in [
        "Tray · 2D",
        "Loom · 1D",
        "Terrarium · breed",
        "Jar 1",
        "Jar 6",
    ] {
        assert!(
            d.outline.contains(&format!("{label:?}")),
            "{label} missing from:\n{}",
            d.outline
        );
    }
    assert!(d.outline.contains("[selected]"), "{}", d.outline);
    use automata_playground::Screen;
    assert_eq!(summary, h.state().screens().terrarium.describe());
}

#[test]
fn tray_methods_drive_the_tray() {
    let sock = TempSocket::new();
    let mut h = harness(&sock.path());
    let p = sock.path();

    let s = ok(&mut h, &p, "tray.get", Value::Null);
    assert_eq!(s["rule"], "B3/S23");
    assert_eq!(s["rule_name"], "Life");
    assert_eq!(s["cartridge"], "life");
    assert_eq!(
        (s["width"].as_u64(), s["height"].as_u64()),
        (Some(40), Some(26))
    );
    assert_eq!(s["rle"], h.state().screens().tray.rle());

    // set_rule → the rule panel shows HighLife.
    let s = ok(&mut h, &p, "tray.set_rule", json!({"rule": "b36s23"}));
    assert_eq!(s["rule"], "B36/S23");
    assert_eq!(s["rule_name"], "HighLife");
    assert_eq!(h.state().screens().tray.rule().code(), "B36/S23");
    assert!(has_label(&h, "Rule: HighLife"));
    let s = ok(&mut h, &p, "tray.set_rule", json!({"rule": "B3/S238"}));
    assert_eq!(s["rule_name"], "Custom rule");
    assert!(has_label(&h, "Rule: Custom rule"));

    // load_pattern → a glider in the middle, counters at 0; its `rule =` header applies.
    let s = ok(
        &mut h,
        &p,
        "tray.load_pattern",
        json!({"pattern": "x = 3, y = 3, rule = B3/S23\nbo$2bo$3o!"}),
    );
    assert_eq!(s["population"], 5);
    assert_eq!(s["generation"], 0);
    assert_eq!(s["rule"], "B3/S23");
    assert!(
        s["note"]
            .as_str()
            .unwrap()
            .contains("rule B3/S23 from the pattern"),
        "{s}"
    );
    assert!(has_label(&h, "Alive: 5"));
    assert!(has_label(&h, "gen 0000"));

    // step 4 → the glider moved one cell down-right and is still 5 cells.
    let before = h.state().screens().tray.grid().bbox().unwrap();
    let s = ok(
        &mut h,
        &p,
        "tray.control",
        json!({"action": "step", "n": 4}),
    );
    assert_eq!(s["generation"], 4);
    assert_eq!(s["population"], 5);
    assert_eq!(s["running"], false);
    let after = h.state().screens().tray.grid().bbox().unwrap();
    assert_eq!((after.x, after.y), (before.x + 1, before.y + 1));
    assert!(has_label(&h, "gen 0004"));

    // A stamp by name, then run / pause / speed.
    let s = ok(
        &mut h,
        &p,
        "tray.load_pattern",
        json!({"pattern": "Glider gun"}),
    );
    assert_eq!(s["population"], 36);
    let s = ok(
        &mut h,
        &p,
        "tray.control",
        json!({"action": "run", "speed": 30}),
    );
    assert_eq!(
        (s["running"].as_bool(), s["speed"].as_f64()),
        (Some(true), Some(30.0))
    );
    assert!(h.state().screens().tray.running());
    assert!(has_label(&h, "Pause"));
    let s = ok(&mut h, &p, "tray.control", json!({"action": "pause"}));
    assert_eq!(s["running"], false);

    // randomize with a seed is reproducible; clear empties; restart is the garden under Life.
    let a = ok(
        &mut h,
        &p,
        "tray.control",
        json!({"action": "randomize", "seed": 42}),
    );
    let b = ok(
        &mut h,
        &p,
        "tray.control",
        json!({"action": "randomize", "seed": 42}),
    );
    assert_eq!(a["rle"], b["rle"]);
    assert_eq!(a["note"], "Random soup from seed 42.");
    let s = ok(&mut h, &p, "tray.control", json!({"action": "clear"}));
    assert_eq!(s["population"], 0);
    assert!(has_label(&h, "Alive: 0"));
    let s = ok(&mut h, &p, "tray.control", json!({"action": "restart"}));
    assert_eq!(s["rule"], "B3/S23");
    assert_eq!(s["population"], h.state().screens().tray.stats().population);

    // A big pattern is cropped, and says so.
    let wide = "x = 60, y = 1\n60o!";
    let s = ok(&mut h, &p, "tray.load_pattern", json!({"pattern": wide}));
    assert_eq!(s["population"], 40);
    assert!(
        s["note"].as_str().unwrap().contains("cropped to 40 × 26"),
        "{s}"
    );
}

#[test]
fn screen_methods_switch_screens_unless_told_not_to() {
    let sock = TempSocket::new();
    let mut h = harness(&sock.path());
    let p = sock.path();
    ok(&mut h, &p, "loom.set", json!({"rule": 90, "show": false}));
    assert_eq!(h.state().route(), Route::Tray);
    assert_eq!(h.state().screens().loom.rule(), 90);
    ok(&mut h, &p, "loom.set", json!({"rule": 30}));
    assert_eq!(h.state().route(), Route::Loom);
    ok(&mut h, &p, "lab.get", Value::Null);
    assert_eq!(h.state().route(), Route::Loom, "reading doesn't switch");
}

#[test]
fn a_change_to_an_unvisited_screen_survives_its_first_visit() {
    // The real app autoplays: a screen starts running the first time it is shown, and the
    // Terrarium seeds its soups then. The bridge enters a screen before changing it.
    let sock = TempSocket::new();
    let opts = AppOptions {
        autoplay: true,
        ..AppOptions::test()
    };
    let mut h = harness_with(&sock.path(), opts);
    let p = sock.path();
    ok(&mut h, &p, "navigate", json!({"screen": "rule-lab"}));
    let kept = ok(
        &mut h,
        &p,
        "terrarium.keep",
        json!({"jars": [2], "show": false}),
    );
    ok(
        &mut h,
        &p,
        "loom.set",
        json!({"running": false, "show": false}),
    );
    ok(&mut h, &p, "navigate", json!({"screen": "terrarium"}));
    let t = &h.state().screens().terrarium;
    assert!(t.is_kept(1), "still kept after the first visit");
    assert_eq!(kept["jars"][1]["rule"], t.jars()[1].code);
    ok(&mut h, &p, "navigate", json!({"screen": "loom"}));
    assert!(!h.state().screens().loom.running(), "still paused");
}

#[test]
fn loom_methods() {
    let sock = TempSocket::new();
    let mut h = harness(&sock.path());
    let p = sock.path();
    let s = ok(
        &mut h,
        &p,
        "loom.set",
        json!({"rule": 90, "start": "single", "running": false, "step": 7, "rows": 8}),
    );
    assert_eq!(s["rule"], 90);
    assert_eq!(s["binary"], "01011010");
    assert_eq!(s["famous"]["name"], "Sierpiński");
    assert_eq!(s["row"], 8);
    assert_eq!(s["running"], false);
    let rows: Vec<&str> = s["rows"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r.as_str().unwrap())
        .collect();
    assert_eq!(rows.len(), 8);
    // Rule 90 from a single cell: 2^popcount(n) live cells in row n (Gould's sequence).
    let counts: Vec<usize> = rows.iter().map(|r| r.matches('#').count()).collect();
    assert_eq!(counts, [1, 2, 2, 4, 2, 4, 4, 8]);
    assert_eq!(h.state().screens().loom.row_count(), 8);
    assert_eq!(h.state().route(), Route::Loom);
    assert!(has_label(&h, "row 0008"));

    let s = ok(&mut h, &p, "loom.set", json!({"seed": 5}));
    assert_eq!(
        (s["start"].as_str(), s["seed"].as_u64()),
        (Some("random"), Some(5))
    );
    let s = ok(&mut h, &p, "loom.get", json!({"rows": 0}));
    assert!(s["rows"].as_array().unwrap().is_empty());
}

#[test]
fn terrarium_methods() {
    let sock = TempSocket::new();
    let mut h = harness(&sock.path());
    let p = sock.path();
    let s = ok(&mut h, &p, "terrarium.get", Value::Null);
    let codes: Vec<&str> = s["jars"]
        .as_array()
        .unwrap()
        .iter()
        .map(|j| j["rule"].as_str().unwrap())
        .collect();
    assert_eq!(
        codes,
        [
            "B3/S23",
            "B36/S23",
            "B3678/S34678",
            "B3/S12345",
            "B36/S125",
            "B368/S245"
        ]
    );
    assert_eq!(s["brood"], 1);

    let s = ok(&mut h, &p, "terrarium.keep", json!({"jars": [1, 3]}));
    assert_eq!(s["jars"][0]["kept"], true);
    assert_eq!(s["jars"][2]["kept"], true);
    assert_eq!(s["keep_note"], h.state().screens().terrarium.keep_note());
    assert!(has_label(&h, "2 kept as parents"));

    let s = ok(
        &mut h,
        &p,
        "terrarium.breed",
        json!({"mutation": 1, "ticks": 12}),
    );
    assert_eq!(s["brood"], 2);
    assert_eq!(s["mutation"], 1);
    assert_eq!(s["jars"][0]["rule"], "B3/S23", "kept jars carry over");
    assert_eq!(s["jars"][1]["rule"], "B3678/S34678");
    assert_eq!(
        s["lineage"][0]["parents"],
        json!(["B3/S23", "B3678/S34678"])
    );
    assert!(
        s["jars"]
            .as_array()
            .unwrap()
            .iter()
            .all(|j| j["kept"] == false)
    );
    assert!(
        s["note"].as_str().unwrap().starts_with("Bred brood 2 from"),
        "{s}"
    );
    assert!(has_label(&h, "brood 2"));
}

#[test]
fn rule_lab_methods() {
    let sock = TempSocket::new();
    let mut h = harness(&sock.path());
    let p = sock.path();
    let s = ok(&mut h, &p, "lab.get", Value::Null);
    assert_eq!(s["report"]["peak_text"], "221 @ 148");
    assert_eq!(
        s["report"]["verdict"],
        "Settles into a period-2 cycle at gen 388."
    );
    assert_eq!(s["spec"]["rule"]["code"], "B3/S23");
    assert_eq!(s["series"].as_array().unwrap().len(), 41);

    let script = s["script"].as_str().unwrap().replace("B3/S23", "B36/S23");
    let s = ok(&mut h, &p, "lab.set_script", json!({"script": script}));
    assert_eq!(s["spec"]["rule"]["code"], "B36/S23");
    assert_eq!(s["script"], script);
    assert_eq!(h.state().screens().rule_lab.spec().rule.code(), "B36/S23");
    assert_eq!(h.state().route(), Route::RuleLab);

    let s = ok(&mut h, &p, "lab.scrub", json!({"generation": 120}));
    assert_eq!(s["generation"], 120);
    assert_eq!(s["alive"], h.state().screens().rule_lab.alive());
    assert_eq!(s["rle"], h.state().screens().rule_lab.rle());
    let s = ok(&mut h, &p, "lab.scrub", json!({"generation": 2000}));
    assert_eq!(s["generation"], 400);
    assert!(s["note"].as_str().unwrap().contains("past the run"), "{s}");
}

#[test]
fn a_client_that_leaves_mid_screenshot_does_no_harm() {
    let sock = TempSocket::new();
    let mut h = harness(&sock.path());
    {
        use std::io::Write;
        let mut raw = std::os::unix::net::UnixStream::connect(sock.path()).unwrap();
        raw.write_all(b"{\"id\":1,\"method\":\"screenshot\",\"params\":{\"screen\":\"loom\"}}\n")
            .unwrap();
        h.step(); // the app takes the request…
    } // …and the client hangs up before the frame it waits for
    h.run_steps(5);
    let r = ok(&mut h, &sock.path(), "status", Value::Null);
    assert_eq!(r["screen"]["id"], "loom");
}

/// Whether the last frame asked eframe to paint even if the window is hidden.
fn asks_for_paint_while_hidden(h: &App) -> bool {
    h.output()
        .viewport_output
        .get(&egui::ViewportId::ROOT)
        .is_some_and(|v| {
            v.commands
                .contains(&egui::ViewportCommand::RequestPaintWhileHidden)
        })
}

#[test]
fn a_hidden_window_asks_for_a_frame_to_answer_in() {
    use std::io::{BufRead, BufReader, Write};
    let sock = TempSocket::new();
    let mut h = harness(&sock.path());
    h.step();
    assert!(!asks_for_paint_while_hidden(&h), "idle: no extra frames");

    // A request arrives while eframe runs only `App::logic`, as it does for a minimized or
    // covered window.
    let mut raw = std::os::unix::net::UnixStream::connect(sock.path()).unwrap();
    raw.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
    raw.write_all(b"{\"id\":1,\"method\":\"tray.get\"}\n")
        .unwrap();
    let ctx = h.ctx.clone();
    let t0 = Instant::now();
    while automata_playground::control::pending(h.state()) == 0 {
        assert!(
            t0.elapsed() < Duration::from_secs(5),
            "request never arrived"
        );
        automata_playground::control::logic(h.state_mut(), &ctx);
        thread::sleep(Duration::from_millis(2));
    }

    // The next frame carries `RequestPaintWhileHidden`, so eframe runs the UI and paints, and
    // the request is answered.
    h.step();
    assert!(asks_for_paint_while_hidden(&h));
    let mut line = String::new();
    BufReader::new(raw).read_line(&mut line).unwrap();
    let reply: Value = serde_json::from_str(&line).unwrap();
    assert_eq!(reply["id"], 1);
    assert_eq!(reply["result"]["rule"], "B3/S23");
    assert_eq!(automata_playground::control::pending(h.state()), 0);
}

#[test]
fn a_second_instance_starts_without_a_bridge() {
    let sock = TempSocket::new();
    let mut first = harness(&sock.path());
    let mut second = harness(&sock.path()); // logs "MCP bridge off", runs normally
    second.run_steps(2);
    let r = ok(&mut first, &sock.path(), "status", Value::Null);
    assert_eq!(r["pid"], std::process::id());
    // A socket path that can't work doesn't stop the app either.
    let mut bad = harness(Path::new("/dev/null/app.sock"));
    bad.run_steps(2);
}

#[test]
fn defaults_and_shared_identity() {
    assert!(
        AppOptions::test().mcp_socket.is_none(),
        "tests never touch the real socket"
    );
    assert!(
        automata_playground::smoke::app_options()
            .mcp_socket
            .is_none()
    );
    assert_eq!(automata_playground::APP_ID, ap_shared::APP_ID);
    assert_eq!(automata_playground::APP_NAME, ap_shared::APP_NAME);
    if std::env::var_os(ap_shared::SOCKET_ENV).is_none() {
        let p = AppOptions::default()
            .mcp_socket
            .expect("the real app listens");
        assert!(p.as_os_str().len() <= lab_mcp::bridge::MAX_SOCKET_PATH);
        assert!(p.to_string_lossy().contains("AutomataPlayground"));
    }
    // The bridge's screen table is the router's.
    assert_eq!(Route::ALL.len(), ap_shared::SCREENS.len());
    for (route, s) in Route::ALL.iter().zip(ap_shared::SCREENS) {
        assert_eq!(route.id(), s.id);
        assert_eq!(route.title(), s.label);
        assert_eq!(route.document(), s.document);
        assert_eq!(route.mockup(), s.mockup);
        assert_eq!(route.board_size(), (s.board.width, s.board.height));
        assert_eq!(
            route.mode().map(|m| m.label()),
            s.mode.map(|m| m.label),
            "{route:?}"
        );
        assert_eq!(
            route.shortcut_digit().map(|d| format!("⌘{d}")).as_deref(),
            s.shortcut
        );
        assert_eq!(
            automata_playground::control::screen_ref(*route),
            s.screen_ref()
        );
    }
}
