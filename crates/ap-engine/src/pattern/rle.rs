//! Run-length encoded patterns (`.rle`, the Golly / LifeWiki format).
//!
//! ```text
//! #N Glider
//! #C The smallest spaceship.
//! x = 3, y = 3, rule = B3/S23
//! bo$2bo$3o!
//! ```

use super::{MAX_PATTERN_CELLS, MAX_PATTERN_DIM, Pattern, PatternError, PatternFile};
use crate::rule::LifeRule;

/// RLE lines are wrapped at this many characters.
pub const LINE_WIDTH: usize = 70;

/// Whether `line` is an RLE header (`x = …`).
pub fn is_header(line: &str) -> bool {
    let line = line.trim_start();
    let mut chars = line.chars();
    matches!(chars.next(), Some('x' | 'X')) && chars.as_str().trim_start().starts_with('=')
}

/// Parses an RLE pattern.
///
/// Lenient where files in the wild are: `#N` (name), `#O` (author), `#C` / `#c` (comments) and
/// other `#` lines anywhere before the body ends; an optional `x = m, y = n, rule = …` header
/// (any spacing, unknown keys ignored); whitespace and newlines anywhere in the body, even
/// between a run count and its tag; multi-digit run counts; `b` / `.` dead, `o` alive, and the
/// multi-state letters `A`–`X` (optionally prefixed `p`–`y`) read as alive; `$` with a count
/// skips rows; everything after `!` is ignored, and a missing `!` is fine. The pattern is at
/// least as large as the header says. Errors carry the 1-based line.
pub fn parse(text: &str) -> Result<PatternFile, PatternError> {
    let mut file = PatternFile::default();
    let (mut header_w, mut header_h) = (0usize, 0usize);
    let mut seen_header = false;
    let mut body_started = false;
    let mut cells: Vec<(usize, usize)> = Vec::new();
    let (mut x, mut y) = (0usize, 0usize);
    let mut count: Option<usize> = None;
    let mut prefix: Option<char> = None;

    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    'lines: for (index, raw) in text.lines().enumerate() {
        let line_no = index + 1;
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with('#') && count.is_none() && prefix.is_none() {
            read_comment(&mut file, line);
            continue;
        }
        if !body_started && !seen_header && is_header(line) {
            seen_header = true;
            let (w, h, rule) = read_header(line, line_no)?;
            header_w = w;
            header_h = h;
            file.rule = rule;
            continue;
        }
        body_started = true;
        for c in line.chars() {
            if c.is_whitespace() {
                continue;
            }
            if let Some(d) = c.to_digit(10) {
                if prefix.is_some() {
                    return Err(PatternError::new(
                        line_no,
                        "a run count can't split a multi-state cell",
                    ));
                }
                let n = count
                    .unwrap_or(0)
                    .checked_mul(10)
                    .and_then(|n| n.checked_add(d as usize))
                    .filter(|&n| n <= MAX_PATTERN_DIM)
                    .ok_or_else(|| {
                        PatternError::new(line_no, format!("run count is over {MAX_PATTERN_DIM}"))
                    })?;
                count = Some(n);
                continue;
            }
            let run = count.take().unwrap_or(1);
            if let Some(p) = prefix.take() {
                if !c.is_ascii_uppercase() || c > 'X' {
                    return Err(PatternError::new(
                        line_no,
                        format!("`{p}` must be followed by a state letter A–X, not `{c}`"),
                    ));
                }
                push_run(&mut cells, &mut x, y, run, true, line_no)?;
                continue;
            }
            match c {
                'b' | '.' => push_run(&mut cells, &mut x, y, run, false, line_no)?,
                'o' | 'A'..='X' => push_run(&mut cells, &mut x, y, run, true, line_no)?,
                'p'..='y' => {
                    count = Some(run);
                    prefix = Some(c);
                }
                '$' => {
                    y = y
                        .checked_add(run)
                        .filter(|&y| y < MAX_PATTERN_DIM)
                        .ok_or_else(|| {
                            PatternError::new(
                                line_no,
                                format!("the pattern is taller than {MAX_PATTERN_DIM} rows"),
                            )
                        })?;
                    x = 0;
                }
                '!' => break 'lines,
                c => {
                    return Err(PatternError::new(
                        line_no,
                        format!(
                            "unexpected `{c}` in the pattern (expected b, o, $, ! or a run count)"
                        ),
                    ));
                }
            }
        }
    }
    file.pattern = Pattern::new(header_w, header_h, cells);
    Ok(file)
}

fn read_comment(file: &mut PatternFile, line: &str) {
    let mut chars = line[1..].chars();
    let tag = chars.next();
    let rest = chars.as_str().trim().to_string();
    match tag {
        Some('N') => file.name = Some(rest),
        Some('O') => file.author = Some(rest),
        Some('C' | 'c') => file.comments.push(rest),
        _ => file.comments.push(line[1..].trim().to_string()),
    }
}

fn read_header(line: &str, line_no: usize) -> Result<(usize, usize, Option<String>), PatternError> {
    let (mut w, mut h, mut rule) = (0, 0, None::<String>);
    for part in line.split(',') {
        let Some((key, value)) = part.split_once('=') else {
            // `rule = B3/S23:T40,26`: that comma belongs to the rule's Golly topology suffix.
            if let Some(r) = rule.as_mut() {
                r.push(',');
                r.push_str(part.trim());
                continue;
            }
            return Err(PatternError::new(
                line_no,
                format!("`{}` in the header isn't `key = value`", part.trim()),
            ));
        };
        let value = value.trim();
        let dim = |v: &str| -> Result<usize, PatternError> {
            v.parse::<usize>()
                .ok()
                .filter(|&n| n <= MAX_PATTERN_DIM)
                .ok_or_else(|| {
                    PatternError::new(
                        line_no,
                        format!("`{v}` isn't a size up to {MAX_PATTERN_DIM}"),
                    )
                })
        };
        match key.trim().to_ascii_lowercase().as_str() {
            "x" => w = dim(value)?,
            "y" => h = dim(value)?,
            "rule" => rule = Some(value.to_string()),
            _ => {}
        }
    }
    Ok((w, h, rule))
}

fn push_run(
    cells: &mut Vec<(usize, usize)>,
    x: &mut usize,
    y: usize,
    run: usize,
    alive: bool,
    line_no: usize,
) -> Result<(), PatternError> {
    let end = x
        .checked_add(run)
        .filter(|&e| e <= MAX_PATTERN_DIM)
        .ok_or_else(|| {
            PatternError::new(
                line_no,
                format!("the pattern is wider than {MAX_PATTERN_DIM} cells"),
            )
        })?;
    if alive {
        if cells.len() + run > MAX_PATTERN_CELLS {
            return Err(PatternError::new(
                line_no,
                format!("more than {MAX_PATTERN_CELLS} live cells"),
            ));
        }
        cells.extend((*x..end).map(|cx| (cx, y)));
    }
    *x = end;
    Ok(())
}

/// Writes `pattern` as standard RLE: an optional `#N name` line, the header
/// `x = w, y = h[, rule = code]`, then the body with runs counted, trailing dead cells dropped,
/// blank rows folded into `n$`, ending in `!`, wrapped to lines of at most 70 characters
/// (never inside a run).
pub fn write(pattern: &Pattern, rule: Option<&LifeRule>, name: Option<&str>) -> String {
    let file = PatternFile {
        pattern: pattern.clone(),
        name: name.map(str::to_string),
        rule: rule.map(LifeRule::code),
        ..PatternFile::default()
    };
    write_file(&file)
}

/// Writes a whole [`PatternFile`]: `#N`, `#O`, `#C` lines (one per comment, newlines folded to
/// spaces), the header with `rule =` when present, and the body as [`write`] does.
pub fn write_file(file: &PatternFile) -> String {
    let one_line = |s: &str| s.replace(['\r', '\n'], " ");
    let mut out = String::new();
    if let Some(name) = &file.name {
        out.push_str(&format!("#N {}\n", one_line(name)));
    }
    if let Some(author) = &file.author {
        out.push_str(&format!("#O {}\n", one_line(author)));
    }
    for c in &file.comments {
        out.push_str(&format!("#C {}\n", one_line(c)));
    }
    let p = &file.pattern;
    out.push_str(&format!("x = {}, y = {}", p.width(), p.height()));
    if let Some(rule) = &file.rule {
        out.push_str(&format!(", rule = {}", one_line(rule)));
    }
    out.push('\n');
    let mut line = String::new();
    for token in body_tokens(p) {
        if !line.is_empty() && line.len() + token.len() > LINE_WIDTH {
            out.push_str(&line);
            out.push('\n');
            line.clear();
        }
        line.push_str(&token);
    }
    out.push_str(&line);
    out.push('\n');
    out
}

fn body_tokens(p: &Pattern) -> Vec<String> {
    fn run(n: usize, tag: char) -> String {
        if n > 1 {
            format!("{n}{tag}")
        } else {
            tag.to_string()
        }
    }
    fn flush(tokens: &mut Vec<String>, alive: &mut usize) {
        if *alive > 0 {
            tokens.push(run(*alive, 'o'));
            *alive = 0;
        }
    }
    let mut tokens = Vec::new();
    // `row`: the row being written; `next_x`: the column after what is written or pending;
    // `alive`: the pending run of live cells ending at `next_x`.
    let (mut row, mut next_x, mut alive) = (0usize, 0usize, 0usize);
    for &(x, y) in p.cells() {
        if y > row {
            flush(&mut tokens, &mut alive);
            tokens.push(run(y - row, '$'));
            row = y;
            next_x = 0;
        }
        if x > next_x {
            flush(&mut tokens, &mut alive);
            tokens.push(run(x - next_x, 'b'));
        }
        alive += 1;
        next_x = x + 1;
    }
    flush(&mut tokens, &mut alive);
    tokens.push("!".to_string());
    tokens
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pattern::library;

    #[test]
    fn writes_the_glider() {
        let g = library::pattern("glider").unwrap();
        assert_eq!(
            write(&g, Some(&LifeRule::LIFE), Some("Glider")),
            "#N Glider\nx = 3, y = 3, rule = B3/S23\nbo$2bo$3o!\n"
        );
    }

    #[test]
    fn folds_blank_rows_and_keeps_margins() {
        let p = Pattern::new(6, 6, [(2, 1), (3, 1), (0, 4)]);
        let text = write(&p, None, None);
        assert_eq!(text, "x = 6, y = 6\n$2b2o3$o!\n");
        assert_eq!(parse(&text).unwrap().pattern, p);
    }

    #[test]
    fn parses_the_wild() {
        let text = "#N Gosper glider gun\n#O Bill Gosper\n#C A true period 30 glider gun.\n#C The first known gun.\nx = 36, y = 9, rule = B3/S23\n24bo$22bobo$12b2o6b2o12b2o$11bo3bo4b2o12b2o$2o8bo5bo3b2o$2o8bo3bob2o4b\nobo$10bo5bo7bo$11bo3bo$12b2o!";
        let f = parse(text).unwrap();
        assert_eq!(f.name.as_deref(), Some("Gosper glider gun"));
        assert_eq!(f.author.as_deref(), Some("Bill Gosper"));
        assert_eq!(f.comments.len(), 2);
        assert_eq!(f.rule.as_deref(), Some("B3/S23"));
        assert_eq!(f.pattern, library::pattern("gun").unwrap());
    }

    #[test]
    fn whitespace_multistate_and_missing_bang() {
        let f = parse("x=3,y=3\n b o $ 2\nb o\n$3A").unwrap();
        assert_eq!(f.pattern, library::pattern("glider").unwrap());
        let f = parse("2pA.B!").unwrap();
        assert_eq!(f.pattern.cells(), &[(0, 0), (1, 0), (3, 0)]);
        let f = parse("x = 0, y = 0, rule = B3/S23:T40,26\n!").unwrap();
        assert_eq!(f.rule.as_deref(), Some("B3/S23:T40,26"));
        assert!(f.pattern.is_empty());
    }

    #[test]
    fn errors_have_lines() {
        assert_eq!(parse("x = 3, y = 3\nbo$2bz!").unwrap_err().line, 2);
        assert!(parse("99999999999o!").is_err());
        assert!(parse("x = q, y = 3\no!").is_err());
        assert!(parse("pq!").is_err());
    }

    #[test]
    fn wraps_long_lines() {
        let cells = (0..200).filter(|x| x % 3 != 0).map(|x| (x, 0));
        let text = write(&Pattern::new(0, 0, cells), None, None);
        assert!(text.lines().all(|l| l.len() <= LINE_WIDTH), "{text}");
        assert!(text.lines().count() > 3);
    }
}
