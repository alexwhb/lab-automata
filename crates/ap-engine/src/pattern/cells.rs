//! Plaintext patterns (`.cells`, the LifeWiki format).
//!
//! ```text
//! !Name: Glider
//! !The smallest spaceship.
//! .O.
//! ..O
//! OOO
//! ```

use super::{MAX_PATTERN_CELLS, MAX_PATTERN_DIM, Pattern, PatternError, PatternFile};

/// Parses a plaintext pattern: lines starting with `!` are comments (`!Name: …` names it);
/// every other line is a row, `.` dead and `O` (or `*`) alive; trailing spaces are dropped and
/// blank lines inside the pattern are empty rows (trailing ones are dropped). Any other
/// character is an error with its line.
pub fn parse(text: &str) -> Result<PatternFile, PatternError> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut file = PatternFile::default();
    let mut cells = Vec::new();
    let (mut rows, mut width) = (0usize, 0usize);
    let mut last_row_with_content = 0usize;
    for (index, raw) in text.lines().enumerate() {
        let line_no = index + 1;
        let line = raw.trim_end();
        if let Some(comment) = line.strip_prefix('!') {
            let comment = comment.trim();
            match comment.strip_prefix("Name:") {
                Some(name) if file.name.is_none() => file.name = Some(name.trim().to_string()),
                _ => file.comments.push(comment.to_string()),
            }
            continue;
        }
        if line.trim().is_empty() && rows == 0 {
            continue;
        }
        let y = rows;
        rows += 1;
        if rows > MAX_PATTERN_DIM {
            return Err(PatternError::new(
                line_no,
                format!("the pattern is taller than {MAX_PATTERN_DIM} rows"),
            ));
        }
        for (x, c) in line.chars().enumerate() {
            if x >= MAX_PATTERN_DIM {
                return Err(PatternError::new(
                    line_no,
                    format!("the pattern is wider than {MAX_PATTERN_DIM} cells"),
                ));
            }
            match c {
                '.' | ' ' => {}
                'O' | '*' => {
                    if cells.len() >= MAX_PATTERN_CELLS {
                        return Err(PatternError::new(
                            line_no,
                            format!("more than {MAX_PATTERN_CELLS} live cells"),
                        ));
                    }
                    cells.push((x, y));
                }
                c => {
                    return Err(PatternError::new(
                        line_no,
                        format!(
                            "unexpected `{c}` (plaintext rows use `.` for dead and `O` for alive)"
                        ),
                    ));
                }
            }
            width = width.max(x + 1);
        }
        if !line.trim().is_empty() {
            last_row_with_content = rows;
        }
    }
    file.pattern = Pattern::new(width, last_row_with_content, cells);
    Ok(file)
}

/// Writes plaintext: an optional `!Name:` line, then one row per line (`.` and `O`), trailing
/// dead cells dropped.
pub fn write(pattern: &Pattern, name: Option<&str>) -> String {
    let mut out = String::new();
    if let Some(name) = name {
        out.push_str(&format!("!Name: {}\n", name.replace(['\r', '\n'], " ")));
    }
    for row in pattern.to_rows() {
        out.push_str(row.trim_end_matches('.'));
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pattern::library;

    #[test]
    fn parses_named_plaintext() {
        let f = parse("!Name: Glider\n!A spaceship.\n.O.\n..O\nOOO\n").unwrap();
        assert_eq!(f.name.as_deref(), Some("Glider"));
        assert_eq!(f.comments, ["A spaceship."]);
        assert_eq!(f.pattern, library::pattern("glider").unwrap());
    }

    #[test]
    fn blank_rows_inside_count_trailing_do_not() {
        let f = parse("O\n\nO\n\n\n").unwrap();
        assert_eq!((f.pattern.width(), f.pattern.height()), (1, 3));
        assert!(parse("O.x").is_err());
    }

    #[test]
    fn round_trips() {
        for s in library::STAMPS {
            let p = s.pattern();
            let back = parse(&write(&p, Some(s.name))).unwrap();
            assert_eq!(back.pattern.trimmed(), p.trimmed(), "{}", s.id);
            assert_eq!(back.name.as_deref(), Some(s.name));
        }
    }
}
