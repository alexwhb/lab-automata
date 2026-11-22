//! The Series screen (SER-1, NAV-3): the identity cards with their ramps, the cell-age scale,
//! "Shared, unchanged", the icon family; the card highlight; Open → the launcher (opened,
//! not installed, planned) reported with toasts.

mod common;

use automata_playground::{AppId, Launcher, Route};
use common::*;
use eframe::egui::accesskit::Role;
use egui_kittest::kittest::{NodeT, Queryable};

/// Every card, swatch and line of the mockup is there, with its text.
#[test]
fn shows_the_mockups_content() {
    let h = harness(Route::Series);
    get(&h, "Same family, one new accent", Role::Heading);
    assert!(has_label(&h, "SPICE-for-X series · fourth bench"));
    for app in AppId::ALL {
        get(&h, app.name(), Role::Group);
        let id = app.identity();
        assert!(has_label(&h, &format!("{}, {}", id.name, id.domain)));
        for (role, c) in [
            ("Accent", id.accent),
            ("Ink", id.ink),
            ("Tint", id.tint),
            ("Edge", id.edge),
        ] {
            let [r, g, b, _] = c.to_srgba_unmultiplied();
            let label = format!("{} {role} #{r:02X}{g:02X}{b:02X}", id.name);
            assert!(has_label(&h, &label), "missing {label}");
        }
    }
    assert!(has_label(&h, "Breadbox Accent #F2A33A"), "the mockup's hex");
    for s in [
        "Newborn #FFE6F7",
        "Alive #F06BD0",
        "Elder #B24C9C",
        "Trail #3B1734",
        "Empty #1B1C21",
    ] {
        assert!(has_label(&h, s), "cell-age {s}");
    }
    for s in [
        "Neutrals, grid ground, tilted bench",
        "Header, mode switch, Run · Step",
        "Parts palette (here: rules and stamps)",
        "Advanced mode as a netlist-style script",
        "Space Grotesk, Inter, JetBrains Mono",
        "Reads in greyscale too. Live cells lift 6 px off the tray.",
        "3 × 3 cell grids: grey empty, magenta live.",
    ] {
        assert!(has_label(&h, s), "missing {s:?}");
    }
    for g in [
        "Glider glyph",
        "Blinker glyph",
        "Block glyph",
        "Ring glyph",
        "Corners glyph",
    ] {
        get(&h, g, Role::Image);
    }
    // Automata is the new one and has no Open button
    assert!(has_label(&h, "Automata Playground (this app, new)"));
    assert!(h.query_by_label_contains("Open Automata").is_none());
}

/// NAV-3: `open_series(app)` opens the Series screen with that card highlighted; opening it
/// any other way highlights nothing.
#[test]
fn open_series_highlights_the_card() {
    let mut h = harness(Route::Tray);
    for app in AppId::SIBLINGS {
        h.state_mut().open_series(app);
        settle(&mut h);
        assert_eq!(route(&h), Route::Series);
        for other in AppId::ALL {
            assert_eq!(
                selected(&h, other.name(), Role::Group),
                Some(other == app),
                "{app:?} → {other:?}"
            );
        }
    }
    h.state_mut().navigate(Route::Tray);
    settle(&mut h);
    h.state_mut().navigate(Route::Series);
    settle(&mut h);
    assert_eq!(selected(&h, "Chem Playground", Role::Group), Some(false));
}

/// At 390 the highlighted card is scrolled into view.
#[test]
fn highlight_scrolls_into_view_on_a_phone() {
    let mut h = app().phone().size(PHONE_W, 700.0).build();
    h.state_mut().open_series(AppId::Bio);
    settle(&mut h);
    let card = get(&h, "Bio Playground", Role::Group).rect();
    let view = viewport(&h);
    assert!(
        view.contains(card.center()),
        "card {card:?} not in view {view:?}"
    );
}

/// Open calls the launcher: an installed app opens, a missing one says it isn't installed,
/// the planned one can't be pressed.
#[test]
fn open_buttons_use_the_launcher() {
    let launcher = Launcher::recording(&[AppId::Chem]);
    let mut h = app().at(Route::Series).launcher(launcher.clone()).build();
    click_button(&mut h, "Open Chem Playground");
    assert_eq!(launcher.calls(), [AppId::Chem]);
    assert!(toasts(&h).contains(&"Opening Chem Playground…".to_string()));

    click_button(&mut h, "Open Breadbox");
    assert_eq!(launcher.calls(), [AppId::Chem, AppId::Breadbox]);
    assert!(toasts(&h).contains(&"Breadbox isn't installed on this Mac".to_string()));
    // the toast is on screen as a status message
    get(&h, "Breadbox isn't installed on this Mac", Role::Status);

    let bio = "Open Bio Playground (planned, not built yet)";
    assert!(disabled(&h, bio));
    h.get_by_label(bio).click();
    settle(&mut h);
    assert_eq!(
        launcher.calls().len(),
        2,
        "the planned app is never launched"
    );
}

/// Toasts go away after a few seconds.
#[test]
fn toasts_expire() {
    let mut h = harness(Route::Series);
    click_button(&mut h, "Open Breadbox");
    assert_eq!(toasts(&h).len(), 1);
    // kittest steps 0.25 s of egui time per frame
    assert!(step_until(&mut h, 40, |h| toasts(h).is_empty()));
    assert!(h.query_by_role(Role::Status).is_none());
}

/// At 1440 the cards sit four (then three) to a row; at 390 they stack.
#[test]
fn cards_flow_into_columns() {
    let h = harness(Route::Series);
    let a = get(&h, "Breadbox", Role::Group).rect();
    let b = get(&h, "Automata Playground", Role::Group).rect();
    assert!(
        (a.top() - b.top()).abs() < 0.5,
        "identity cards share a row"
    );
    assert!(b.left() > a.right());
    let h = app().at(Route::Series).phone().build();
    let a = get(&h, "Breadbox", Role::Group).rect();
    let b = get(&h, "Chem Playground", Role::Group).rect();
    assert!(b.top() > a.bottom(), "stacked at 390");
    assert!((a.left() - b.left()).abs() < 0.5);
    assert!(a.right() <= PHONE_W, "{a:?}");
}

/// The identity cards and lower cards land on the reference's pixel rows (Series-1440.png,
/// measured: cards 165–327, lower row 352–541 on the 640 px page under the 57 px header).
#[test]
fn layout_matches_the_reference_rows() {
    let h = app().at(Route::Series).size(1440.0, 697.0).build();
    let id = get(&h, "Breadbox", Role::Group).rect();
    assert!((id.top() - 57.0 - 165.0).abs() <= 1.0, "{id:?}");
    assert!((id.bottom() - 57.0 - 328.0).abs() <= 1.0, "{id:?}");
    let lower = get(&h, "Shared with the series", Role::Group).rect();
    assert!((lower.top() - 57.0 - 352.0).abs() <= 1.0, "{lower:?}");
    assert!((lower.bottom() - 57.0 - 542.0).abs() <= 1.5, "{lower:?}");
    // the three lower cards are stretched to the same height (CSS grid rows)
    let age = get(&h, "Cell-age scale", Role::Group).rect();
    let icons = get(&h, "Icon family", Role::Group).rect();
    assert!((age.height() - lower.height()).abs() < 0.5);
    assert!((icons.height() - lower.height()).abs() < 0.5);
    let _ = h.root().children_recursive().count();
}
