//! The Loom's stage: the tilted fabric board and print head ([`LoomScene`]), the eight rule
//! keys as buttons ([`paint_key`]) and the caption under them.
//!
//! At desktop widths everything sits where the mockup puts it (an 860 px container centred
//! at the top of the stage), zoomed down to fit smaller stages. When the stage is too narrow
//! for the key row at a usable size (phones), the board and head stay at full size and are
//! cropped at the sides like the 390 px reference, and the keys move into a 4 × 2 grid
//! centred under the head so every key stays on screen.

use super::State;
use crate::widgets::text::{css_line_height, wrapped};
use crate::widgets::{hit, interact, set_bounds};
use ap_bench::loom::{CONTAINER, KEY_SIZE};
use ap_bench::{KeyVisual, LoomGeometry, LoomScene, paint_key, spool_angle};
use ap_theme as t;
use eframe::egui::{
    Pos2, Rect, Sense, Ui, UiBuilder, Vec2, WidgetInfo, accesskit::Role, pos2, vec2,
};

/// The caption under the keys.
pub const CAPTION: &str =
    "These 8 keys are the rule. Read the outputs as binary to get its number.";
/// The key group's accessible name (the mockup's `aria-label`).
pub const KEYS_LABEL: &str = "Rule keys: press a key to make that pattern produce a live cell";

/// Below this zoom the key row gets too small and the keys go into the grid.
const MIN_ROW_ZOOM: f32 = 0.7;
/// The key cap's drop under its face (`box-shadow: 0 7px 0 …`).
const KEY_DROP: f32 = 7.0;
/// Grid gaps (CSS px).
const GRID_GAP: Vec2 = Vec2::new(18.0, 14.0);
/// The key row's top in the container (`top: 560px`).
const KEYS_TOP: f32 = 560.0;
/// Caption distance under the grid, and the room kept under the caption.
const CAPTION_GAP: f32 = 16.0;
const BOTTOM_ROOM: f32 = 16.0;

/// Where the keys and the caption go for one stage.
struct Arrangement {
    zoom: f32,
    grid: bool,
}

impl Arrangement {
    fn for_stage(stage: Vec2, caption_line: f32) -> Arrangement {
        if stage.x / CONTAINER.x >= MIN_ROW_ZOOM {
            return Arrangement {
                zoom: LoomScene::fit_zoom(stage),
                grid: false,
            };
        }
        let grid = grid_size();
        let need_h = KEYS_TOP + grid.y + CAPTION_GAP + 2.0 * caption_line + BOTTOM_ROOM;
        let zoom = (stage.y / need_h)
            .min((stage.x - 32.0).max(1.0) / grid.x)
            .clamp(0.3, 1.0);
        Arrangement { zoom, grid: true }
    }
}

/// The 4 × 2 key grid's size in CSS px (drops included).
fn grid_size() -> Vec2 {
    vec2(
        4.0 * KEY_SIZE.x + 3.0 * GRID_GAP.x,
        2.0 * (KEY_SIZE.y + KEY_DROP) + GRID_GAP.y,
    )
}

/// The neighbourhood pattern of key `i` (0 = 111 … 7 = 000).
fn pattern_of(i: usize) -> u8 {
    7 - i as u8
}

/// "Pattern 110 makes a live cell" (the mockup's key `aria-label`).
pub fn key_name(rule: u8, p: u8) -> String {
    let on = (rule >> (p & 7)) & 1 == 1;
    format!(
        "Pattern {}{}{} makes {}",
        (p >> 2) & 1,
        (p >> 1) & 1,
        p & 1,
        if on { "a live cell" } else { "an empty cell" }
    )
}

impl State {
    /// Draws the stage into `ui` (the `.graph` stage under the tab row).
    pub(super) fn stage(&mut self, ui: &mut Ui) {
        let stage = ui.max_rect();
        let body = lab_theme::font::body(12.0);
        let arrangement = Arrangement::for_stage(stage.size(), css_line_height(ui, &body));
        let zoom = arrangement.zoom;
        let rule = self.rule();
        let head = format!("PRINT HEAD · {} CELLS · WRAPS", self.weave.width());
        let rule_label = format!("Rule {rule}");
        let geo = {
            let rows: Vec<&[bool]> = self.weave.rows().collect();
            let mut scene = LoomScene::new(&rows);
            scene.width = self.weave.width();
            scene.capacity = self.weave.keep();
            scene.head_label = &head;
            scene.rule_label = &rule_label;
            scene.progress = self.weave.progress() as f32;
            scene.spool_angle = spool_angle(self.spool_secs);
            scene.zoom = zoom;
            scene.paint(ui.painter(), stage)
        };

        // A11Y-4: the fabric and the print head, in words
        let description = self.description();
        ui.interact(
            geo.board.element_bounds().intersect(stage),
            ui.scope_id().with("fabric"),
            Sense::hover(),
        )
        .widget_info(|| WidgetInfo::labeled(Role::Image, true, &description));
        let head_text = format!(
            "Print head, {} cells, wraps; {} of {} rows woven; Rule {rule}",
            self.weave.width(),
            self.weave.len(),
            self.weave.keep()
        );
        ui.interact(
            geo.head.intersect(stage),
            ui.scope_id().with("head"),
            Sense::hover(),
        )
        .widget_info(|| WidgetInfo::labeled(Role::Label, true, &head_text));

        // the keys and the caption
        let (keys, caption, caption_w) = if arrangement.grid {
            grid_layout(stage, &geo, zoom)
        } else {
            (geo.keys, geo.caption, (CONTAINER.x - 60.0) * zoom)
        };
        if let Some(p) = self.keys(ui, &keys, zoom) {
            self.toggle_key(p);
        }
        let font = lab_theme::font::body(12.0 * zoom);
        let line = css_line_height(ui, &font);
        let g = wrapped(ui, CAPTION, font, t::TEXT_MUTED, caption_w, Some(line));
        let rect = Rect::from_min_size(caption, g.size());
        ui.painter().galley(caption, g, t::TEXT_MUTED);
        ui.interact(rect, ui.scope_id().with("caption"), Sense::hover())
            .widget_info(|| WidgetInfo::labeled(Role::Label, true, CAPTION));
    }

    /// The eight keys at `keys` (unpressed faces, 111 → 000) as toggle buttons in a group;
    /// returns the pattern whose key was pressed.
    fn keys(&self, ui: &mut Ui, keys: &[Rect; 8], zoom: f32) -> Option<u8> {
        let rule = self.rule();
        let drop = vec2(0.0, KEY_DROP * zoom);
        let faces: Vec<Rect> = keys
            .iter()
            .map(|r| Rect::from_min_max(r.min, r.max + drop))
            .collect();
        let targets = hit::tile(&faces);
        let bounds = faces.iter().fold(Rect::NOTHING, |a, r| a.union(*r));
        let group = ui.new_child(
            UiBuilder::new()
                .id_salt("rule-keys")
                .max_rect(bounds)
                .accessibility_role(Role::Group)
                .accessibility_label(KEYS_LABEL),
        );
        set_bounds(&group, bounds);
        let mut pressed = None;
        for (i, rect) in keys.iter().enumerate() {
            let p = pattern_of(i);
            let mut key = KeyVisual::for_rule(rule, p);
            let name = key_name(rule, p);
            let resp = interact(
                &group,
                group.scope_id().with(("key", i)),
                targets[i],
                true,
                Role::Button,
                &name,
                Some(key.on),
            );
            key.hovered = resp.hovered();
            key.pressed = resp.is_pointer_button_down_on();
            key.focused = resp.has_focus();
            paint_key(ui.painter(), *rect, key);
            if resp.clicked() {
                pressed = Some(p);
            }
        }
        pressed
    }
}

/// The phone layout: keys in a 4 × 2 grid centred on the stage under the head, the caption
/// under them wrapped to the grid's width.
fn grid_layout(stage: Rect, geo: &LoomGeometry, zoom: f32) -> ([Rect; 8], Pos2, f32) {
    let grid = grid_size();
    let left = stage.center().x - grid.x * zoom / 2.0;
    let top = geo.keys[0].top();
    let keys = std::array::from_fn(|i| {
        let (col, row) = ((i % 4) as f32, (i / 4) as f32);
        let min = pos2(
            left + col * (KEY_SIZE.x + GRID_GAP.x) * zoom,
            top + row * (KEY_SIZE.y + KEY_DROP + GRID_GAP.y) * zoom,
        );
        Rect::from_min_size(min, KEY_SIZE * zoom)
    });
    let caption = pos2(left, top + (grid.y + CAPTION_GAP) * zoom);
    (keys, caption, grid.x * zoom)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_names_follow_the_mockup() {
        assert_eq!(key_name(30, 4), "Pattern 100 makes a live cell");
        assert_eq!(key_name(30, 7), "Pattern 111 makes an empty cell");
        assert_eq!(key_name(30, 0), "Pattern 000 makes an empty cell");
    }

    #[test]
    fn narrow_stages_use_the_grid() {
        let wide = Arrangement::for_stage(vec2(1120.0, 798.0), 15.0);
        assert!(!wide.grid);
        assert_eq!(wide.zoom, 1.0);
        let medium = Arrangement::for_stage(vec2(680.0, 798.0), 15.0);
        assert!(!medium.grid && medium.zoom < 1.0);
        let phone = Arrangement::for_stage(vec2(390.0, 767.0), 15.0);
        assert!(phone.grid);
        assert!(phone.zoom > 0.9 && phone.zoom <= 1.0, "{}", phone.zoom);
    }
}
