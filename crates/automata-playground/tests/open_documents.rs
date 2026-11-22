//! Patterns opened from Finder (TRAY-9): a path queued the way the Apple Event `odoc` handler
//! and the command line queue it (`platform::queue_open`) arms the Tray's custom stamp and
//! switches to the Tray; a file that doesn't parse only shows a toast; an app that isn't the
//! receiver leaves the queue alone.
//!
//! One test function: the queue and the receiver are process-wide.

mod common;

use automata_playground::screens::tray::{StampChoice, Tool};
use automata_playground::{Route, platform};
use common::*;

#[test]
fn opened_patterns_arm_the_tray_stamp() {
    let dir = std::env::temp_dir().join(format!("ap-open-docs-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let rle = dir.join("my glider.rle");
    std::fs::write(
        &rle,
        "#N Lucky glider\nx = 3, y = 3, rule = B3/S23\nbo$2bo$3o!\n",
    )
    .unwrap();
    let cells = dir.join("blinker.cells");
    std::fs::write(&cells, ".O.\n.O.\n.O.\n").unwrap();
    let broken = dir.join("broken.rle");
    std::fs::write(&broken, "x = 3, y = 3\nbo$2bo$3q!\n").unwrap();

    let mut h = app().at(Route::Series).build();
    let mut bystander = app().at(Route::Loom).build();
    platform::set_receiver(&h.ctx);

    // an .rle with a #N name: the Tray opens with it armed under that name
    platform::queue_open([rle.clone()]);
    bystander.run_steps(2);
    assert_eq!(route(&bystander), Route::Loom, "not the receiver");
    assert!(bystander.state().screens().tray.custom_stamp().is_none());
    h.run_steps(2);
    assert_eq!(route(&h), Route::Tray);
    let tray = &h.state().screens().tray;
    let stamp = tray.custom_stamp().expect("armed");
    assert_eq!(stamp.name, "Lucky glider");
    assert_eq!(stamp.pattern.cells().len(), 5);
    assert_eq!(tray.stamp(), StampChoice::Custom);
    assert_eq!(tray.current_tool(), Tool::Stamp);
    assert!(
        toasts(&h)
            .iter()
            .any(|t| t.starts_with("Armed Lucky glider as a stamp")),
        "{:?}",
        toasts(&h)
    );
    // an extra, selected stamp tile in the aside, and the hint names it
    assert_eq!(pressed(&h, "Lucky glider"), Some(true));
    assert_eq!(pressed(&h, "Glider"), Some(false));
    assert!(has_label(&h, "Click to place Lucky glider"));

    // a plaintext .cells without a name: named by the file's stem; from another screen too
    goto(&mut h, Route::RuleLab);
    platform::queue_open([cells]);
    h.run_steps(2);
    assert_eq!(route(&h), Route::Tray);
    assert_eq!(
        h.state()
            .screens()
            .tray
            .custom_stamp()
            .map(|s| s.name.as_str()),
        Some("blinker")
    );

    // a file that doesn't parse: a toast, the stamp and the screen stay
    goto(&mut h, Route::Loom);
    platform::queue_open([broken, dir.join("missing.rle")]);
    h.run_steps(2);
    assert_eq!(route(&h), Route::Loom);
    let t = toasts(&h);
    assert!(
        t.iter().any(|t| t.starts_with("Couldn't read broken.rle")),
        "{t:?}"
    );
    assert!(
        t.iter().any(|t| t.starts_with("Couldn't read missing.rle")),
        "{t:?}"
    );
    assert_eq!(
        h.state()
            .screens()
            .tray
            .custom_stamp()
            .map(|s| s.name.as_str()),
        Some("blinker")
    );
    let _ = std::fs::remove_dir_all(&dir);
}
