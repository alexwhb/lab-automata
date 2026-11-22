//! Loom · 1D (SCR-2, LOOM-1…6) through the real app: the rule keys, −1 / +1, presets,
//! Surprise me, First row, weaving at the set speed against the engine, Weave / Pause, New
//! thread, Share, the keyboard, accessibility, and renders at 1440 and 390.

mod common;

use ap_engine::elementary::{self, FAMOUS};
use ap_engine::{Elementary, WeaveStart};
use automata_playground::Route;
use automata_playground::screens::loom::{self, KEYS_LABEL, key_name};
use common::*;
use eframe::egui::{self, Key, accesskit::Role};

// ---- helpers --------------------------------------------------------------------------------

/// A 390 px wide app as tall as its page, so nothing needs the page to scroll (a column's own
/// scroll area keeps `scroll_to_me` from scrolling the page; see PROGRESS.md › Requests).
fn phone_full(route: Route) -> App {
    let mut h = app().at(route).phone().build();
    let need = h.state().content_height().ceil();
    h.set_size(egui::vec2(PHONE_W, need.max(PHONE_H)));
    settle(&mut h);
    h
}

fn loom(h: &App) -> &loom::State {
    &h.state().screens().loom
}

fn loom_mut(h: &mut App) -> &mut loom::State {
    &mut h.state_mut().screens_mut().loom
}

/// The rule panel's number + binary label.
fn number_label(rule: u8) -> String {
    format!("Rule {rule}, binary {rule:08b}")
}

/// The single-cell rows of `rule` after `n` more rows, computed with the engine directly.
fn expected_rows(rule: u8, n: usize) -> Vec<Vec<bool>> {
    let mut row = vec![false; 121];
    row[60] = true;
    let mut rows = vec![row];
    for _ in 0..n {
        let next = elementary::next_row(rule, rows.last().expect("a row"));
        rows.push(next);
    }
    let skip = rows.len().saturating_sub(72);
    rows.split_off(skip)
}

fn to_text(rows: &[Vec<bool>]) -> String {
    rows.iter()
        .map(|r| {
            let mut s: String = r.iter().map(|&c| if c { '█' } else { '·' }).collect();
            s.push('\n');
            s
        })
        .collect()
}

// ---- rule keys (LOOM-3) ---------------------------------------------------------------------

/// Each key flips exactly its bit, and the number, binary, blurb, head label and preset
/// highlight follow.
#[test]
fn keys_flip_exactly_their_bit() {
    let mut h = harness(Route::Loom);
    assert_eq!(loom(&h).rule(), 30);
    assert_eq!(
        child_labels(&h, KEYS_LABEL, Role::Group, Role::Button).len(),
        8,
        "eight keys in the group"
    );
    for p in (0..8u8).rev() {
        let before = loom(&h).rule();
        let on = (before >> p) & 1 == 1;
        assert_eq!(pressed(&h, &key_name(before, p)), Some(on), "key {p:03b}");
        click_button(&mut h, &key_name(before, p));
        let after = loom(&h).rule();
        assert_eq!(after, before ^ (1 << p), "key {p:03b} flips only bit {p}");
        assert_eq!(pressed(&h, &key_name(after, p)), Some(!on));
        assert!(
            has_label(&h, &number_label(after)),
            "{}",
            number_label(after)
        );
        assert!(has_label(&h, &Elementary(after).blurb()));
        assert!(has_label_containing(&h, &format!("; Rule {after}")));
    }
    // all eight flipped: 30 → !30
    assert_eq!(loom(&h).rule(), !30u8);
    assert!(has_label(&h, "Uncharted rule"));
    for f in FAMOUS {
        assert_eq!(
            pressed(&h, &format!("{} {}", f.rule, f.name)),
            Some(false),
            "no famous thread lit for rule 225"
        );
    }
    // back to Chaos with its preset
    click_button(&mut h, "30 Chaos");
    assert_eq!(pressed(&h, "30 Chaos"), Some(true));
    assert!(has_label(&h, "Chaos · Random-looking"));
}

/// A key press restarts the thread (LOOM-4).
#[test]
fn rule_changes_restart_the_thread() {
    let mut h = harness(Route::Loom);
    loom_mut(&mut h).step_n(20);
    settle(&mut h);
    assert!(has_label(&h, "row 0021"));
    click_button(&mut h, &key_name(30, 0));
    assert_eq!(loom(&h).row_count(), 1);
    assert!(has_label(&h, "row 0001"));
}

// ---- rule panel (LOOM-4/5) ------------------------------------------------------------------

/// −1 / +1 step the number and wrap 0 ↔ 255.
#[test]
fn minus_and_plus_wrap() {
    let mut h = harness(Route::Loom);
    click_button(&mut h, "Next rule");
    assert_eq!(loom(&h).rule(), 31);
    click_button(&mut h, "Previous rule");
    click_button(&mut h, "Previous rule");
    assert_eq!(loom(&h).rule(), 29);
    loom_mut(&mut h).set_rule(0);
    settle(&mut h);
    click_button(&mut h, "Previous rule");
    assert_eq!(loom(&h).rule(), 255);
    assert!(has_label(&h, &number_label(255)));
    click_button(&mut h, "Next rule");
    assert_eq!(loom(&h).rule(), 0);
    assert!(has_label(&h, "Rule 0, binary 00000000"));
}

/// The famous threads set the rule and light up; the blurb names them.
#[test]
fn presets_pick_famous_threads() {
    let mut h = harness(Route::Loom);
    for f in FAMOUS {
        let name = format!("{} {}", f.rule, f.name);
        click_button(&mut h, &name);
        assert_eq!(loom(&h).rule(), f.rule);
        assert_eq!(pressed(&h, &name), Some(true));
        assert!(has_label(&h, &format!("{} · {}", f.name, f.note)));
        let lit = FAMOUS
            .iter()
            .filter(|g| pressed(&h, &format!("{} {}", g.rule, g.name)) == Some(true))
            .count();
        assert_eq!(lit, 1);
    }
}

/// Surprise me changes the rule, the same way every run under the test seed.
#[test]
fn surprise_me_is_deterministic() {
    let picks = |h: &mut App| -> Vec<u8> {
        (0..3)
            .map(|_| {
                let before = loom(h).rule();
                click_button(h, "Surprise me");
                let after = loom(h).rule();
                assert_ne!(after, before, "Surprise me always changes the rule");
                after
            })
            .collect()
    };
    let mut a = harness(Route::Loom);
    let mut b = phone_full(Route::Loom);
    let first = picks(&mut a);
    assert_eq!(first, picks(&mut b), "seed 7 gives the same surprises");
    assert!(has_label(&a, &number_label(first[2])));
}

// ---- first row, speed, weaving (LOOM-1/6) ---------------------------------------------------

/// Single cell: one live cell in the middle. Random: a fresh half-full row on every press.
#[test]
fn first_row_single_or_random() {
    let mut h = harness(Route::Loom);
    assert_eq!(pressed(&h, "Single cell"), Some(true));
    let single = loom(&h).weave().newest().to_vec();
    assert_eq!(single.iter().filter(|&&c| c).count(), 1);
    assert!(single[60]);

    click_button(&mut h, "Random");
    assert_eq!(pressed(&h, "Random"), Some(true));
    assert_eq!(pressed(&h, "Single cell"), Some(false));
    assert!(matches!(loom(&h).start(), WeaveStart::Random(_)));
    let a = loom(&h).weave().newest().to_vec();
    let alive = a.iter().filter(|&&c| c).count();
    assert!((30..=91).contains(&alive), "about half alive: {alive}");
    click_button(&mut h, "Random");
    let b = loom(&h).weave().newest().to_vec();
    assert_ne!(a, b, "each press draws a new row");
    // New thread with a random start draws yet another row
    click_button(&mut h, "New thread");
    assert_ne!(loom(&h).weave().newest(), b.as_slice());

    click_button(&mut h, "Single cell");
    assert_eq!(loom(&h).weave().newest(), single.as_slice());
    assert_eq!(loom(&h).row_count(), 1);
}

/// The Speed slider sets rows per second (2–40) and shows "N rows/s".
#[test]
fn speed_slider() {
    let mut h = harness(Route::Loom);
    assert_eq!(loom(&h).speed(), 14.0);
    assert!(has_label(&h, "Speed: 14 rows/s"));
    get(&h, "Speed", Role::Slider).set_value_accesskit(20.0);
    settle(&mut h);
    assert_eq!(loom(&h).speed(), 20.0);
    assert!(has_label(&h, "Speed: 20 rows/s"));
    get(&h, "Speed", Role::Slider).set_value_accesskit(99.0);
    settle(&mut h);
    assert_eq!(loom(&h).speed(), 40.0);
    get(&h, "Speed", Role::Slider).decrement_accesskit();
    settle(&mut h);
    assert_eq!(loom(&h).speed(), 39.0);
}

/// Weave runs the thread at the set speed (each test frame is 0.1 s of loom time), Pause
/// stops it, and the counter follows.
#[test]
fn weave_and_pause_at_the_set_speed() {
    let mut h = harness(Route::Loom);
    loom_mut(&mut h).set_speed(20.0);
    click_button(&mut h, "Weave");
    assert!(loom(&h).running());
    get(&h, "Pause", Role::Button);
    let c0 = loom(&h).row_count();
    h.run_steps(5);
    assert_eq!(loom(&h).row_count(), c0 + 10, "20 rows/s × 0.5 s");
    click_button(&mut h, "Pause");
    assert!(!loom(&h).running());
    get(&h, "Weave", Role::Button);
    let c1 = loom(&h).row_count();
    h.run_steps(5);
    assert_eq!(loom(&h).row_count(), c1, "paused");
    assert!(has_label(&h, &format!("row {c1:04}")));
}

/// The fabric is the engine's: rule 90 from a single cell has 2^popcount(n) live cells in
/// row n (Gould's sequence), and every kept row matches `next_row`.
#[test]
fn fabric_matches_the_engine() {
    let mut h = harness(Route::Loom);
    click_button(&mut h, "90 Sierpiński");
    for n in 1..=60u32 {
        loom_mut(&mut h).step_n(1);
        let alive = loom(&h).weave().newest().iter().filter(|&&c| c).count();
        assert_eq!(alive, 1 << n.count_ones(), "rule 90 row {n}");
    }
    settle(&mut h);
    assert_eq!(loom(&h).row_count(), 61);
    assert_eq!(loom(&h).rows_text(), to_text(&expected_rows(90, 60)));
    get(
        &h,
        "Woven pattern, rule 90, 121 cells wide, 61 rows woven",
        Role::Image,
    );
    // past 72 rows only the newest 72 are kept
    click_button(&mut h, "30 Chaos");
    loom_mut(&mut h).step_n(99);
    settle(&mut h);
    assert_eq!(loom(&h).weave().len(), 72);
    assert_eq!(loom(&h).rows_text(), to_text(&expected_rows(30, 99)));
    assert!(has_label(&h, "row 0100"));
    assert!(has_label_containing(&h, "72 of 72 rows woven"));
}

/// New thread starts over at row 1.
#[test]
fn new_thread_restarts() {
    let mut h = harness(Route::Loom);
    loom_mut(&mut h).step_n(46);
    settle(&mut h);
    assert!(has_label(&h, "row 0047"));
    click_button(&mut h, "New thread");
    assert_eq!(loom(&h).row_count(), 1);
    assert!(has_label(&h, "row 0001"));
    assert_eq!(loom(&h).rule(), 30, "the rule stays");
}

/// Share copies "Rule N" and the visible rows as text, and says so.
#[test]
fn share_copies_the_fabric() {
    let mut h = harness(Route::Loom);
    click_button(&mut h, "110 Universal");
    loom_mut(&mut h).step_n(9);
    settle(&mut h);
    click_button(&mut h, "Share");
    let copied = h.state().last_copied().expect("copied").to_string();
    assert_eq!(copied, loom(&h).share_text());
    let mut lines = copied.lines();
    assert_eq!(lines.next(), Some("Rule 110"));
    let rows: Vec<&str> = lines.collect();
    assert_eq!(rows.len(), 10);
    assert!(rows.iter().all(|r| r.chars().count() == 121));
    assert!(rows[0].chars().filter(|&c| c == '█').count() == 1);
    assert_eq!(rows.concat().replace(['█', '·'], ""), "");
    assert!(toasts(&h).contains(&"Copied Rule 110 and 10 rows as text".to_string()));
}

/// Space toggles Weave / Pause (NAV-5); `S` pauses and weaves one row.
#[test]
fn keyboard_controls() {
    let mut h = harness(Route::Loom);
    press(&mut h, Key::Space);
    assert!(loom(&h).running());
    press(&mut h, Key::Space);
    assert!(!loom(&h).running());
    let c = loom(&h).row_count();
    press(&mut h, Key::S);
    assert_eq!(loom(&h).row_count(), c + 1);
    assert!(!loom(&h).running());
}

/// The keys are keyboard operable (A11Y-3): focus one and press Enter.
#[test]
fn keys_work_from_the_keyboard() {
    let mut h = harness(Route::Loom);
    let name = key_name(30, 7);
    get(&h, &name, Role::Button).focus();
    h.step();
    press(&mut h, Key::Enter);
    settle(&mut h);
    assert_eq!(loom(&h).rule(), 30 | 0x80);
}

/// The control API (for the MCP bridge) drives the same state the GUI shows.
#[test]
fn control_api() {
    let mut h = harness(Route::Loom);
    let s = loom_mut(&mut h);
    s.set_rule(184);
    s.set_start(WeaveStart::Random(42));
    s.step_n(3);
    s.set_running(true);
    settle(&mut h);
    assert_eq!(pressed(&h, "184 Traffic"), Some(true));
    assert_eq!(pressed(&h, "Random"), Some(true));
    get(&h, "Pause", Role::Button);
    assert!(loom(&h).row_count() > 4);
    assert!(loom(&h).describe_text().contains("rule 184"));
}

// ---- renders (DONE-1) -----------------------------------------------------------------------

/// At 1440 × 900 with rule 30 woven to row 47 (the reference's state) the page is close to
/// the mockup's rendered reference.
#[test]
fn renders_like_the_mockup_at_1440() {
    let mut h = harness(Route::Loom);
    loom_mut(&mut h).step_n(46);
    settle(&mut h);
    let img = render(&mut h);
    save(&img, &screenshot_dir().join("loom-1440.png"));
    let mock = load(&mockup_png(Route::Loom, 1440));
    // the stage (fabric, head, keys, caption) and the aside, under the header
    let stage = mean_diff(&img, &mock, Some((0, 102, 1120, 798)));
    let aside = mean_diff(&img, &mock, Some((1121, 57, 319, 843)));
    println!("loom 1440: stage |Δ| = {stage:.2}, aside |Δ| = {aside:.2}");
    assert!(stage < 2.5, "stage drifted: {stage:.2}");
    assert!(aside < 3.5, "aside drifted: {aside:.2}");
    // the keys sit on the reference's row (y ≈ 663–745) and span the head (x 160–960)
    let first = get(&h, &key_name(30, 7), Role::Button).rect();
    let last = get(&h, &key_name(30, 0), Role::Button).rect();
    assert!((first.top() - 662.0).abs() < 12.0, "{first:?}");
    assert!((first.left() - 160.0).abs() < 12.0 && (last.right() - 960.0).abs() < 12.0);
}

/// At 390 the panels stack, every key is on screen (a 4 × 2 grid) and the aside follows.
#[test]
fn works_at_390() {
    let mut h = phone_full(Route::Loom);
    for p in 0..8 {
        let r = get(&h, &key_name(30, p), Role::Button).rect();
        assert!(
            r.left() >= 0.0 && r.right() <= PHONE_W,
            "key {p:03b} off screen: {r:?}"
        );
        assert!(r.width() >= 44.0 && r.height() >= 44.0, "target {r:?}");
    }
    // keys and the aside are reachable and work
    click_button(&mut h, &key_name(30, 0));
    assert_eq!(loom(&h).rule(), 31);
    click_button(&mut h, "150 Lace");
    assert_eq!(loom(&h).rule(), 150);
    let aside = get(&h, "Rule and presets", Role::Region).rect();
    let stage = get(&h, "Loom", Role::Region).rect();
    assert!(aside.top() >= stage.bottom() - 0.5, "stacked");

    let mock = load(&mockup_png(Route::Loom, 390));
    let img = render_full(Route::Loom, PHONE_W, mock.height() as f32);
    save(&img, &screenshot_dir().join("loom-390.png"));
    // the aside (same content, same place within the page)
    let d = mean_diff(&img, &mock, Some((0, 1072, 390, 700)));
    println!("loom 390: aside |Δ| = {d:.2}");
    assert!(d < 3.0, "390 aside drifted: {d:.2}");
}

/// Extension used by `control_api`: the screen's `describe()` text.
trait Describe {
    fn describe_text(&self) -> String;
}

impl Describe for loom::State {
    fn describe_text(&self) -> String {
        automata_playground::Screen::describe(self)
    }
}
