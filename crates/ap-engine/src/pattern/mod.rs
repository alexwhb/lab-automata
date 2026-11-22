//! Patterns: a box of live cells, the built-in stamps, RLE and plaintext `.cells` files
//! (SPEC ENG-3, TRAY-5, TRAY-9, LAB-5).

pub mod cells;
pub mod library;
pub mod rle;

use std::fmt;

use thiserror::Error;

use crate::grid::Grid;

/// Parsers refuse patterns wider or taller than this many cells.
pub const MAX_PATTERN_DIM: usize = 100_000;
/// Parsers refuse patterns with more live cells than this.
pub const MAX_PATTERN_CELLS: usize = 4_000_000;

/// A `w × h` box with some live cells, independent of any grid. The cells are kept sorted
/// row-major (by `y`, then `x`) and unique, so equal patterns compare equal.
#[derive(Clone, PartialEq, Eq, Hash, Default)]
pub struct Pattern {
    w: usize,
    h: usize,
    cells: Vec<(usize, usize)>,
}

impl Pattern {
    /// A pattern of at least `w × h`; cells outside that box grow it to fit.
    pub fn new(w: usize, h: usize, cells: impl IntoIterator<Item = (usize, usize)>) -> Pattern {
        let mut cells: Vec<(usize, usize)> = cells.into_iter().collect();
        cells.sort_unstable_by_key(|&(x, y)| (y, x));
        cells.dedup();
        let w = cells.iter().map(|&(x, _)| x + 1).max().unwrap_or(0).max(w);
        let h = cells.iter().map(|&(_, y)| y + 1).max().unwrap_or(0).max(h);
        Pattern { w, h, cells }
    }

    /// The empty 0 × 0 pattern.
    pub fn empty() -> Pattern {
        Pattern::default()
    }

    /// A pattern from text rows, the mockup's stamp notation: `O` (or `*`) alive, anything else
    /// dead. The width is the longest row, the height the number of rows.
    pub fn from_rows<S: AsRef<str>>(rows: &[S]) -> Pattern {
        let w = rows
            .iter()
            .map(|r| r.as_ref().chars().count())
            .max()
            .unwrap_or(0);
        let cells = rows.iter().enumerate().flat_map(|(y, r)| {
            r.as_ref()
                .chars()
                .enumerate()
                .filter(|(_, c)| matches!(c, 'O' | '*'))
                .map(move |(x, _)| (x, y))
        });
        Pattern::new(w, rows.len(), cells)
    }

    /// The live cells of `grid` inside the box `x..x+w`, `y..y+h` (clipped to the grid),
    /// relative to the box's top-left. The pattern has the clipped box's size.
    pub fn from_grid_region(grid: &Grid, x: usize, y: usize, w: usize, h: usize) -> Pattern {
        let x1 = x.saturating_add(w).min(grid.width());
        let y1 = y.saturating_add(h).min(grid.height());
        let (pw, ph) = (x1.saturating_sub(x), y1.saturating_sub(y));
        let cells = (y..y1)
            .flat_map(|cy| (x..x1).map(move |cx| (cx, cy)))
            .filter(|&(cx, cy)| grid.is_alive(cx, cy))
            .map(|(cx, cy)| (cx - x, cy - y));
        Pattern::new(pw, ph, cells)
    }

    /// The live cells of `grid` trimmed to their bounding box (empty 0 × 0 when nothing lives).
    pub fn from_grid(grid: &Grid) -> Pattern {
        match grid.bbox() {
            Some(b) => Pattern::from_grid_region(grid, b.x, b.y, b.w, b.h),
            None => Pattern::empty(),
        }
    }

    /// Width in cells.
    pub fn width(&self) -> usize {
        self.w
    }

    /// Height in cells.
    pub fn height(&self) -> usize {
        self.h
    }

    /// The live cells `(x, y)`, row-major.
    pub fn cells(&self) -> &[(usize, usize)] {
        &self.cells
    }

    /// Number of live cells.
    pub fn population(&self) -> usize {
        self.cells.len()
    }

    /// Whether no cell is alive.
    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }

    /// Whether `(x, y)` is alive.
    pub fn contains(&self, x: usize, y: usize) -> bool {
        self.cells
            .binary_search_by_key(&(y, x), |&(cx, cy)| (cy, cx))
            .is_ok()
    }

    /// The same cells shifted so the bounding box starts at (0, 0), sized to it.
    pub fn trimmed(&self) -> Pattern {
        let x0 = self.cells.iter().map(|&(x, _)| x).min().unwrap_or(0);
        let y0 = self.cells.first().map_or(0, |&(_, y)| y);
        Pattern::new(0, 0, self.cells.iter().map(|&(x, y)| (x - x0, y - y0)))
    }

    /// Text rows, `O` alive and `.` dead (the stamp notation; previews use it).
    pub fn to_rows(&self) -> Vec<String> {
        let mut rows = vec![vec!['.'; self.w]; self.h];
        for &(x, y) in &self.cells {
            rows[y][x] = 'O';
        }
        rows.into_iter().map(|r| r.into_iter().collect()).collect()
    }
}

impl fmt::Debug for Pattern {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Pattern({}×{}, {:?})", self.w, self.h, self.cells)
    }
}

/// A pattern read from a file or the clipboard, with what the file said about it.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PatternFile {
    /// The cells.
    pub pattern: Pattern,
    /// `#N` (RLE) or `!Name:` (plaintext).
    pub name: Option<String>,
    /// `#O` (RLE).
    pub author: Option<String>,
    /// `#C` / `#c` lines (RLE) or other `!` lines (plaintext), without the marker.
    pub comments: Vec<String>,
    /// The RLE header's `rule =` value, verbatim (parse it with
    /// [`LifeRule::parse`](crate::LifeRule::parse)).
    pub rule: Option<String>,
}

impl PatternFile {
    /// A file holding just `pattern`.
    pub fn new(pattern: Pattern) -> PatternFile {
        PatternFile {
            pattern,
            ..PatternFile::default()
        }
    }

    /// The `rule =` value parsed as a Moore rule, if there was one.
    pub fn life_rule(&self) -> Option<Result<crate::LifeRule, crate::RuleParseError>> {
        self.rule.as_deref().map(crate::LifeRule::parse)
    }
}

/// An error from a pattern parser, with the 1-based line it was found on.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("line {line}: {message}")]
pub struct PatternError {
    /// 1-based line number.
    pub line: usize,
    /// What went wrong, for people.
    pub message: String,
}

impl PatternError {
    pub(crate) fn new(line: usize, message: impl Into<String>) -> PatternError {
        PatternError {
            line,
            message: message.into(),
        }
    }
}

/// The text's format, as guessed by [`detect`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    /// Run-length encoded (`.rle`).
    Rle,
    /// Plaintext (`.cells`).
    Cells,
}

/// Guesses whether `text` is RLE or plaintext: a leading `!` comment or rows made only of `.`,
/// `O` and `*` mean plaintext; a `x = …` header or anything else means RLE.
pub fn detect(text: &str) -> Format {
    let mut lines = text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .peekable();
    if lines.peek().is_some_and(|l| l.starts_with('!')) {
        return Format::Cells;
    }
    let body: Vec<&str> = lines.filter(|l| !l.starts_with('#')).collect();
    if body.iter().any(|l| rle::is_header(l)) {
        return Format::Rle;
    }
    if !body.is_empty()
        && body
            .iter()
            .all(|l| l.chars().all(|c| matches!(c, '.' | 'O' | '*')))
    {
        return Format::Cells;
    }
    Format::Rle
}

/// Parses RLE or plaintext, whichever the text looks like ([`detect`]) — for paste and drop
/// (TRAY-9).
pub fn parse_any(text: &str) -> Result<PatternFile, PatternError> {
    match detect(text) {
        Format::Rle => rle::parse(text),
        Format::Cells => cells::parse(text),
    }
}

/// Parses by file name: `.cells` as plaintext, `.rle` as RLE, anything else by [`detect`].
pub fn parse_file(file_name: &str, text: &str) -> Result<PatternFile, PatternError> {
    let lower = file_name.to_ascii_lowercase();
    if lower.ends_with(".cells") {
        cells::parse(text)
    } else if lower.ends_with(".rle") {
        rle::parse(text)
    } else {
        parse_any(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_rows_uses_the_longest_row() {
        let gun = library::pattern("gun").unwrap();
        assert_eq!((gun.width(), gun.height(), gun.population()), (36, 9, 36));
        let p = Pattern::from_rows(&["O", "..O"]);
        assert_eq!((p.width(), p.height()), (3, 2));
        assert!(p.contains(2, 1) && !p.contains(1, 1));
        assert_eq!(p.to_rows(), ["O..", "..O"]);
    }

    #[test]
    fn grid_regions_and_trimming() {
        let mut g = Grid::new(8, 8);
        g.set(3, 4, true);
        g.set(5, 6, true);
        let p = Pattern::from_grid(&g);
        assert_eq!((p.width(), p.height()), (3, 3));
        assert_eq!(p.cells(), &[(0, 0), (2, 2)]);
        let r = Pattern::from_grid_region(&g, 2, 2, 100, 100);
        assert_eq!((r.width(), r.height()), (6, 6));
        assert_eq!(r.trimmed(), p);
        assert!(Pattern::from_grid(&Grid::new(3, 3)).is_empty());
        assert_eq!(
            Pattern::from_grid_region(&g, 50, 50, 3, 3),
            Pattern::empty()
        );
    }

    #[test]
    fn detects_formats() {
        assert_eq!(detect("!Name: x\n.O\nOO"), Format::Cells);
        assert_eq!(detect(".O.\n..O\nOOO\n"), Format::Cells);
        assert_eq!(detect("#N g\nx = 3, y = 3\nbo$2bo$3o!"), Format::Rle);
        assert_eq!(detect("bo$2bo$3o!"), Format::Rle);
        assert_eq!(
            parse_any(".O.\n..O\nOOO").unwrap().pattern,
            library::pattern("glider").unwrap()
        );
        assert_eq!(
            parse_file("g.rle", "bo$2bo$3o!").unwrap().pattern,
            library::pattern("glider").unwrap()
        );
    }
}
