//! Tray · 2D (SCR-1, TRAY-1…10) through the real app: pointer drawing through the tilted
//! perspective, the tools, cartridges and bits, the run controls, the readouts against the
//! engine, Share / paste / drop (TRAY-9), the keyboard cursor (TRAY-10), NAV-4 carry-overs,
//! the control API, and renders at 1440 and 390.

mod common;

use ap_bench::CellVisual;
use ap_engine::pattern::{self, library};
use ap_engine::presets::{CARTRIDGES, TRAY_H, TRAY_W, starting_garden};
use ap_engine::{Grid, LifeRule, Pattern};
use automata_playground::screens::tray::{self, StampChoice, Tool};
use automata_playground::{Carry, Route};
use common::*;
use eframe::egui::{self, Event, Key, OutputCommand, Pos2, accesskit::Role};
use egui_kittest::kittest::NodeT;
use std::sync::Arc;

// ---- helpers --------------------------------------------------------------------------------

fn tray(h: &App) -> &tray::State {
    &h.state().screens().tray
}

fn tray_mut(h: &mut App) -> &mut tray::State {
    &mut h.state_mut().screens_mut().tray
}

/// A 1440 × 900 Tray with an empty tray (the cleared state many tests start from).
fn cleared() -> App {
    let mut h = harness(Route::Tray);
    tray_mut(&mut h).clear();
    settle(&mut h);
    h
}

/// Where cell `(x, y)` is on screen, from the geometry the screen drew last frame.
fn at(h: &App, x: usize, y: usize) -> Pos2 {
    tray(h)
        .cell_center(x, y)
        .unwrap_or_else(|| panic!("cell ({x}, {y}) not drawn"))
}

/// Presses and releases the pointer on cell `(x, y)`.
fn click_cell(h: &mut App, x: usize, y: usize) {
    let p = at(h, x, y);
    h.hover_at(p);
    h.drag_at(p);
    h.drop_at(p);
    settle(h);
}

/// Presses on the first cell, moves through the others (one frame each) and releases.
fn drag_cells(h: &mut App, cells: &[(usize, usize)]) {
    let pts: Vec<Pos2> = cells.iter().map(|&(x, y)| at(h, x, y)).collect();
    h.hover_at(pts[0]);
    h.drag_at(pts[0]);
    for p in &pts[1..] {
        h.hover_at(*p);
    }
    h.drop_at(*pts.last().expect("a cell"));
    settle(h);
}

fn alive(h: &App) -> Vec<(usize, usize)> {
    tray(h).grid().alive_cells().collect()
}

fn alive_set(g: &Grid) -> Vec<(usize, usize)> {
    g.alive_cells().collect()
}

/// The cells `pattern` covers when stamped centred on `(x, y)` of the torus.
fn stamped(pattern: &Pattern, x: usize, y: usize) -> Vec<(usize, usize)> {
    let mut g = Grid::new(TRAY_W, TRAY_H);
    g.place(
        pattern,
        x as i64,
        y as i64,
        ap_engine::Anchor::Center,
        ap_engine::Boundary::Torus,
    );
    alive_set(&g)
}

fn grid_node_label(h: &App) -> String {
    tray(h).description()
}

/// A file dropped on the window (what eframe hands over on a drag-and-drop).
#[derive(Debug)]
struct Dropped {
    path: std::path::PathBuf,
    text: String,
}

impl egui::DroppedFile for Dropped {
    fn path(&self) -> &std::path::Path {
        &self.path
    }
    fn bytes(&self) -> Result<Vec<u8>, String> {
        Ok(self.text.as_bytes().to_vec())
    }
}

// ---- TRAY-3: drawing through the perspective ------------------------------------------------

/// A press on a cell — anywhere on the tilted tray, near or far — toggles exactly that cell,
/// and pressing it again (now lifted 6 px) toggles it back.
#[test]
fn clicking_a_cell_through_the_perspective_toggles_exactly_it() {
    let mut h = cleared();
    for (x, y) in [(0, 0), (39, 0), (20, 13), (0, 25), (39, 25), (7, 19)] {
        click_cell(&mut h, x, y);
        assert_eq!(alive(&h), [(x, y)], "pressing ({x}, {y})");
        assert_eq!(tray(&h).cell_visual(x, y), CellVisual::Live(1), "a newborn");
        click_cell(&mut h, x, y);
        assert!(alive(&h).is_empty(), "pressing ({x}, {y}) again");
    }
    // the hover highlight follows the pointer through the perspective
    let p = at(&h, 12, 4);
    h.hover_at(p);
    settle(&mut h);
    assert_eq!(tray(&h).cell_at(p), Some((12, 4)));
}

/// Dragging paints the pressed cell's new value along the stroke, filling the cells between
/// samples that skip ahead (a fast drag); a stroke that starts on a live cell erases.
#[test]
fn drag_paints_a_line_without_gaps() {
    let mut h = cleared();
    // samples jump 4–5 cells at a time
    drag_cells(&mut h, &[(3, 10), (7, 10), (12, 10), (16, 10)]);
    let want: Vec<(usize, usize)> = (3..=16).map(|x| (x, 10)).collect();
    assert_eq!(alive(&h), want);
    // diagonal strokes are filled too
    drag_cells(&mut h, &[(20, 2), (24, 6)]);
    for i in 0..=4 {
        assert!(tray(&h).grid().is_alive(20 + i, 2 + i), "diagonal {i}");
    }
    // starting on a live cell paints "dead"
    drag_cells(&mut h, &[(5, 10), (9, 10)]);
    for x in 5..=9 {
        assert!(!tray(&h).grid().is_alive(x, 10), "erased {x}");
    }
    assert!(tray(&h).grid().is_alive(4, 10) && tray(&h).grid().is_alive(10, 10));
    // the readouts follow edits (the newest history value is replaced)
    let pop = tray(&h).grid().population();
    assert_eq!(*tray(&h).history().last().unwrap(), pop);
    assert!(has_label(&h, &format!("Alive: {pop}")));
}

/// Erase: drag to clear, and pressing an empty cell doesn't draw.
#[test]
fn erase_tool_clears_cells() {
    let mut h = harness(Route::Tray);
    click_button(&mut h, "Erase");
    assert_eq!(tray(&h).current_tool(), Tool::Erase);
    assert!(has_label(&h, "Drag to erase"));
    // the pulsar's top row: (28..=30, 12) and (34..=36, 12)
    assert!(tray(&h).grid().is_alive(28, 12));
    let before = tray(&h).grid().population();
    drag_cells(&mut h, &[(28, 12), (36, 12)]);
    for x in 28..=36 {
        assert!(!tray(&h).grid().is_alive(x, 12), "{x}");
    }
    assert_eq!(tray(&h).grid().population(), before - 6);
    click_cell(&mut h, 2, 20);
    assert!(!tray(&h).grid().is_alive(2, 20), "Erase never draws");
}

/// Stamp: picking a stamp selects the Stamp tool; a click places it centred on the cell and
/// wraps around the torus at the edges.
#[test]
fn stamps_place_centred_and_wrap() {
    let mut h = cleared();
    click_button(&mut h, "Glider");
    assert_eq!(pressed(&h, "Stamp"), Some(true));
    assert_eq!(pressed(&h, "Glider"), Some(true));
    assert_eq!(pressed(&h, "Spaceship"), Some(false));
    assert!(has_label(&h, "Click to place Glider"));
    let glider = library::pattern("glider").unwrap();
    click_cell(&mut h, 10, 10);
    assert_eq!(alive(&h), stamped(&glider, 10, 10));
    assert!(tray(&h).grid().is_alive(11, 10) && tray(&h).grid().is_alive(9, 11));

    // at the corner it wraps onto the opposite edges
    tray_mut(&mut h).clear();
    click_button(&mut h, "Spaceship");
    let lwss = library::pattern("lwss").unwrap();
    settle(&mut h);
    click_cell(&mut h, 0, 0);
    let want = stamped(&lwss, 0, 0);
    assert_eq!(alive(&h), want);
    assert!(want.iter().any(|&(x, _)| x >= TRAY_W - 2), "wrapped left");
    assert!(want.iter().any(|&(_, y)| y >= TRAY_H - 2), "wrapped up");

    // every stamp tile selects its stamp
    for (i, s) in library::STAMPS.iter().enumerate() {
        click_button(&mut h, s.name);
        assert_eq!(tray(&h).stamp(), StampChoice::Library(i));
        assert!(has_label(&h, &format!("Click to place {}", s.name)));
    }
}

// ---- TRAY-4 / TRAY-6: rules -----------------------------------------------------------------

/// Every cartridge sets its rule; exactly the matching one is lit; the rule panel and the
/// tray's engraved code follow.
#[test]
fn cartridges_set_the_rule_and_light_up() {
    let mut h = harness(Route::Tray);
    assert_eq!(pressed(&h, "Life"), Some(true));
    for c in CARTRIDGES.iter().rev() {
        click_button(&mut h, c.name);
        assert_eq!(tray(&h).rule(), &c.rule, "{}", c.name);
        for other in CARTRIDGES.iter() {
            assert_eq!(
                pressed(&h, other.name),
                Some(other.id == c.id),
                "{} lit after picking {}",
                other.name,
                c.name
            );
        }
        assert!(has_label(&h, &format!("Rule: {}", c.name)));
        get(&h, &c.rule.code(), Role::Label);
        for n in 0..=8 {
            assert_eq!(
                pressed(&h, &format!("Born with {n}")),
                Some(c.rule.born_with(n))
            );
            assert_eq!(
                pressed(&h, &format!("Survives with {n}")),
                Some(c.rule.survives_with(n))
            );
        }
    }
}

/// The bit rows edit the rule: the code, the name ("Custom rule") and the lit cartridge
/// follow; the new rule is what the engine runs.
#[test]
fn bits_edit_the_rule() {
    let mut h = harness(Route::Tray);
    click_button(&mut h, "Born with 6");
    assert_eq!(tray(&h).rule().code(), "B36/S23");
    assert_eq!(pressed(&h, "HighLife"), Some(true));
    assert_eq!(pressed(&h, "Life"), Some(false));
    assert!(has_label(&h, "Rule: HighLife"));
    click_button(&mut h, "Survives with 3");
    assert_eq!(tray(&h).rule().code(), "B36/S2");
    get(&h, "B36/S2", Role::Label);
    assert!(has_label(&h, "Rule: Custom rule"));
    for c in &CARTRIDGES {
        assert_eq!(pressed(&h, c.name), Some(false), "{}", c.name);
    }
    assert_eq!(pressed(&h, "Survives with 3"), Some(false));
    // the engine runs the edited rule
    let mut g = starting_garden();
    g.step(
        &LifeRule::parse("B36/S2").unwrap(),
        ap_engine::Boundary::Torus,
    );
    press(&mut h, Key::S);
    assert_eq!(tray(&h).grid(), &g);
    // toggling back restores Life and its cartridge
    click_button(&mut h, "Born with 6");
    click_button(&mut h, "Survives with 3");
    assert_eq!(pressed(&h, "Life"), Some(true));
}

// ---- TRAY-7: run controls -------------------------------------------------------------------

/// Step pauses and advances one; Run advances at the set speed (frames are 0.1 s here);
/// Pause stops; Restart brings back the garden under Life; Clear empties; Random soup makes a
/// different 30 % soup each press.
#[test]
fn run_controls_drive_the_engine() {
    let mut h = harness(Route::Tray);
    let mut garden = starting_garden();
    click_button(&mut h, "Step");
    garden.step(&LifeRule::LIFE, ap_engine::Boundary::Torus);
    assert_eq!(tray(&h).grid(), &garden);
    assert_eq!(tray(&h).generation, 1);
    assert!(has_label(&h, "gen 0001"));

    // Run: 10 gen/s at 0.1 s per frame is one generation a frame
    click_button(&mut h, "Run");
    assert!(tray(&h).running());
    assert!(has_label(&h, "Pause"));
    let g0 = tray(&h).generation;
    h.run_steps(10);
    assert_eq!(tray(&h).generation, g0 + 10);
    // 30 gen/s: three a frame
    get(&h, "Speed", Role::Slider).set_value_accesskit(30.0);
    h.step();
    assert_eq!(tray(&h).speed(), 30.0);
    let g1 = tray(&h).generation;
    h.run_steps(4);
    assert_eq!(tray(&h).generation, g1 + 12);
    assert!(has_label(&h, "Speed: 30 gen/s"));
    // 1 gen/s: one every ten frames
    get(&h, "Speed", Role::Slider).set_value_accesskit(1.0);
    h.step();
    let g2 = tray(&h).generation;
    h.run_steps(20);
    assert_eq!(tray(&h).generation, g2 + 2);
    click_button(&mut h, "Pause");
    let g3 = tray(&h).generation;
    h.run_steps(5);
    assert_eq!(tray(&h).generation, g3, "paused");

    // Step while running pauses first
    tray_mut(&mut h).set_running(true);
    click_button(&mut h, "Step");
    assert!(!tray(&h).running());

    click_button(&mut h, "Born with 6");
    click_button(&mut h, "Restart");
    assert_eq!(tray(&h).grid(), &starting_garden());
    assert_eq!(tray(&h).rule(), &LifeRule::LIFE, "Restart brings back Life");
    assert_eq!(tray(&h).generation, 0);

    click_button(&mut h, "Clear");
    assert_eq!(tray(&h).grid().population(), 0);
    assert!(has_label(&h, "gen 0000 · 0 alive"));

    click_button(&mut h, "Random soup");
    let a = tray(&h).grid().clone();
    let density = a.population() as f64 / (TRAY_W * TRAY_H) as f64;
    assert!((0.22..0.38).contains(&density), "30 % soup: {density}");
    click_button(&mut h, "Random soup");
    assert_ne!(tray(&h).grid(), &a, "each press is a new soup");
    assert_eq!(tray(&h).generation, 0);
}

/// The slider also follows the keyboard and is clamped to 1–30.
#[test]
fn speed_slider_sets_gens_per_second() {
    let mut h = harness(Route::Tray);
    assert_eq!(tray(&h).speed(), 10.0);
    assert!(has_label(&h, "Speed: 10 gen/s"));
    get(&h, "Speed", Role::Slider).set_value_accesskit(99.0);
    settle(&mut h);
    assert_eq!(tray(&h).speed(), 30.0);
    get(&h, "Speed", Role::Slider).decrement_accesskit();
    settle(&mut h);
    assert_eq!(tray(&h).speed(), 29.0);
    assert!(has_label(&h, "Speed: 29 gen/s"));
}

// ---- TRAY-1 / TRAY-2: look --------------------------------------------------------------------

/// Ghost trails: a cell that just died shows trail 3 → 2 → 1 with trails on and nothing with
/// them off, and the tray renders differently.
#[test]
fn ghost_trails_toggle_changes_dying_cells() {
    let mut h = cleared();
    tray_mut(&mut h).load_pattern(&Pattern::from_rows(&["OOO"])); // a blinker at (19..=21, 13)
    press(&mut h, Key::S);
    assert_eq!(tray(&h).cell_visual(19, 13), CellVisual::Trail(3));
    assert_eq!(pressed(&h, "Ghost trails"), Some(true));
    let with = render(&mut h);
    click_button(&mut h, "Ghost trails");
    assert_eq!(pressed(&h, "Ghost trails"), Some(false));
    assert_eq!(tray(&h).cell_visual(19, 13), CellVisual::Empty);
    let without = render(&mut h);
    let r = tray(&h).cell_rect(19, 13).unwrap();
    let region = (
        r.left() as u32,
        r.top() as u32,
        r.width() as u32,
        r.height() as u32,
    );
    assert!(
        mean_diff(&with, &without, Some(region)) > 2.0,
        "the trail cell looks different"
    );
    // the engine still tracks trails; turning them back on shows the fading one
    press(&mut h, Key::S);
    click_button(&mut h, "Ghost trails");
    assert_eq!(
        tray(&h).cell_visual(19, 13),
        CellVisual::Live(1),
        "blinker is back"
    );
    assert_eq!(tray(&h).cell_visual(20, 12), CellVisual::Trail(3));
}

/// Top-down view animates the tilt to flat over 0.4 s (frames are 0.1 s) and back.
#[test]
fn top_down_view_animates_the_tilt() {
    let mut h = harness(Route::Tray);
    assert_eq!(tray(&h).tilt(), 1.0);
    let tilted = tray(&h).cell_rect(20, 0).unwrap();
    get(&h, "Top-down view", Role::Button).click();
    h.step();
    h.step();
    assert!(tray(&h).flat());
    let mid = tray(&h).tilt();
    assert!(mid > 0.0 && mid < 1.0, "animating: {mid}");
    settle(&mut h);
    settle(&mut h);
    assert_eq!(tray(&h).tilt(), 0.0);
    assert_eq!(pressed(&h, "Top-down view"), Some(true));
    // flat: the far row is as wide as the near one
    let (far, near) = (
        tray(&h).cell_rect(20, 0).unwrap(),
        tray(&h).cell_rect(20, 25).unwrap(),
    );
    assert!(
        (far.width() - near.width()).abs() < 0.05,
        "{far:?} {near:?}"
    );
    assert!(far.width() > tilted.width());
    // clicking still hits the right cell when flat
    tray_mut(&mut h).clear();
    settle(&mut h);
    click_cell(&mut h, 33, 3);
    assert_eq!(alive(&h), [(33, 3)]);
    click_button(&mut h, "Top-down view");
    settle(&mut h);
    settle(&mut h);
    assert_eq!(tray(&h).tilt(), 1.0);
}

// ---- TRAY-8: readouts ---------------------------------------------------------------------------

/// After 30 generations the chips, header, tiles, sparkline and description show the engine's
/// numbers (the mockup's gen 30: 90 alive, 27 born, 57 died) and the callout its mood.
#[test]
fn readouts_follow_the_engine() {
    let mut h = harness(Route::Tray);
    for _ in 0..30 {
        h.key_press(Key::S);
    }
    settle(&mut h);
    let s = tray(&h).stats();
    assert_eq!(
        (s.generation, s.population, s.born, s.died),
        (30, 90, 27, 57)
    );
    for label in [
        "gen 0030",
        "gen 0030 · 90 alive",
        "Generation: 30",
        "Alive: 90",
        "Born: 27",
        "Died: 57",
        "Click or drag to draw",
        "Rule: Life",
        "Population: last 120 gens",
    ] {
        assert!(has_label(&h, label), "missing {label:?}");
    }
    let hist = tray(&h).history().to_vec();
    assert_eq!(hist.len(), 31);
    assert_eq!(*hist.last().unwrap(), 90);
    let spark = get(&h, "Population over the last 120 generations", Role::Image);
    assert_eq!(
        spark.value().as_deref(),
        Some(
            format!(
                "31 generations, latest 90, peak {}",
                hist.iter().max().unwrap()
            )
            .as_str()
        )
    );
    // the callout is the classifier's verdict
    let mood = ap_engine::Mood::classify(&hist);
    assert!(has_label(&h, mood.title()) && has_label(&h, mood.text()));
    assert_eq!(s.mood_title, mood.title());
    // A11Y-4: the tray's description
    let d = get(
        &h,
        "Cell tray, 40 by 26, generation 30, 90 alive",
        Role::Grid,
    );
    assert!(
        d.accesskit_node()
            .description()
            .is_some_and(|t| t.contains("PETRI TRAY · 40 × 26 · TORUS") && t.contains("B3/S23"))
    );
    assert_eq!(
        h.state().screens().get_ref(Route::Tray).describe(),
        grid_node_label(&h)
    );
}

/// The mood callout: Extinct on an empty tray, Settled once a still life has held 12
/// generations, Oscillating for a blinker... except a blinker's population is constant, so
/// it reads Settled too — exactly the mockup's classifier.
#[test]
fn mood_callout_reads_the_history() {
    let mut h = cleared();
    assert!(has_label(&h, "Extinct"));
    assert!(has_label(&h, "Everything died. Stamp or draw to restart."));
    tray_mut(&mut h).load_pattern(&Pattern::from_rows(&["OO", "OO"]));
    tray_mut(&mut h).step_n(12);
    settle(&mut h);
    assert!(has_label(&h, "Settled"));
    assert!(has_label(
        &h,
        "Only still lifes left. Try hitting one with a glider."
    ));
    // a soup under Seeds explodes: Booming
    tray_mut(&mut h).set_rule(LifeRule::parse("B2/S").unwrap());
    tray_mut(&mut h).load_pattern(&Pattern::from_rows(&["O.O"]));
    tray_mut(&mut h).step_n(21);
    settle(&mut h);
    let mood = tray(&h).stats().mood_title;
    assert!(has_label(&h, mood), "{mood}");
}

// ---- TRAY-9: Share, paste, drop ---------------------------------------------------------------

/// Share copies the whole tray as RLE with `rule =` (through egui's clipboard command) and
/// says so; the RLE parses back to the same cells.
#[test]
fn share_copies_the_tray_as_rle() {
    let mut h = harness(Route::Tray);
    click_button(&mut h, "Born with 6");
    tray_mut(&mut h).step_n(17);
    settle(&mut h);
    reveal(&mut h, "Share", Some(Role::Button));
    get(&h, "Share", Role::Button).click();
    h.step();
    let copied = h
        .output()
        .platform_output
        .commands
        .iter()
        .find_map(|c| match c {
            OutputCommand::CopyText(t) => Some(t.clone()),
            _ => None,
        })
        .expect("Share copies to the clipboard");
    settle(&mut h);
    assert_eq!(h.state().last_copied(), Some(copied.as_str()));
    assert!(toasts(&h).iter().any(|t| t == "Copied the tray as RLE"));
    let file = pattern::parse_any(&copied).expect("valid RLE");
    assert_eq!(file.rule.as_deref(), Some("B36/S23"));
    assert_eq!(
        (file.pattern.width(), file.pattern.height()),
        (TRAY_W, TRAY_H)
    );
    let mut back = Grid::new(TRAY_W, TRAY_H);
    back.place(
        &file.pattern,
        0,
        0,
        ap_engine::Anchor::TopLeft,
        ap_engine::Boundary::Torus,
    );
    assert_eq!(alive_set(&back), alive(&h));
}

/// ⌘V with RLE (or plaintext) on the clipboard arms it as a stamp — an extra selected stamp
/// tile and the hint "Click to place <name>" — and a click places it centred.
#[test]
fn pasting_a_pattern_arms_a_stamp() {
    let mut h = cleared();
    h.event(Event::Paste(
        "#N Beacon\nx = 4, y = 4, rule = B3/S23\n2o$2o$2b2o$2b2o!".into(),
    ));
    settle(&mut h);
    assert_eq!(tray(&h).stamp(), StampChoice::Custom);
    assert_eq!(tray(&h).current_tool(), Tool::Stamp);
    assert_eq!(pressed(&h, "Beacon"), Some(true), "an extra stamp tile");
    assert_eq!(pressed(&h, "Glider"), Some(false));
    assert!(has_label(&h, "Click to place Beacon"));
    assert!(toasts(&h).iter().any(|t| t.contains("Beacon")));
    click_cell(&mut h, 30, 20);
    let beacon = Pattern::from_rows(&["OO..", "OO..", "..OO", "..OO"]);
    assert_eq!(alive(&h), stamped(&beacon, 30, 20));

    // plaintext works too; nameless patterns get a generic name
    h.event(Event::Paste(".O.\n..O\nOOO".into()));
    settle(&mut h);
    assert!(has_label(&h, "Click to place Pasted pattern"));
    // picking a library stamp, then the custom tile again
    click_button(&mut h, "Acorn");
    assert_eq!(tray(&h).stamp(), StampChoice::Library(5));
    click_button(&mut h, "Pasted pattern");
    assert_eq!(tray(&h).stamp(), StampChoice::Custom);

    // garbage is refused with a toast and changes nothing
    h.event(Event::Paste("x = 3, y = 1\nzz!".into()));
    settle(&mut h);
    assert!(toasts(&h).iter().any(|t| t.starts_with("Couldn't read")));
    assert_eq!(tray(&h).custom_stamp().unwrap().name, "Pasted pattern");
}

/// A dropped `.cells` / `.rle` file arms its pattern under the file's name.
#[test]
fn dropping_a_pattern_file_arms_it() {
    let mut h = cleared();
    let file: egui::DroppedFileHandle = Arc::new(Dropped {
        path: "/tmp/two-blinkers.cells".into(),
        text: "! no name\nOOO.OOO\n".into(),
    });
    h.input_mut().dropped_files.push(file);
    settle(&mut h);
    let custom = tray(&h).custom_stamp().expect("armed");
    assert_eq!(custom.name, "two-blinkers");
    assert_eq!(custom.pattern.population(), 6);
    assert!(has_label(&h, "Click to place two-blinkers"));
}

// ---- TRAY-10: keyboard ----------------------------------------------------------------------

/// The tray is focusable: arrows move the cursor (wrapping), Space / Enter apply the tool at
/// it, and Space doesn't toggle Run / Pause while the tray has focus (it does otherwise).
#[test]
fn keyboard_cursor_draws() {
    let mut h = cleared();
    let label = grid_node_label(&h);
    get(&h, &label, Role::Grid).focus();
    settle(&mut h);
    assert_eq!(tray(&h).cursor(), (20, 13));
    press(&mut h, Key::ArrowRight);
    press(&mut h, Key::ArrowRight);
    press(&mut h, Key::ArrowDown);
    assert_eq!(tray(&h).cursor(), (22, 14));
    press(&mut h, Key::Space);
    settle(&mut h);
    assert_eq!(alive(&h), [(22, 14)]);
    assert!(!tray(&h).running(), "Space drew instead of running");
    press(&mut h, Key::Enter);
    settle(&mut h);
    assert!(alive(&h).is_empty(), "Enter toggles it back");
    // the cursor wraps around the torus
    for _ in 0..15 {
        h.key_press(Key::ArrowUp);
    }
    settle(&mut h);
    assert_eq!(tray(&h).cursor(), (22, 25));
    // Stamp at the cursor
    click_button(&mut h, "Glider");
    let label = grid_node_label(&h);
    get(&h, &label, Role::Grid).focus();
    settle(&mut h);
    press(&mut h, Key::Enter);
    settle(&mut h);
    let glider = library::pattern("glider").unwrap();
    assert_eq!(alive(&h), stamped(&glider, 22, 25));
    // with nothing focused, Space is Run / Pause again
    h.state_mut().screens_mut().tray.set_running(false);
    h.ctx
        .memory_mut(|m| m.surrender_focus(m.focused().unwrap()));
    settle(&mut h);
    press(&mut h, Key::Space);
    assert!(tray(&h).running());
}

// ---- NAV-4: carry-overs -----------------------------------------------------------------------

/// A Rule-lab payload sets the rule and loads the pattern centred on a cleared tray; a rule
/// the Tray can't parse is refused with a toast.
#[test]
fn receives_a_rule_and_pattern_from_the_rule_lab() {
    let mut h = harness(Route::RuleLab);
    h.state_mut().navigate_with(
        Route::Tray,
        Carry::rule("B36/S23", Route::RuleLab).with_rle("x = 3, y = 3, rule = B36/S23\nbo$2bo$3o!"),
    );
    settle(&mut h);
    assert_eq!(route(&h), Route::Tray);
    assert_eq!(tray(&h).rule().code(), "B36/S23");
    assert_eq!(pressed(&h, "HighLife"), Some(true));
    let glider = library::pattern("glider").unwrap();
    assert_eq!(alive(&h), stamped(&glider, TRAY_W / 2, TRAY_H / 2));
    assert_eq!(tray(&h).generation, 0);
    assert!(toasts(&h).iter().any(|t| t.contains("B36/S23")));

    // a 64-wide Rule-lab frame is cropped to the tray instead of wrapping onto itself
    let wide = Pattern::new(64, 1, (0..64).map(|x| (x, 0)));
    let rle = pattern::rle::write(&wide, Some(&LifeRule::LIFE), None);
    h.state_mut().navigate_with(
        Route::Tray,
        Carry::rule("B3/S23", Route::RuleLab).with_rle(rle),
    );
    settle(&mut h);
    assert_eq!(tray(&h).grid().population(), TRAY_W);
    assert!(toasts(&h).iter().any(|t| t.contains("cropped")));

    // rule only: the cells stay
    let before = tray(&h).grid().clone();
    h.state_mut()
        .navigate_with(Route::Tray, Carry::rule("B3/S12345", Route::Terrarium));
    settle(&mut h);
    assert_eq!(tray(&h).rule().code(), "B3/S12345");
    assert_eq!(tray(&h).grid(), &before);

    h.state_mut()
        .navigate_with(Route::Tray, Carry::rule("hello", Route::Terrarium));
    settle(&mut h);
    assert_eq!(tray(&h).rule().code(), "B3/S12345", "unchanged");
    assert!(toasts(&h).iter().any(|t| t.contains("can't run hello")));
}

// ---- control API ------------------------------------------------------------------------------

/// The methods the MCP bridge drives.
#[test]
fn control_api() {
    let mut h = harness(Route::Tray);
    let t = tray_mut(&mut h);
    t.set_rule(LifeRule::parse("B36/S23").unwrap());
    assert_eq!(t.rule().code(), "B36/S23");
    let rep = library::pattern("rep").unwrap();
    assert!(!t.load_pattern(&rep));
    assert_eq!(t.grid().population(), rep.population());
    t.step_n(12);
    assert_eq!(t.stats().generation, 12);
    let rle = t.rle();
    assert!(rle.contains("rule = B36/S23"));
    t.set_running(true);
    assert!(t.running());
    t.set_running(false);
    t.randomize(42);
    let soup = t.grid().clone();
    t.randomize(42);
    assert_eq!(t.grid(), &soup, "seeded");
    t.clear();
    assert_eq!(t.stats().population, 0);
    assert_eq!(t.stats().mood_title, "Extinct");
    t.restart();
    assert_eq!(t.grid(), &starting_garden());
    assert_eq!(t.rule(), &LifeRule::LIFE);
    settle(&mut h);
    assert!(has_label(&h, "gen 0000"));
}

// ---- renders ------------------------------------------------------------------------------------

/// The Tray renders at 1440 × 900 and 390 wide without panics; at 390 the tray is zoomed to
/// fit the stage and still hit-tests. With `AP_COMPARE=1` it also writes gen-30 renders next
/// to the mockup (`target/compare/Tray-*-gen30.png`, the state the references show).
#[test]
fn renders_at_1440_and_390() {
    let compare = std::env::var_os("AP_COMPARE").is_some_and(|v| v == "1");
    let mut h = harness(Route::Tray);
    // running like the reference, landing on gen 30 when the frame is rendered (settle runs
    // four frames, render two, one generation each at 10 gen/s)
    tray_mut(&mut h).step_n(23);
    tray_mut(&mut h).set_running(true);
    settle(&mut h);
    let img = render(&mut h);
    assert_eq!((img.width(), img.height()), (1440, 900));
    tray_mut(&mut h).set_running(false);
    if compare {
        save_compare(&img, Route::Tray, 1440);
    }

    let mut p = app().at(Route::Tray).phone().build();
    tray_mut(&mut p).clear();
    settle(&mut p);
    let grid_rect = tray(&p).cell_rect(39, 25).unwrap();
    assert!(
        grid_rect.right() <= PHONE_W,
        "the tray fits 390: {grid_rect:?}"
    );
    let label = grid_node_label(&p);
    reveal(&mut p, &label, Some(Role::Grid));
    click_cell(&mut p, 39, 25);
    click_cell(&mut p, 0, 0);
    assert_eq!(alive(&p), [(0, 0), (39, 25)]);
    let need = p.state().content_height();
    p.set_size(egui::vec2(PHONE_W, need));
    tray_mut(&mut p).restart();
    tray_mut(&mut p).step_n(22);
    tray_mut(&mut p).set_running(true);
    settle(&mut p);
    let img = render(&mut p);
    tray_mut(&mut p).set_running(false);
    assert_eq!(img.width(), PHONE_W as u32);
    if compare {
        save_compare(&img, Route::Tray, 390);
    }
}

fn save_compare(img: &image::RgbaImage, route: Route, width: u32) {
    let mock = load(&mockup_png(route, width));
    let h = img.height().max(mock.height());
    let mut out = image::RgbaImage::from_pixel(
        img.width() + 8 + mock.width(),
        h,
        image::Rgba([128, 128, 128, 255]),
    );
    image::imageops::overlay(&mut out, img, 0, 0);
    image::imageops::overlay(&mut out, &mock, (img.width() + 8) as i64, 0);
    let path = repo().join(format!("target/compare/Tray-{width}-gen30.png"));
    save(&out, &path);
    println!("compare: {}", path.display());
}
