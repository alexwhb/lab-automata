//! Shared helpers for the GUI tests: build the real app at any size and route with
//! deterministic options, find and click widgets by their AccessKit label (and role), read
//! pressed / selected / disabled states, type, press shortcuts, step until a condition holds,
//! read the route, and render PNGs.
//!
//! ```ignore
//! mod common;
//! use common::*;
//!
//! let mut h = app().at(Route::Loom).phone().build();   // 390 × 1000
//! click_tab(&mut h, "Tray · 2D");
//! assert_eq!(route(&h), Route::Tray);
//! ```
#![allow(dead_code)]

use automata_playground::{AppOptions, AutomataApp, Launcher, Route};
use eframe::egui::{self, Modifiers, accesskit::Role, accesskit::Toggled};
use egui_kittest::kittest::{NodeT, Queryable};
use egui_kittest::{Harness, Node};
use std::path::{Path, PathBuf};

/// The app under test.
pub type App = Harness<'static, AutomataApp>;

/// Desktop width and height (the mockup's 1440 × 900 board).
pub const W: f32 = 1440.0;
pub const H: f32 = 900.0;
/// Phone width (the mockup's 390 px references).
pub const PHONE_W: f32 = 390.0;
/// Phone height for interactive tests (renders use the reference's full height).
pub const PHONE_H: f32 = 1000.0;

// --------------------------------------------------------------------------------------------
// Building

/// Starts building an app harness: `app().at(Route::Loom).size(1024.0, 768.0).build()`.
pub fn app() -> AppBuilder {
    AppBuilder {
        opts: AppOptions::test(),
        size: egui::vec2(W, H),
    }
}

/// Builder for [`App`].
pub struct AppBuilder {
    opts: AppOptions,
    size: egui::Vec2,
}

impl AppBuilder {
    /// Open on `route`.
    pub fn at(mut self, route: Route) -> Self {
        self.opts.start = route;
        self
    }
    /// Window size in points (1 pt = 1 px: the harness renders at 1×).
    pub fn size(mut self, w: f32, h: f32) -> Self {
        self.size = egui::vec2(w, h);
        self
    }
    /// 390 × 1000.
    pub fn phone(self) -> Self {
        self.size(PHONE_W, PHONE_H)
    }
    /// The route's mockup board size (`Route::board_size`).
    pub fn board(self) -> Self {
        let (w, h) = self.opts.start.board_size();
        self.size(w, h)
    }
    /// A launcher (e.g. `Launcher::recording(&[AppId::Chem])`; keep a clone to read calls).
    pub fn launcher(mut self, l: Launcher) -> Self {
        self.opts.launcher = l;
        self
    }
    /// Start running (like the real app).
    pub fn autoplay(mut self) -> Self {
        self.opts.autoplay = true;
        self
    }
    /// Any options (the route set by [`AppBuilder::at`] is replaced by `opts.start`).
    pub fn options(mut self, opts: AppOptions) -> Self {
        self.opts = opts;
        self
    }

    /// Builds the harness and lets layout settle.
    pub fn build(self) -> App {
        let opts = self.opts;
        let mut h = Harness::builder()
            .with_size(self.size)
            .with_pixels_per_point(1.0)
            .with_max_steps(64)
            .wgpu()
            .build_eframe(move |cc| AutomataApp::new(cc, opts));
        settle(&mut h);
        h
    }
}

/// A 1440 × 900 app on `route`.
pub fn harness(route: Route) -> App {
    app().at(route).build()
}

/// An app on `route` at `w` × `h`.
pub fn harness_at(route: Route, w: f32, h: f32) -> App {
    app().at(route).size(w, h).build()
}

/// Runs a few frames so layout (remembered slot sizes, fonts) has settled.
pub fn settle(h: &mut App) {
    h.run_steps(4);
}

// --------------------------------------------------------------------------------------------
// State

/// The current screen.
pub fn route(h: &App) -> Route {
    h.state().route()
}

/// Puts an existing harness on `r` (cheaper than building a new one).
pub fn goto(h: &mut App, r: Route) {
    h.state_mut().navigate(r);
    settle(h);
    assert_eq!(route(h), r);
}

/// Steps (up to `max_steps` frames) until `cond` holds. Returns whether it did.
pub fn step_until(h: &mut App, max_steps: usize, mut cond: impl FnMut(&mut App) -> bool) -> bool {
    for _ in 0..max_steps {
        if cond(h) {
            return true;
        }
        h.step();
    }
    cond(h)
}

/// The texts of the toasts showing.
pub fn toasts(h: &App) -> Vec<String> {
    h.state()
        .toasts()
        .items()
        .iter()
        .map(|t| t.text.clone())
        .collect()
}

// --------------------------------------------------------------------------------------------
// Finding and acting by accessible label

/// True if any node has exactly this label.
pub fn has_label(h: &App, label: &str) -> bool {
    h.query_all_by_label(label).next().is_some()
}

/// True if any node's label contains `part`.
pub fn has_label_containing(h: &App, part: &str) -> bool {
    h.query_all_by_label_contains(part).next().is_some()
}

/// The single node with this label and role (panics with a useful message otherwise).
pub fn get<'a>(h: &'a App, label: &'a str, role: Role) -> Node<'a> {
    h.get_by_role_and_label(role, label)
}

/// Clicks the single node labelled `label` (any role) and settles.
pub fn click(h: &mut App, label: &str) {
    h.get_by_label(label).click();
    settle(h);
}

/// Clicks the node labelled `label` with `role` and settles.
pub fn click_role(h: &mut App, label: &str, role: Role) {
    get(h, label, role).click();
    settle(h);
}

/// Clicks a link (the brand, mode links, app-switcher icons).
pub fn click_link(h: &mut App, label: &str) {
    click_role(h, label, Role::Link);
}

/// Clicks a button.
pub fn click_button(h: &mut App, label: &str) {
    click_role(h, label, Role::Button);
}

/// Clicks a bench tab.
pub fn click_tab(h: &mut App, label: &str) {
    click_role(h, label, Role::Tab);
}

/// The pressed state (`aria-pressed`) of the button labelled `label`.
pub fn pressed(h: &App, label: &str) -> Option<bool> {
    get(h, label, Role::Button)
        .accesskit_node()
        .toggled()
        .map(|t| t == Toggled::True)
}

/// The selected state (tabs, the current mode / app) of the node with `label` and `role`.
pub fn selected(h: &App, label: &str, role: Role) -> Option<bool> {
    get(h, label, role).accesskit_node().is_selected()
}

/// Whether the node labelled `label` (any role) is disabled.
pub fn disabled(h: &App, label: &str) -> bool {
    h.get_by_label(label).accesskit_node().is_disabled()
}

/// Focuses the text field labelled `label` and types `text` into it.
pub fn type_into(h: &mut App, label: &str, text: &str) {
    h.get_by_label(label).focus();
    h.step();
    h.get_by_label(label).type_text(text);
    settle(h);
}

/// Presses a key and runs a frame.
pub fn press(h: &mut App, key: egui::Key) {
    h.key_press(key);
    h.step();
}

/// Presses ⌘ + `key` and runs a frame.
pub fn press_cmd(h: &mut App, key: egui::Key) {
    h.key_press_modifiers(Modifiers::COMMAND, key);
    h.step();
}

/// The label of the node that has keyboard focus, if any.
pub fn focused_label(h: &App) -> Option<String> {
    h.root()
        .children_recursive()
        .find(|n| n.is_focused())
        .and_then(|n| n.accesskit_node().label())
}

/// Every node with one of `roles`, as (role, label, bounds).
pub fn nodes_with_roles(h: &App, roles: &[Role]) -> Vec<(Role, String, egui::Rect)> {
    h.root()
        .children_recursive()
        .filter(|n| roles.contains(&n.accesskit_node().role()))
        .map(|n| {
            let ak = n.accesskit_node();
            (ak.role(), ak.label().unwrap_or_default(), n.rect())
        })
        .collect()
}

/// The labels of the children with `role` under the node `parent` (with `parent_role`).
pub fn child_labels(h: &App, parent: &str, parent_role: Role, role: Role) -> Vec<String> {
    get(h, parent, parent_role)
        .children_recursive()
        .filter(|n| n.accesskit_node().role() == role)
        .map(|n| n.accesskit_node().label().unwrap_or_default())
        .collect()
}

// --------------------------------------------------------------------------------------------
// Rendering

/// The repository root.
pub fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Where test screenshots go (the test's scratch dir, so a run leaves the tree clean).
pub fn screenshot_dir() -> PathBuf {
    let d = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("screenshots");
    let _ = std::fs::create_dir_all(&d);
    d
}

/// Renders the current frame (cursor hidden).
pub fn render(h: &mut App) -> image::RgbaImage {
    h.remove_cursor();
    h.run_steps(2);
    h.render().unwrap_or_else(|e| panic!("render failed: {e}"))
}

/// Renders and saves `<screenshot_dir>/<name>.png`; returns the path.
pub fn screenshot(h: &mut App, name: &str) -> PathBuf {
    let img = render(h);
    let path = screenshot_dir().join(format!("{name}.png"));
    img.save(&path)
        .unwrap_or_else(|e| panic!("save {}: {e}", path.display()));
    path
}

/// Renders `route` `w` px wide, at least `min_h` tall, grown to the app's content height so
/// nothing is clipped.
pub fn render_full(route: Route, w: f32, min_h: f32) -> image::RgbaImage {
    let mut h = harness_at(route, w, min_h);
    let need = h.state().content_height().ceil();
    if need > min_h + 0.5 {
        h.set_size(egui::vec2(w, need));
        settle(&mut h);
    }
    render(&mut h)
}

/// Saves `img` to `path` (creating parent dirs).
pub fn save(img: &image::RgbaImage, path: &Path) {
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    img.save(path)
        .unwrap_or_else(|e| panic!("save {}: {e}", path.display()));
}

/// The mockup's screen name for `route` (`Tray`, `RuleLab`, …).
pub fn mockup_name(route: Route) -> &'static str {
    route.mockup().trim_end_matches(".dc.html")
}

/// The mockup's rendered reference for `route` at `width` (1440 or 390).
pub fn mockup_png(route: Route, width: u32) -> PathBuf {
    repo().join(format!(
        "docs/design/screenshots/{}-{width}.png",
        mockup_name(route)
    ))
}

/// Loads a PNG.
pub fn load(path: &Path) -> image::RgbaImage {
    image::open(path)
        .unwrap_or_else(|e| panic!("open {}: {e}", path.display()))
        .to_rgba8()
}

/// Mean absolute channel difference between two images over their common area.
pub fn mean_diff(
    a: &image::RgbaImage,
    b: &image::RgbaImage,
    region: Option<(u32, u32, u32, u32)>,
) -> f64 {
    let (x0, y0, w, h) =
        region.unwrap_or((0, 0, a.width().min(b.width()), a.height().min(b.height())));
    let (mut sum, mut n) = (0u64, 0u64);
    for y in y0..(y0 + h).min(a.height()).min(b.height()) {
        for x in x0..(x0 + w).min(a.width()).min(b.width()) {
            let (p, q) = (a.get_pixel(x, y).0, b.get_pixel(x, y).0);
            sum += (0..3)
                .map(|i| (p[i] as i32 - q[i] as i32).unsigned_abs() as u64)
                .sum::<u64>();
            n += 3;
        }
    }
    if n == 0 { 0.0 } else { sum as f64 / n as f64 }
}
