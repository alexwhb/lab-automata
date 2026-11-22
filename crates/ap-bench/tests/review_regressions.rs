//! Regressions for the independent review's ap-bench findings (each test names its finding).

use ap_bench::loom::LoomScene;
use egui::{Rect, pos2, vec2};

/// Review finding 13: `Rect::contains` includes the board's bottom edge, so a point that
/// unprojects to exactly `y = board height` gave row index `capacity` — one past the rows
/// (`(560, 470)` on a full 72-row loom answered `Some((60, 72))`).
#[test]
fn fabric_cell_at_never_returns_a_row_past_the_rows() {
    let row = vec![true; 121];
    let rows: Vec<&[bool]> = (0..72).map(|_| row.as_slice()).collect();
    let scene = LoomScene::new(&rows);
    let stage = Rect::from_min_size(pos2(0.0, 0.0), vec2(1120.0, 800.0));
    let geo = scene.geometry(stage);
    if let Some((i, r)) = geo.fabric_cell_at(&scene, pos2(560.0, 470.0)) {
        assert!(i < 121 && r < rows.len(), "({i}, {r})");
    }
    // walk screen y across the projected bottom edge, one float step at a time, and check
    // every hit (the edge itself included)
    let size = geo.board.size();
    let mut edge_hits = 0;
    for x in [size.x * 0.1, size.x / 2.0, size.x * 0.9] {
        let mid = geo.board.project(pos2(x, size.y));
        let mut y = mid.y - 0.01;
        while y < mid.y + 0.01 {
            let p = pos2(mid.x, y);
            if geo.board.unproject_inside(p).is_some_and(|l| l.y == size.y) {
                edge_hits += 1;
            }
            if let Some((i, r)) = geo.fabric_cell_at(&scene, p) {
                assert!(i < 121 && r < rows.len(), "({i}, {r}) at {p:?}");
            }
            y = f32::from_bits(y.to_bits() + 1);
        }
    }
    assert!(edge_hits > 0, "the scan never hit the bottom edge exactly");
}
