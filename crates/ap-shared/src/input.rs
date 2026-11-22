//! Reading patterns and rules as people (and models) write them: RLE, plaintext or a stamp id
//! for patterns; a rulestring or a named rule for rules. The app's bridge methods and the
//! server's engine tools both use these, so they accept exactly the same text, with error
//! messages that say what to write instead.

use ap_engine::pattern::{self, Format, library};
use ap_engine::{LifeRule, Neighbourhood, PatternFile, presets};

/// Where a pattern came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatternSource {
    /// A built-in stamp, by its id (`glider`).
    Stamp(&'static str),
    /// Run-length encoded text.
    Rle,
    /// Plaintext (`.cells`) rows.
    Plaintext,
}

impl PatternSource {
    /// `stamp`, `rle` or `plaintext`.
    pub fn key(self) -> &'static str {
        match self {
            PatternSource::Stamp(_) => "stamp",
            PatternSource::Rle => "rle",
            PatternSource::Plaintext => "plaintext",
        }
    }
}

/// A pattern read by [`pattern_input`].
#[derive(Debug, Clone, PartialEq)]
pub struct PatternInput {
    /// The cells and what the file said about them (a stamp gets its name).
    pub file: PatternFile,
    /// What kind of text it was.
    pub source: PatternSource,
}

/// The longest pattern text accepted (bytes).
pub const MAX_PATTERN_TEXT: usize = 1 << 20;

/// The built-in stamp with this id or name, ignoring case, spaces, `-` and `_` (`glider`,
/// `Glider gun`, `r-pentomino`, `rpent`, `LWSS`, `spaceship`, …).
pub fn stamp(text: &str) -> Option<&'static library::Stamp> {
    let key = squash(text);
    if key.is_empty() {
        return None;
    }
    let alias = match key.as_str() {
        "rpentomino" => "rpent",
        "pentadecathlon" => "penta",
        "gosper" | "gospergun" | "gosperglidergun" => "gun",
        "replicator" | "highlifereplicator" => "rep",
        "spaceship" => "lwss",
        other => other,
    };
    library::STAMPS
        .iter()
        .find(|s| s.id == alias || squash(s.name) == alias)
}

/// Reads a pattern: a stamp id or name (see [`stamp`]), RLE (`x = 3, y = 3` + `bo$2bo$3o!`,
/// header optional) or plaintext rows of `.` and `O` (`!` comments allowed).
pub fn pattern_input(text: &str) -> Result<PatternInput, String> {
    if text.len() > MAX_PATTERN_TEXT {
        return Err(format!(
            "the pattern text is {} bytes; the limit is {MAX_PATTERN_TEXT}",
            text.len()
        ));
    }
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err(format!(
            "the pattern is empty; give RLE (e.g. `x = 3, y = 3` then `bo$2bo$3o!`), plaintext \
             rows of `.` and `O`, or a stamp id ({})",
            stamp_ids()
        ));
    }
    if let Some(s) = stamp(trimmed) {
        let mut file = PatternFile::new(s.pattern());
        file.name = Some(s.name.to_owned());
        return Ok(PatternInput {
            file,
            source: PatternSource::Stamp(s.id),
        });
    }
    let (source, parsed) = match pattern::detect(trimmed) {
        Format::Rle => (PatternSource::Rle, pattern::rle::parse(trimmed)),
        Format::Cells => (PatternSource::Plaintext, pattern::cells::parse(trimmed)),
    };
    match parsed {
        Ok(file) => Ok(PatternInput { file, source }),
        Err(e) => Err(format!(
            "couldn't read the pattern as {} ({e}); give RLE (e.g. `x = 3, y = 3` then \
             `bo$2bo$3o!`), plaintext rows of `.` and `O`, or a stamp id ({})",
            match source {
                PatternSource::Plaintext => "plaintext",
                _ => "RLE",
            },
            stamp_ids()
        )),
    }
}

/// The stamp ids, comma-separated (for messages).
pub fn stamp_ids() -> String {
    library::STAMPS
        .iter()
        .map(|s| s.id)
        .collect::<Vec<_>>()
        .join(", ")
}

/// The rulestring for a named rule (`Life`, `HighLife`, `Day & Night`, `2×2`, …) or a
/// cartridge id (`life`, `high`, `dn`, …), ignoring case, spaces and punctuation.
pub fn named_rule(text: &str) -> Option<LifeRule> {
    let key = squash(text);
    if key.is_empty() {
        return None;
    }
    let key = match key.as_str() {
        "conway" | "conwayslife" | "gameoflife" => "life",
        "dayandnight" => "daynight",
        other => other,
    };
    presets::CARTRIDGES
        .iter()
        .find(|c| c.id == key)
        .map(|c| c.rule)
        .or_else(|| {
            presets::NAMED
                .iter()
                .find(|(name, _)| squash(name) == key)
                .map(|(_, rule)| *rule)
        })
}

/// Reads a rule for `neighbourhood`: a rulestring (`B3/S23`, `b36s23`, `23/3`, `B3,10/S2,3`)
/// or a name ([`named_rule`]; its counts are then used with `neighbourhood`).
pub fn rule_input(text: &str, neighbourhood: Neighbourhood) -> Result<LifeRule, String> {
    let text = text.trim();
    if let Some(named) = named_rule(text) {
        return LifeRule::parse_with(&named.code(), neighbourhood).map_err(|e| {
            format!(
                "`{text}` is {}, which doesn't fit this neighbourhood ({} cells): {e}",
                named.code(),
                neighbourhood.len()
            )
        });
    }
    LifeRule::parse_with(text, neighbourhood).map_err(|e| {
        format!(
            "couldn't read the rule `{text}`: {e}. Write it like B3/S23 (birth counts after B, \
             survival counts after S), or use a name such as Life, HighLife, Seeds, Day & Night"
        )
    })
}

/// Lowercase letters and digits only (`×` counts as `x`).
fn squash(s: &str) -> String {
    s.chars()
        .map(|c| if c == '×' { 'x' } else { c })
        .filter(char::is_ascii_alphanumeric)
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stamps_by_id_and_name() {
        assert_eq!(stamp("glider").unwrap().id, "glider");
        assert_eq!(stamp("Glider gun").unwrap().id, "gun");
        assert_eq!(stamp("R-pentomino").unwrap().id, "rpent");
        assert_eq!(stamp("LWSS").unwrap().id, "lwss");
        assert_eq!(stamp("Spaceship").unwrap().id, "lwss");
        assert!(stamp("bo$2bo$3o!").is_none());
        assert!(stamp("").is_none());
    }

    #[test]
    fn patterns_in_every_form() {
        let g = pattern_input(" glider ").unwrap();
        assert_eq!(g.source, PatternSource::Stamp("glider"));
        assert_eq!(g.file.pattern.population(), 5);
        assert_eq!(g.file.name.as_deref(), Some("Glider"));

        let r = pattern_input("x = 3, y = 3, rule = B36/S23\nbo$2bo$3o!").unwrap();
        assert_eq!(r.source, PatternSource::Rle);
        assert_eq!(r.file.pattern, g.file.pattern);
        assert_eq!(r.file.rule.as_deref(), Some("B36/S23"));

        let c = pattern_input("!Name: glider\n.O.\n..O\nOOO").unwrap();
        assert_eq!(c.source, PatternSource::Plaintext);
        assert_eq!(c.file.pattern, g.file.pattern);

        let e = pattern_input("x = 3, y = 3\nbo$2bo$3q!").unwrap_err();
        assert!(e.starts_with("couldn't read the pattern as RLE"), "{e}");
        assert!(pattern_input("   ").unwrap_err().contains("stamp id"));
    }

    #[test]
    fn rules_by_code_and_name() {
        let m = Neighbourhood::MOORE;
        assert_eq!(rule_input("b36s23", m).unwrap().code(), "B36/S23");
        assert_eq!(rule_input("HighLife", m).unwrap().code(), "B36/S23");
        assert_eq!(rule_input("Day & Night", m).unwrap().code(), "B3678/S34678");
        assert_eq!(rule_input("dn", m).unwrap().code(), "B3678/S34678");
        assert_eq!(rule_input("2×2", m).unwrap().code(), "B36/S125");
        assert_eq!(rule_input("23/3", m).unwrap().code(), "B3/S23");
        let e = rule_input("B9/S23", m).unwrap_err();
        assert!(e.contains("couldn't read the rule `B9/S23`"), "{e}");
        let vn = Neighbourhood::VON_NEUMANN;
        assert!(
            rule_input("Day & Night", vn)
                .unwrap_err()
                .contains("4 cells")
        );
        assert_eq!(rule_input("Life", vn).unwrap().code(), "B3/S23");
    }
}
