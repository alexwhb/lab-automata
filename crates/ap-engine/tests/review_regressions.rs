//! Regressions for the independent review's engine findings (each test names its finding).

use ap_engine::RuleScript;
use std::time::{Duration, Instant};

/// Parses `text` and checks it is valid and took well under a frame budget even in the test
/// profile (the release bound would be < 50 ms).
fn parses_quickly(text: &str) {
    let t = Instant::now();
    let parsed = RuleScript::parse(text);
    let took = t.elapsed();
    assert!(parsed.is_ok(), "{parsed:?}");
    assert!(
        took < Duration::from_millis(200),
        "{} chars took {took:?}",
        text.chars().count()
    );
}

/// Review finding 8: the rule-script tokenizer looked ahead over the rest of a whitespace run
/// for every whitespace character, so a 100 k-space line took ~2 s to parse (release). It is
/// linear now.
#[test]
fn long_whitespace_runs_parse_in_linear_time() {
    let text = format!(
        ".rule{}life_like B3/S23\n.neigh moore\n.grid 64x40\n.seed rpent\n.run gens=400\n.end",
        " ".repeat(100_000)
    );
    assert!(text.chars().count() > 100_000);
    parses_quickly(&text);
    // spaces before `=` (glued to the key) and inside parentheses take the other branches
    let text = format!(
        ".rule life_like B3/S23\n.neigh moore\n.grid 64x40 boundary{}=torus\n.seed rpent @ ({}32,20)\n.run gens=400",
        " ".repeat(50_000),
        " ".repeat(50_000)
    );
    parses_quickly(&text);
}
