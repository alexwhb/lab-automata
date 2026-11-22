//! Toasts: short confirmations ("Copied the tray as RLE", "Breadbox isn't installed on this
//! Mac") shown bottom-centre for a few seconds. Time is egui's input time, so tests stepping
//! the harness see them expire deterministically.
//!
//! Not in the mockup; drawn in its vocabulary: `#111111` card, `#2E2E2E` border, radius 8,
//! `padding: 10px 14px`, Inter 13 px with a 7 px accent dot. AccessKit: a `Status` node (a
//! polite live region) named by the text.

use ap_theme as t;
use eframe::egui::{
    self, Align2, CornerRadius, Id, Rect, Sense, Stroke, StrokeKind, WidgetInfo, accesskit::Role,
    pos2, vec2,
};

/// How long a toast stays (seconds of egui time).
pub const TOAST_SECS: f64 = 4.0;

/// One toast.
#[derive(Debug, Clone, PartialEq)]
pub struct Toast {
    /// The message.
    pub text: String,
    /// When it was pushed (egui time), `None` until first shown.
    pub since: Option<f64>,
}

/// The toast queue (newest last).
#[derive(Debug, Clone, Default)]
pub struct Toasts {
    items: Vec<Toast>,
}

impl Toasts {
    /// Queues a toast. Pushing the text already showing restarts its timer instead of adding
    /// a duplicate.
    pub fn push(&mut self, text: impl Into<String>) {
        let text = text.into();
        self.items.retain(|t| t.text != text);
        self.items.push(Toast { text, since: None });
        if self.items.len() > 3 {
            self.items.remove(0);
        }
    }

    /// The toasts currently queued or showing.
    pub fn items(&self) -> &[Toast] {
        &self.items
    }

    /// Removes every toast.
    pub fn clear(&mut self) {
        self.items.clear();
    }

    /// Draws the toasts over everything, bottom-centre, and expires old ones.
    pub fn show(&mut self, ctx: &egui::Context) {
        let now = ctx.input(|i| i.time);
        for t in &mut self.items {
            t.since.get_or_insert(now);
        }
        self.items
            .retain(|t| now - t.since.unwrap_or(now) < TOAST_SECS);
        if self.items.is_empty() {
            return;
        }
        let screen = ctx.content_rect();
        let mut bottom = screen.bottom() - 16.0;
        let mut dismissed = None;
        for (i, toast) in self.items.iter().enumerate().rev() {
            let id = Id::unique("ap-toast").with(i);
            let area = egui::Area::new(id)
                .order(egui::Order::Foreground)
                .fixed_pos(pos2(screen.center().x, bottom))
                .pivot(Align2::CENTER_BOTTOM)
                .interactable(true);
            let shown = area.show(ctx, |ui| {
                let font = lab_theme::font::body(13.0);
                let max_w = (screen.width() - 32.0).min(420.0) - 28.0 - 15.0;
                let g = super::text::wrapped(ui, &toast.text, font, t::TEXT, max_w, None);
                let size = vec2(g.size().x + 28.0 + 15.0, g.size().y + 20.0);
                let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
                let painter = ui.painter();
                painter.rect(
                    rect,
                    CornerRadius::same(t::radius::WELL),
                    t::CARD,
                    Stroke::new(1.0, t::BORDER_STRONG),
                    StrokeKind::Inside,
                );
                let cy =
                    rect.top() + 10.0 + g.rows.first().map_or(8.0, |r| r.rect().height() / 2.0);
                painter.circle_filled(pos2(rect.left() + 14.0 + 3.5, cy), 3.5, t::ACCENT);
                painter.galley(rect.min + vec2(14.0 + 15.0, 10.0), g, t::TEXT);
                let text = toast.text.clone();
                resp.widget_info(|| WidgetInfo::labeled(Role::Status, true, &text));
                if resp.clicked() {
                    dismissed = Some(i);
                }
                rect
            });
            let r: Rect = shown.inner;
            bottom -= r.height() + 8.0;
        }
        if let Some(i) = dismissed {
            self.items.remove(i);
        }
        ctx.request_repaint_after(std::time::Duration::from_millis(250));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_dedupes_and_caps() {
        let mut t = Toasts::default();
        t.push("a");
        t.push("b");
        t.push("a");
        assert_eq!(
            t.items()
                .iter()
                .map(|t| t.text.as_str())
                .collect::<Vec<_>>(),
            ["b", "a"]
        );
        for s in ["c", "d", "e"] {
            t.push(s);
        }
        assert_eq!(t.items().len(), 3);
        assert_eq!(t.items()[2].text, "e");
    }
}
