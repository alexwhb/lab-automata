//! Side-by-side `[app | mockup]` images of every screen, for human and agent review (not a
//! pixel-diff gate). Run through `scripts/compare-mockup.sh`, which sets `AP_COMPARE=1`;
//! without it the render test does nothing.
//!
//! For each route it renders the app at the mockup's board size (`Route::board_size`; the
//! Series screen gets the header on top of its 640 px board, which the mockup leaves out) and
//! at 390 px wide (as tall as the reference or the app's content, whichever is taller), puts
//! the reference from `docs/design/screenshots/` next to it and writes
//! `target/compare/<Screen>-1440.png` / `<Screen>-390.png`. `AP_COMPARE_ONLY=series,tray`
//! limits it to some routes.

mod common;

use automata_playground::Route;
use common::*;
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

/// The Series mockup is the bare page (no header); the app draws its header above it, so the
/// app's render is that much taller.
fn desktop(route: Route) -> RgbaImage {
    let (w, h) = route.board_size();
    if route == Route::Series {
        let mut app = harness_at(route, w, h + 57.0);
        render(&mut app)
    } else {
        let mut app = harness_at(route, w, h);
        render(&mut app)
    }
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
        let phone = render_full(route, PHONE_W, mock390.height() as f32);
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
