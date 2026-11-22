//! Hit-testing through the perspective (SPEC TRAY-3: hovering highlights the cell under the
//! pointer through the perspective; TRAY-10: cell rects for the focus ring).

mod common;

use ap_bench::loom::LoomScene;
use ap_bench::topdown::TopCell;
use ap_bench::{CellVisual, TopDownGrid, TrayScene};
use egui::{Pos2, Rect, pos2, vec2};

const STAGE: Rect = Rect {
    min: Pos2::new(248.0, 102.0),
    max: Pos2::new(1120.0, 900.0),
};

fn centre(q: [Pos2; 4]) -> Pos2 {
    // the centre of a projected square is where its diagonals cross; the mean of the corners
    // is close enough for a hit test (≪ 1 px off) and stays inside the face
    pos2(
        q.iter().map(|p| p.x).sum::<f32>() / 4.0,
        q.iter().map(|p| p.y).sum::<f32>() / 4.0,
    )
}

#[test]
fn every_cell_centre_hits_its_cell_at_any_tilt_and_zoom() {
    let garden = common::Garden::mockup().run(30).clone();
    let cell = |c, r| garden.visual(c, r);
    for tilt in [0.0, 0.25, 0.5, 0.75, 1.0] {
        for zoom in [1.0, 0.6] {
            let mut scene = TrayScene::new(40, 26, &cell);
            scene.tilt = tilt;
            scene.zoom = zoom;
            let geo = scene.geometry(STAGE);
            for r in 0..26 {
                for c in 0..40 {
                    let p = centre(geo.cell_quad(c, r));
                    assert_eq!(geo.cell_at(p), Some((c, r)), "tilt {tilt} zoom {zoom}");
                    assert_eq!(geo.cell_near(p), Some((c, r)));
                }
            }
        }
    }
}

#[test]
fn corner_cells_with_and_without_lift() {
    for lifted in [false, true] {
        let cell = |_c, _r| {
            if lifted {
                CellVisual::Live(5)
            } else {
                CellVisual::Empty
            }
        };
        for tilt in [0.0, 0.5, 1.0] {
            let mut scene = TrayScene::new(40, 26, &cell);
            scene.tilt = tilt;
            let geo = scene.geometry(STAGE);
            for (c, r) in [(0, 0), (39, 0), (0, 25), (39, 25)] {
                let q = geo.cell_quad(c, r);
                assert_eq!(
                    geo.cell_at(centre(q)),
                    Some((c, r)),
                    "lifted {lifted} tilt {tilt}"
                );
                // a point just inside each corner of the face also hits
                for corner in q {
                    let p = corner + (centre(q) - corner) * 0.1;
                    assert_eq!(geo.cell_at(p), Some((c, r)));
                }
            }
        }
    }
}

#[test]
fn gaps_belong_to_no_cell_but_drags_snap_to_the_nearest() {
    let cell = |_c, _r| CellVisual::Empty;
    let scene = TrayScene::new(40, 26, &cell);
    let geo = scene.geometry(STAGE);
    let p = geo.projection();
    // the middle of the vertical gap between (10, 5) and (11, 5): local x = 35 + 10·19 + 17.5
    let gap = p.project(pos2(35.0 + 10.0 * 19.0 + 17.5, 36.0 + 5.0 * 19.0 + 8.0));
    assert_eq!(geo.cell_at(gap), None);
    assert!(matches!(geo.cell_near(gap), Some((10 | 11, 5))));
    // horizontal gap
    let gap = p.project(pos2(35.0 + 3.0 * 19.0 + 8.0, 36.0 + 7.0 * 19.0 + 17.5));
    assert_eq!(geo.cell_at(gap), None);
    assert!(matches!(geo.cell_near(gap), Some((3, 7 | 8))));
}

#[test]
fn lifted_cells_hit_on_their_face_not_their_side() {
    // only (5, 5) is alive: its face is 6 px up the tray, its vacated bottom is no cell
    let cell = |c, r| {
        if (c, r) == (5, 5) {
            CellVisual::Live(1)
        } else {
            CellVisual::Empty
        }
    };
    let scene = TrayScene::new(40, 26, &cell);
    let geo = scene.geometry(STAGE);
    let p = geo.projection();
    let x = 35.0 + 5.0 * 19.0 + 8.0;
    let y0 = 36.0 + 5.0 * 19.0;
    // the face's top reaches into the gap and the row above's bottom 3 px
    assert_eq!(geo.cell_at(p.project(pos2(x, y0 - 5.0))), Some((5, 5)));
    assert_eq!(geo.cell_at(p.project(pos2(x, y0 - 1.5))), Some((5, 5)));
    // the bottom 6 px (where the side shows) are not the cell
    assert_eq!(geo.cell_at(p.project(pos2(x, y0 + 13.0))), None);
    assert_eq!(geo.cell_near(p.project(pos2(x, y0 + 13.0))), Some((5, 5)));
}

#[test]
fn outside_the_grid_is_none() {
    let cell = |_c, _r| CellVisual::Empty;
    for tilt in [0.0, 1.0] {
        let mut scene = TrayScene::new(40, 26, &cell);
        scene.tilt = tilt;
        let geo = scene.geometry(STAGE);
        let g = geo.grid_rect();
        for p in [
            g.left_top() - vec2(20.0, 20.0),
            g.right_bottom() + vec2(20.0, 20.0),
            pos2(g.center().x, g.top() - 30.0),
            pos2(g.left() - 30.0, g.center().y),
            STAGE.left_top(), // far above the horizon of the tilted tray
            pos2(-5000.0, -5000.0),
        ] {
            assert_eq!(geo.cell_at(p), None, "{p:?} tilt {tilt}");
            assert_eq!(geo.cell_near(p), None, "{p:?} tilt {tilt}");
        }
    }
}

#[test]
fn cell_rects_are_inside_the_tray_and_shrink_with_distance() {
    let cell = |_c, _r| CellVisual::Empty;
    let scene = TrayScene::new(40, 26, &cell);
    let geo = scene.geometry(STAGE);
    let tray = geo.tray_rect();
    let near = geo.cell_rect(0, 25);
    let far = geo.cell_rect(0, 0);
    assert!(tray.contains_rect(near) && tray.contains_rect(far));
    assert!(far.width() < near.width() && far.height() < near.height());
}

#[test]
fn loom_keys_and_fabric_hit_test() {
    let rows_owned = common::weave(30, 47);
    let rows: Vec<&[bool]> = rows_owned.iter().map(|r| r.as_slice()).collect();
    let scene = LoomScene::new(&rows);
    let geo = scene.geometry(Rect::from_min_size(Pos2::ZERO, vec2(1120.0, 798.0)));
    // keys are in order and don't overlap
    for w in geo.keys.windows(2) {
        assert!(w[0].right() < w[1].left());
    }
    // the newest row's centre cell
    let cw = 752.0 / 121.0;
    let rh = 452.0 / 72.0;
    let local = pos2(60.0 * cw + cw / 2.0, 71.0 * rh + rh / 2.0);
    let screen = geo.board.project(local);
    assert_eq!(geo.fabric_cell_at(&scene, screen), Some((60, 46)));
    // above the woven part: nothing
    let screen = geo.board.project(pos2(100.0, 10.0));
    assert_eq!(geo.fabric_cell_at(&scene, screen), None);
}

#[test]
fn top_down_cells_round_trip() {
    let cell = |_x, _y| TopCell::Empty;
    let grid = TopDownGrid::new(64, 40, &cell);
    for avail in [800.0, 400.0] {
        let geo = grid.geometry(pos2(10.0, 20.0), avail);
        assert!(geo.well.width() <= avail + 0.01);
        for (x, y) in [(0, 0), (63, 39), (31, 19)] {
            assert_eq!(geo.cell_at(geo.cell_rect(x, y).center()), Some((x, y)));
        }
        assert_eq!(geo.cell_at(pos2(0.0, 0.0)), None);
    }
}
