//! Side-by-side `[app | mockup]` images of every screen, for human and agent review (not a
//! pixel-diff gate). Run through `scripts/compare-mockup.sh`, which sets `AP_COMPARE=1`;
//! without it the render test does nothing.
//!
//! For each route it renders the app at the mockup's board size (`Route::board_size`; the
//! Series screen gets the header on top of its 640 px board, which the mockup leaves out) and
//! at 390 px wide (as tall as the reference or the app's content, whichever is taller), in
//! the reference's state (the Tray running at gen 30, the Loom weaving at row 47, the
//! Terrarium running), puts the reference from `docs/design/screenshots/` next to it and
//! writes `target/compare/<Screen>-1440.png` / `<Screen>-390.png`.
//! `AP_COMPARE_ONLY=series,tray` limits it to some routes.
//!
//! `screens_stay_close_to_their_mockups` always runs: the mean channel difference of each
//! screen's stable regions (header, asides, deterministic benches) must stay under its bound.

mod common;

use automata_playground::Route;
use common::*;
use eframe::egui::accesskit::Role;
use image::{Rgba, RgbaImage};

/// Gap between the two images, filled with mid grey so the seam is obvious.
const SEAM: u32 = 8;

fn side_by_side(app: &RgbaImage, mockup: &RgbaImage) -> RgbaImage {
    let h = app.height().max(mockup.height());
    let w = app.width() + SEAM + mockup.width();
    let mut out = RgbaImage::from_pixel(w, h, Rgba([128, 128, 128, 255]));
    image::imageops::overlay(&mut out, app, 0, 0);
    image::imageops::overlay(&mut out, mockup, (app.width() + SEAM) as i64, 0);
    out
}

fn wanted(route: Route) -> bool {
    match std::env::var("AP_COMPARE_ONLY") {
        Ok(list) if !list.trim().is_empty() => list.split(',').any(|s| {
            let s = s.trim().to_ascii_lowercase();
            s == route.id() || s == mockup_name(route).to_ascii_lowercase()
        }),
        _ => true,
    }
}

/// Puts an app on `route` into the state its reference shows, ending on one running frame:
/// the Tray at gen 30 (REF-2's numbers in the readouts), the Loom weaving rule 30 at row 47
/// (the same fabric), the Terrarium running. The Rule lab and the Series open in their
/// reference state.
fn mockup_state(app: &mut App, route: Route) {
    let screens = app.state_mut().screens_mut();
    match route {
        Route::Tray => {
            screens.tray.step_n(29);
            screens.tray.set_running(true);
        }
        Route::Loom => {
            screens.loom.step_n(45);
            screens.loom.set_running(true);
        }
        Route::Terrarium => screens.terrarium.set_running(true),
        Route::RuleLab | Route::Series => {}
    }
    // one 0.1 s frame: one generation at 10 gen/s, one row at 14 rows/s from a fresh ticker
    app.step();
}

/// The app on `route` at `w` × at least `min_h` (grown to its content), in its reference's
/// state, and its render.
fn app_like_mockup(route: Route, w: f32, min_h: f32) -> (App, RgbaImage) {
    let mut app = harness_at(route, w, min_h);
    let need = app.state().content_height().ceil();
    if need > min_h + 0.5 {
        app.set_size(eframe::egui::vec2(w, need));
        settle(&mut app);
    }
    app.remove_cursor();
    settle(&mut app);
    mockup_state(&mut app, route);
    let img = app.render().expect("render");
    match route {
        Route::Tray => assert!(has_label(&app, "gen 0030"), "the Tray at gen 30"),
        Route::Loom => assert!(has_label(&app, "row 0047"), "the Loom at row 47"),
        _ => {}
    }
    (app, img)
}

fn render_like_mockup(route: Route, w: f32, min_h: f32) -> RgbaImage {
    app_like_mockup(route, w, min_h).1
}

/// The Series mockup is the bare page (no header); the app draws its header above it, so the
/// app's render is that much taller.
fn desktop(route: Route) -> RgbaImage {
    let (w, h) = route.board_size();
    let extra = if route == Route::Series { 57.0 } else { 0.0 };
    render_like_mockup(route, w, h + extra)
}

#[test]
fn side_by_side_with_mockup() {
    if std::env::var_os("AP_COMPARE").is_none_or(|v| v != "1") {
        eprintln!("compare: skipped (run scripts/compare-mockup.sh or set AP_COMPARE=1)");
        return;
    }
    let out_dir = repo().join("target/compare");
    std::fs::create_dir_all(&out_dir).expect("create target/compare");
    for route in Route::ALL.into_iter().filter(|r| wanted(*r)) {
        let name = mockup_name(route);
        let img = desktop(route);
        let mock = load(&mockup_png(route, 1440));
        let path = out_dir.join(format!("{name}-1440.png"));
        side_by_side(&img, &mock).save(&path).expect("save");
        img.save(out_dir.join(format!("{name}-1440-app.png")))
            .expect("save");
        println!("compare: {}", path.display());

        let mock390 = load(&mockup_png(route, 390));
        let phone = render_like_mockup(route, PHONE_W, mock390.height() as f32);
        let path = out_dir.join(format!("{name}-390.png"));
        side_by_side(&phone, &mock390).save(&path).expect("save");
        println!("compare: {}", path.display());
    }
}

/// The Series page (under the header) stays close to its mockup at 1440: the same ground,
/// cards and text positions give a small mean channel difference.
#[test]
fn series_is_close_to_its_mockup() {
    let img = desktop(Route::Series);
    let mock = load(&mockup_png(Route::Series, 1440));
    // compare the page under the header with the mockup's page
    let page = image::imageops::crop_imm(&img, 0, 57, 1440, 640).to_image();
    let d = mean_diff(&page, &mock, None);
    println!("series mean |Δ| = {d:.2}");
    assert!(d < 6.0, "Series drifted from its mockup: mean |Δ| = {d:.2}");
}

/// A region to compare: its name and (x, y, w, h) in both images.
type Region = (&'static str, (u32, u32, u32, u32));

/// The rect of the region (column) labelled `label`, clipped to the image.
fn region(app: &App, label: &'static str, img: &RgbaImage) -> Region {
    let r = get(app, label, Role::Region).rect();
    let x = r.left().max(0.0) as u32;
    let y = r.top().max(0.0) as u32;
    let w = (r.right().min(img.width() as f32) as u32).saturating_sub(x);
    let h = (r.bottom().min(img.height() as f32) as u32).saturating_sub(y);
    (label, (x, y, w, h))
}

/// The regions that don't animate (or animate identically: the Tray at gen 30, the Loom at
/// row 47 are deterministic) for `route`, with the largest mean |Δ| allowed in each.
fn stable_regions(app: &App, route: Route, img: &RgbaImage) -> Vec<(Region, f64)> {
    let full = (0, 0, img.width(), img.height());
    let phone = img.width() < 1000;
    let header = ("header", (0, 0, img.width(), if phone { 120 } else { 56 }));
    let r = |label| region(app, label, img);
    match (route, phone) {
        (Route::Tray, false) => vec![
            (header, 2.6),
            (r("Cartridges and stamps"), 2.7),
            (r("Bench"), 4.0),
            (r("Rule and readouts"), 2.0),
            (("whole", full), 3.3),
        ],
        (Route::Tray, true) => vec![
            (header, 2.4),
            (r("Cartridges and stamps"), 2.7),
            (r("Rule and readouts"), 1.8),
            (("whole", full), 5.0),
        ],
        (Route::Loom, false) => vec![
            (header, 2.4),
            (r("Rule and presets"), 2.4),
            (r("Loom"), 1.6),
            (("whole", full), 1.8),
        ],
        (Route::Loom, true) => vec![(header, 2.0), (r("Rule and presets"), 1.4)],
        (Route::Terrarium, false) => vec![
            (header, 2.4),
            (r("Breeding"), 2.2),
            (("tab row", (0, 57, 1117, 44)), 2.7),
        ],
        (Route::Terrarium, true) => vec![(header, 2.2), (r("Breeding"), 1.0)],
        (Route::RuleLab, false) => vec![
            (header, 2.5),
            (r("Rule spec"), 1.8),
            (r("Run"), 2.7),
            (r("Script and analysis"), 7.2),
            (("whole", full), 3.6),
        ],
        (Route::RuleLab, true) => vec![
            (header, 2.6),
            (r("Rule spec"), 1.8),
            (r("Script and analysis"), 6.2),
            (("whole", full), 5.0),
        ],
        (Route::Series, _) => vec![],
    }
}

/// DONE-1: every screen stays close to its reference at 1440 and 390 on the regions that
/// don't animate, in the reference's state (mean absolute channel difference per region; a
/// moved panel or a wrong colour pushes it up). Prints the table.
#[test]
fn screens_stay_close_to_their_mockups() {
    let mut failures = Vec::new();
    for route in Route::ALL {
        if route == Route::Series {
            continue; // series_is_close_to_its_mockup (the page under the header)
        }
        let (w, h) = route.board_size();
        for (width, min_h, mock) in [
            (w, h, load(&mockup_png(route, 1440))),
            (PHONE_W, 0.0, load(&mockup_png(route, 390))),
        ] {
            let min_h = if min_h > 0.0 {
                min_h
            } else {
                mock.height() as f32
            };
            let (app, img) = app_like_mockup(route, width, min_h);
            for ((name, rect), max) in stable_regions(&app, route, &img) {
                let d = mean_diff(&img, &mock, Some(rect));
                println!(
                    "close: {:9} {:4} {name:22} |Δ| {d:5.2} (max {max})",
                    route.id(),
                    width
                );
                if d >= max {
                    failures.push(format!("{route:?} {width} {name}: {d:.2} ≥ {max}"));
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "drifted from the mockup: {failures:#?}"
    );
}
