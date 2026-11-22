//! Accessible descriptions of the bench canvases (SPEC A11Y-4). The screens put them on the
//! canvas response (`response.widget_info(|| WidgetInfo::labeled(WidgetType::Other, true,
//! text))`) and update them every frame; every number comes from the caller (the engine).
//!
//! The default wording follows the spec; [`fill`] lets a screen use its own template.

/// "Cell tray, 40 by 26, generation 30, 90 alive".
pub fn tray(cols: usize, rows: usize, generation: u64, alive: usize) -> String {
    format!("Cell tray, {cols} by {rows}, generation {generation}, {alive} alive")
}

/// "Woven pattern, rule 30, 121 cells wide, 47 rows woven".
pub fn loom(rule: u8, width: usize, rows: u64) -> String {
    format!("Woven pattern, rule {rule}, {width} cells wide, {rows} rows woven")
}

/// "Jar running rule B3/S23, 122 cells alive" (the mockup's `aria-label`).
pub fn jar(code: &str, alive: usize) -> String {
    format!("Jar running rule {code}, {alive} cells alive")
}

/// "Top-down view of the grid, 64 by 40, generation 120, 148 alive".
pub fn top_down(cols: usize, rows: usize, generation: usize, alive: usize) -> String {
    format!("Top-down view of the grid, {cols} by {rows}, generation {generation}, {alive} alive")
}

/// Fills `{name}` placeholders in `template` from `values`; unknown placeholders stay as they
/// are. `fill("Tray {w} by {h}", &[("w", "40".into()), ("h", "26".into())])`.
pub fn fill(template: &str, values: &[(&str, String)]) -> String {
    let mut out = String::with_capacity(template.len() + 16);
    let mut rest = template;
    while let Some(start) = rest.find('{') {
        out.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        match after.find('}') {
            Some(end) => {
                let key = &after[..end];
                match values.iter().find(|(k, _)| *k == key) {
                    Some((_, v)) => out.push_str(v),
                    None => {
                        out.push('{');
                        out.push_str(key);
                        out.push('}');
                    }
                }
                rest = &after[end + 1..];
            }
            None => {
                out.push_str(&rest[start..]);
                rest = "";
            }
        }
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spec_wording() {
        assert_eq!(tray(40, 26, 30, 90), "Cell tray, 40 by 26, generation 30, 90 alive");
        assert_eq!(jar("B3/S23", 122), "Jar running rule B3/S23, 122 cells alive");
    }

    #[test]
    fn fill_templates() {
        let v = [("w", "40".to_string()), ("h", "26".to_string())];
        assert_eq!(fill("Tray {w} by {h}, {x}", &v), "Tray 40 by 26, {x}");
        assert_eq!(fill("open {w", &v), "open {w");
    }
}
