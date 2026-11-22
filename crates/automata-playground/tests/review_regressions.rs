//! Regressions for the independent review's findings in the app (screens' models, the screens
//! and the MCP bridge). Each test names its finding; the reviewer's reproductions are ported
//! as they were, against the real app in egui_kittest and a real bridge client.

use ap_engine::{RuleScript, RunSpec, Seed};
use automata_playground::control;
use automata_playground::screens::rule_lab::model::Lab;
use automata_playground::{AppOptions, AutomataApp, Carry, Route};
use eframe::egui::{self, Key, Modifiers, accesskit::Role};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use lab_mcp::bridge::{BridgeClient, BridgeError};
use serde_json::{Value, json};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::thread;
use std::time::{Duration, Instant};

type App = Harness<'static, AutomataApp>;

// ---- harness --------------------------------------------------------------------------------

/// A private temp dir with a socket path in it, removed on drop.
struct TempSocket {
    dir: PathBuf,
}

impl TempSocket {
    fn new() -> Self {
        static N: AtomicU32 = AtomicU32::new(0);
        let dir = std::env::temp_dir().join(format!(
            "ap-review-{}-{}",
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

fn build(opts: AppOptions, step_dt: f32) -> App {
    let mut h = Harness::builder()
        .with_size(egui::vec2(1440.0, 900.0))
        .with_pixels_per_point(1.0)
        .with_max_steps(64)
        .with_step_dt(step_dt)
        .wgpu()
        .build_eframe(move |cc| AutomataApp::new(cc, opts));
    h.run_steps(3);
    h
}

/// The app at `route` (paused, seed 7), no bridge.
fn app_at(route: Route) -> App {
    build(AppOptions::at(route), 1.0 / 60.0)
}

/// The app with its bridge on `socket`.
fn bridged(socket: &Path) -> App {
    let opts = AppOptions {
        mcp_socket: Some(socket.to_path_buf()),
        ..AppOptions::test()
    };
    build(opts, 1.0 / 60.0)
}

/// Sends one raw request line and returns the stream to read its reply from.
fn send(socket: &Path, id: u64, method: &str, params: Value) -> UnixStream {
    let mut s = UnixStream::connect(socket).unwrap();
    s.set_read_timeout(Some(Duration::from_secs(20))).unwrap();
    let line = json!({"id": id, "method": method, "params": params}).to_string() + "\n";
    s.write_all(line.as_bytes()).unwrap();
    s
}

/// Pulls arrived requests into the app's inbox (no frame) until `n` are pending.
fn wait_pending(h: &mut App, n: usize) {
    let ctx = h.ctx.clone();
    let t0 = Instant::now();
    while control::pending(h.state()) < n {
        assert!(
            t0.elapsed() < Duration::from_secs(5),
            "request never arrived"
        );
        control::logic(h.state_mut(), &ctx);
        thread::sleep(Duration::from_millis(2));
    }
}

fn read_reply(s: UnixStream) -> Value {
    let mut line = String::new();
    BufReader::new(s).read_line(&mut line).unwrap();
    serde_json::from_str(&line).unwrap()
}

/// Calls the app from another thread, stepping frames until the reply arrives.
fn call(h: &mut App, socket: &Path, method: &str, params: Value) -> Result<Value, BridgeError> {
    let (socket, method) = (socket.to_path_buf(), method.to_owned());
    let client = thread::spawn(move || {
        BridgeClient::connect(&socket)?.call(&method, params, Duration::from_secs(20))
    });
    let t0 = Instant::now();
    while !client.is_finished() {
        assert!(t0.elapsed() < Duration::from_secs(25), "no reply");
        h.step();
        thread::sleep(Duration::from_millis(2));
    }
    client.join().unwrap()
}

// ---- Rule lab: background runs (findings 1, 9, 10) -------------------------------------------

/// A spec well over the synchronous budget (256 × 256 × 2000 cell-generations).
fn big_spec(gens: usize) -> RunSpec {
    RunSpec {
        w: 256,
        h: 256,
        gens,
        seed: Seed::MOCKUP_SOUP,
        ..RunSpec::default()
    }
}

fn big_script(gens: usize) -> String {
    RuleScript::from_spec("glider-garden.rule", &big_spec(gens)).to_text()
}

/// Types `text` into the Rule lab's script editor (select all, then type), one frame.
fn type_script(h: &mut App, text: &str) {
    let editor = h.get_by_role_and_label(Role::MultilineTextInput, "Rule script");
    editor.scroll_to_me();
    h.run_steps(2);
    h.get_by_role_and_label(Role::MultilineTextInput, "Rule script")
        .focus();
    h.step();
    h.key_press_modifiers(Modifiers::COMMAND, Key::A);
    h.step();
    h.get_by_role_and_label(Role::MultilineTextInput, "Rule script")
        .type_text(text);
    h.step();
}

/// Finding 1 (model): `set_script` with the text of the run computing in the background
/// returned before the run (the spec said 256 × 256, the run was still the 64 × 40 one).
#[test]
fn lab_set_script_waits_for_the_pending_run() {
    let mut lab = Lab::new();
    let big = big_script(2000);
    lab.edit_text(big.clone());
    assert!(lab.is_running());
    lab.set_script(&big).unwrap();
    assert!(!lab.is_running(), "set_script returned before the run");
    assert_eq!(lab.run().spec().w, 256);
    assert_eq!(lab.report(), &ap_engine::run(&big_spec(2000)).report());
}

/// Finding 1 (bridge): a big script typed into the editor goes to a worker; `lab.get` in the
/// next frame answered with the new spec but the old run's report, series and RLE, and
/// `lab.scrub 1500` clamped to the old run's 400.
#[test]
fn lab_bridge_calls_wait_for_a_typed_big_run() {
    let sock = TempSocket::new();
    let mut h = bridged(&sock.path());
    h.state_mut().navigate(Route::RuleLab);
    h.run_steps(3);
    type_script(&mut h, &big_script(2000));
    assert!(h.state().screens().rule_lab.lab().is_running());

    let a = send(&sock.path(), 1, "lab.get", json!({}));
    wait_pending(&mut h, 1);
    h.step();
    let r = read_reply(a);
    let res = &r["result"];
    let want = ap_engine::run(&big_spec(2000));
    let report = want.report();
    assert_eq!(res["spec"]["width"], 256);
    assert_eq!(res["report"]["peak_text"], report.peak_text());
    assert_eq!(res["report"]["final_population"], report.final_pop);
    let g = res["generation"].as_u64().unwrap() as usize;
    assert_eq!(res["rle"], want.rle(g).unwrap());
    assert_eq!(
        res["series"].as_array().unwrap().last().unwrap()[1],
        report.final_pop
    );

    // scrubbing past the old run's 400 while the next typed run computes
    type_script(&mut h, &big_script(1999));
    assert!(h.state().screens().rule_lab.lab().is_running());
    let r = call(
        &mut h,
        &sock.path(),
        "lab.scrub",
        json!({"generation": 1500}),
    )
    .unwrap();
    assert_eq!(r["generation"], 1500, "{r}");
    assert_eq!(r["note"], Value::Null);
}

/// Finding 1 (GUI): "Open in Playground" and Copy RLE right after a big edit carried the old
/// run's cells; while the run computes, the analysis shows that it is running rather than the
/// old run's numbers under the new spec.
#[test]
fn lab_actions_use_the_new_run_and_the_wait_is_shown() {
    let mut h = app_at(Route::RuleLab);
    let old_rle = h.state().screens().rule_lab.rle();
    type_script(&mut h, &big_script(2000));
    let lab = h.state().screens().rule_lab.lab();
    assert!(lab.is_running());
    // the old run's numbers aren't shown as the new spec's
    let old_peak = lab.report().peak_text();
    assert!(
        h.query_by_label_contains(&format!("Peak population: {old_peak}"))
            .is_none(),
        "the old run's peak {old_peak} is still shown"
    );
    assert!(
        h.query_by_label_contains("running").is_some(),
        "no running state shown"
    );
    let summary = automata_playground::Screen::describe(&h.state().screens().rule_lab);
    assert!(summary.contains("Running 256 × 256"), "{summary}");

    // Open in Playground while it computes carries the new run's cells
    h.get_by_label("Open in Playground").click();
    h.run_steps(4);
    let carried = h.state().screens().tray.received.clone().expect("carried");
    let g = h.state().screens().rule_lab.generation();
    assert_eq!(carried.rle, ap_engine::run(&big_spec(2000)).rle(g));
    assert_ne!(carried.rle.as_deref(), Some(old_rle.as_str()));
}

/// Finding 9: Analyse in Rule lab into a lab holding a big spec blocked the frame on the run.
#[test]
fn receiving_a_rule_does_not_block_on_a_big_spec() {
    let mut h = app_at(Route::Terrarium);
    h.state_mut()
        .screens_mut()
        .rule_lab
        .set_script(&big_script(2000))
        .unwrap();
    h.run_steps(2);
    h.state_mut()
        .navigate_with(Route::RuleLab, Carry::rule("B36/S23", Route::Terrarium));
    let t = Instant::now();
    h.step();
    let took = t.elapsed();
    let lab = &mut h.state_mut().screens_mut().rule_lab;
    assert_eq!(lab.spec().rule.code(), "B36/S23");
    assert!(lab.lab().is_running(), "the run went to the worker");
    assert!(took < Duration::from_millis(250), "the frame took {took:?}");
    lab.finish();
    assert_eq!(lab.lab().run().spec().rule.code(), "B36/S23");
}

/// Finding 10: every quick edit of a big spec left its worker computing to the end (10 edits
/// = 10 threads × ~0.6 s CPU × 16 MB); superseded runs now stop within a generation.
#[test]
fn superseded_background_runs_are_cancelled() {
    let mut lab = Lab::new();
    for g in (1990..2000).rev() {
        lab.edit_text(big_script(g));
    }
    assert!(lab.is_running());
    let t0 = Instant::now();
    while lab.workers() > 1 && t0.elapsed() < Duration::from_secs(3) {
        thread::sleep(Duration::from_millis(5));
    }
    let took = t0.elapsed();
    assert!(
        took < Duration::from_millis(200),
        "{} workers still running after {took:?}",
        lab.workers()
    );
    lab.finish();
    assert_eq!(lab.gens(), 1990, "the latest edit's run");
    let t0 = Instant::now();
    while lab.workers() > 0 && t0.elapsed() < Duration::from_secs(1) {
        thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(lab.workers(), 0);
}

// ---- Rule lab: BOM (finding 11) -------------------------------------------------------------

/// Finding 11: a pasted script starting with a byte-order mark parsed, but the first directive
/// wasn't recognised when a control patched the text, so the text was replaced by the
/// canonical script and the user's lines were lost.
#[test]
fn a_leading_bom_survives_a_control_change() {
    let mut lab = Lab::new();
    let body: Vec<&str> = lab.text().lines().skip(1).collect();
    let text = format!("\u{feff}{}", body.join("\n")).replace(".end", "* my note\n.end");
    lab.edit_text(text);
    assert!(lab.errors().is_empty(), "{:?}", lab.errors());
    let mut rule = lab.spec().rule;
    rule.toggle_birth(6);
    lab.set_rule(rule, false);
    assert!(lab.text().contains("* my note"), "{}", lab.text());
    assert!(lab.text().starts_with(".rule"), "{:?}", lab.text());
    assert_eq!(lab.text().matches(".rule").count(), 1, "{}", lab.text());
    assert_eq!(lab.spec().rule.code(), "B36/S23");
    // set_script likewise
    let mut lab = Lab::new();
    let canonical = lab.text().to_string();
    lab.set_script(&format!("\u{feff}{canonical}")).unwrap();
    assert_eq!(lab.text(), canonical);
    assert_eq!(
        automata_playground::screens::rule_lab::model::directive_of("\u{feff}.rule x"),
        Some(".rule")
    );
}
