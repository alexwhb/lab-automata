//! Golden check of the projection against Chrome itself: `tests/data/chrome-geometry.json` holds
//! the projected border quads (`DOM.getBoxModel`) and bounding boxes Chrome computed for the
//! mockup's transformed elements (recorded by `scripts/chrome-geometry.mjs`). Our scenes must
//! put the same elements at the same screen positions.

use ap_bench::loom::LoomScene;
use ap_bench::stage::FLOOR_TILT;
use ap_bench::{CellVisual, JarScene, Projection, TrayScene, View};
use egui::{Pos2, Rect, pos2, vec2};
use serde_json::Value;

/// Largest allowed deviation in px.
const TOL: f32 = 0.5;

fn data() -> Value {
    serde_json::from_str(include_str!("data/chrome-geometry.json")).expect("valid JSON")
}

fn quad(v: &Value) -> [Pos2; 4] {
    let q: Vec<f32> = v["quad"]
        .as_array()
        .expect("quad")
        .iter()
        .map(|x| x.as_f64().unwrap() as f32)
        .collect();
    [
        pos2(q[0], q[1]),
        pos2(q[2], q[3]),
        pos2(q[4], q[5]),
        pos2(q[6], q[7]),
    ]
}

fn rect(v: &Value) -> Rect {
    let f = |k: &str| v[k].as_f64().unwrap() as f32;
    Rect::from_min_size(pos2(f("x"), f("y")), vec2(f("width"), f("height")))
}

fn stage(v: &Value) -> Rect {
    let g = &v["graph"];
    // coordinates in the JSON are relative to the stage's top-left
    Rect::from_min_size(
        Pos2::ZERO,
        vec2(
            g["width"].as_f64().unwrap() as f32,
            g["height"].as_f64().unwrap() as f32,
        ),
    )
}

/// Max corner distance and bbox deviation between our quad and Chrome's (with a relative
/// allowance for the floor, whose near corners land thousands of px away).
fn assert_quad(name: &str, ours: [Pos2; 4], chrome: [Pos2; 4]) -> f32 {
    let mut worst = 0.0f32;
    for (a, b) in ours.iter().zip(chrome.iter()) {
        let d = (*a - *b).length();
        let tol = TOL + b.to_vec2().length() * 2e-5;
        assert!(d <= tol, "{name}: corner {a:?} vs Chrome {b:?} ({d:.3} px)");
        worst = worst.max(d);
    }
    let (ra, rb) = (Rect::from_points(&ours), Rect::from_points(&chrome));
    for (x, y) in [(ra.min, rb.min), (ra.max, rb.max)] {
        let d = (x - y).abs().max_elem();
        assert!(
            d <= TOL + y.to_vec2().length() * 2e-5,
            "{name}: bbox {ra:?} vs {rb:?}"
        );
    }
    worst
}

fn tray_case(key: &str, tilt: f32) -> f32 {
    let d = data();
    let v = &d[key];
    let items = v["items"].as_array().unwrap();
    let cells: Vec<usize> = v["cells"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_u64().unwrap() as usize)
        .collect();
    // which recorded cells are live (lifted)
    let lifted: Vec<(usize, usize)> = cells
        .iter()
        .zip(&items[1..])
        .filter(|(_, it)| {
            let c = it["className"].as_str().unwrap_or("");
            c.split_whitespace()
                .any(|k| matches!(k, "cn" | "ca" | "co"))
        })
        .map(|(&i, _)| (i % 40, i / 40))
        .collect();
    assert!(!lifted.is_empty(), "the goldens include a lifted cell");
    let cell = |c: usize, r: usize| {
        if lifted.contains(&(c, r)) {
            CellVisual::Live(2)
        } else {
            CellVisual::Empty
        }
    };
    let mut scene = TrayScene::new(40, 26, &cell);
    scene.tilt = tilt;
    let geo = scene.geometry(stage(v));
    let p = geo.projection();
    let mut worst = assert_quad(
        &format!("{key} tray"),
        p.quad(p.local_rect()),
        quad(&items[0]),
    );
    worst = worst.max(
        (geo.tray_rect().min - rect(&items[0]["bbox"]).min)
            .abs()
            .max_elem(),
    );
    for (k, &i) in cells.iter().enumerate() {
        let (c, r) = (i % 40, i / 40);
        worst = worst.max(assert_quad(
            &format!("{key} cell {c},{r}"),
            geo.cell_quad(c, r),
            quad(&items[1 + k]),
        ));
    }
    // the floor (last item): the container is centred in the stage
    let floor = items.last().unwrap();
    let container_x = stage(v).width() / 2.0 - 420.0;
    let fp = Projection::new(
        FLOOR_TILT,
        Rect::from_min_size(pos2(container_x - 360.0, 300.0), vec2(1560.0, 760.0)),
        View::new(Pos2::ZERO, 1.0),
    );
    worst.max(assert_quad(
        &format!("{key} floor"),
        fp.quad(fp.local_rect()),
        quad(floor),
    ))
}

#[test]
fn tray_tilted_matches_chrome() {
    let worst = tray_case("tray_tilted", 1.0);
    eprintln!("tray tilted: max deviation {worst:.4} px");
}

#[test]
fn tray_flat_matches_chrome() {
    let worst = tray_case("tray_flat", 0.0);
    eprintln!("tray flat: max deviation {worst:.4} px");
}

#[test]
fn loom_board_head_and_keys_match_chrome() {
    let d = data();
    let v = &d["loom"];
    let items = v["items"].as_array().unwrap();
    let rows: Vec<&[bool]> = Vec::new();
    let scene = LoomScene::new(&rows);
    let geo = scene.geometry(stage(v));
    let p = geo.board;
    let mut worst = assert_quad("loom board", p.quad(p.local_rect()), quad(&items[0]));
    let c = vec2(stage(v).width() / 2.0 - 430.0, 0.0);
    let fp = Projection::new(
        FLOOR_TILT,
        Rect::from_min_size(pos2(c.x - 360.0, 250.0), vec2(1580.0, 760.0)),
        View::new(Pos2::ZERO, 1.0),
    );
    worst = worst.max(assert_quad(
        "loom floor",
        fp.quad(fp.local_rect()),
        quad(&items[1]),
    ));
    let layout = &v["layout"];
    let head = rect(&layout["head"]).translate(c);
    assert!(
        (geo.head.min - head.min).abs().max_elem() < TOL,
        "{:?} vs {head:?}",
        geo.head
    );
    assert!((geo.head.size() - head.size()).abs().max_elem() < TOL);
    for (i, k) in layout["keys"].as_array().unwrap().iter().enumerate() {
        let on = layout["key_on"][i].as_bool().unwrap();
        // Chrome reports pressed keys translated by 4 px
        let chrome = rect(k).translate(c - vec2(0.0, if on { 4.0 } else { 0.0 }));
        let d = (geo.keys[i].min - chrome.min).abs().max_elem();
        assert!(d < TOL, "key {i}: {:?} vs {chrome:?}", geo.keys[i]);
        assert!((geo.keys[i].size() - chrome.size()).abs().max_elem() < TOL);
        worst = worst.max(d);
    }
    eprintln!("loom: max deviation {worst:.4} px");
}

#[test]
fn terrarium_jar_matches_chrome() {
    let d = data();
    let v = &d["jar"];
    let items = v["items"].as_array().unwrap();
    // items: the transformed jar, its bob wrapper, the 272 × 228 jar area
    let area = rect(&items[2]["bbox"]);
    let none = |_: usize, _: usize| false;
    let scene = JarScene::new(&none, egui::Color32::WHITE);
    let p = scene.projection(area);
    let worst = assert_quad("jar", p.quad(p.local_rect()), quad(&items[0]));
    eprintln!("jar: max deviation {worst:.4} px");
}
