//! Mesh-building cost of full frames. The timing test is ignored by default; run it in release:
//!
//! ```text
//! cargo test -p ap-bench --release --test perf -- --ignored --nocapture
//! ```

mod common;

use ap_bench::loom::{KeyVisual, LoomScene, build_key};
use ap_bench::{Batch, TrayScene};
use egui::{Pos2, Rect, vec2};
use std::time::Instant;

const STAGE: Rect = Rect {
    min: Pos2::new(248.0, 102.0),
    max: Pos2::new(1120.0, 900.0),
};

/// A busy tray: every cell live or trailing, so every cell has a side and most a glow.
fn busy() -> common::Garden {
    let mut g = common::Garden::new(40, 26);
    for (i, a) in g.age.iter_mut().enumerate() {
        *a = (i % 13) as u16;
    }
    for (i, t) in g.trail.iter_mut().enumerate() {
        *t = (i % 4) as u8;
    }
    g
}

fn build_tray(g: &common::Garden) -> Batch {
    let cell = |c, r| g.visual(c, r);
    let mut scene = TrayScene::new(40, 26, &cell);
    scene.hover = Some((3, 4));
    scene.cursor = Some((5, 6));
    let mut b = Batch::with_params(2.0, vec2(1.0 / 1024.0, 1.0 / 1024.0));
    b.reserve(1100);
    scene.build(&mut b, STAGE);
    b
}

#[test]
fn tray_mesh_is_valid_and_bounded() {
    let g = busy();
    let b = build_tray(&g);
    let m = b.mesh();
    assert!(m.is_valid());
    assert!(m.vertices.len() < 250_000, "{} vertices", m.vertices.len());
}

#[test]
#[ignore = "timing; run in release with --ignored --nocapture"]
fn tray_frame_under_2ms() {
    for (name, g) in [
        ("mockup gen 30", common::Garden::mockup().run(30).clone()),
        ("every cell busy", busy()),
    ] {
        let _ = build_tray(&g); // warm the shadow cache
        let n = 200;
        let t = Instant::now();
        let mut verts = 0;
        for _ in 0..n {
            verts = build_tray(&g).mesh().vertices.len();
        }
        let per = t.elapsed().as_secs_f64() * 1000.0 / n as f64;
        eprintln!("tray ({name}): {per:.3} ms per frame, {verts} vertices");
        assert!(per < 2.0, "{name}: {per:.3} ms");
    }
    let rows_owned = common::weave(30, 72);
    let rows: Vec<&[bool]> = rows_owned.iter().map(|r| r.as_slice()).collect();
    let scene = LoomScene::new(&rows);
    let geo = scene.geometry(STAGE);
    let n = 200;
    let t = Instant::now();
    let mut verts = 0;
    for _ in 0..n {
        let mut b = Batch::with_params(2.0, vec2(1.0, 1.0));
        scene.paint_board(&mut b, &geo.board);
        for (i, r) in geo.keys.iter().enumerate() {
            build_key(&mut b, *r, KeyVisual::for_rule(30, 7 - i as u8));
        }
        verts = b.mesh().vertices.len();
    }
    let per = t.elapsed().as_secs_f64() * 1000.0 / n as f64;
    eprintln!("loom (72 rows of rule 30 + keys): {per:.3} ms per frame, {verts} vertices");
    assert!(per < 2.0);
}
